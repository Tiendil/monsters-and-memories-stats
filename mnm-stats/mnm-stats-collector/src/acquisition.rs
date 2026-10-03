use crate::{Result, liveview, parser};
use chrono::{DateTime, Utc};
use mnm_stats_model::Snapshot;
use reqwest::{
    Url,
    blocking::Client,
    cookie::{CookieStore, Jar},
    header::{HeaderValue, USER_AGENT},
};
use scraper::Html;
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    net::TcpStream,
    sync::Arc,
    time::{Duration, Instant},
};
use tungstenite::{Message, client::IntoClientRequest, stream::MaybeTlsStream};

fn user_agent(revision: Option<&str>, branch: Option<&str>) -> Result<HeaderValue> {
    let version = revision
        .filter(|value| !value.is_empty())
        .map(|value| value.chars().take(12).collect::<String>())
        .unwrap_or_else(|| "dev".into());
    let branch = branch
        .filter(|value| !value.is_empty())
        .map(|value| {
            let escaped = value
                .replace('\\', "\\\\")
                .replace('(', "\\(")
                .replace(')', "\\)");
            format!("branch={escaped}; ")
        })
        .unwrap_or_default();
    Ok(format!(
        "mnm-stats-collector/{version} ({branch}+https://github.com/Tiendil/monsters-and-memories-stats)"
    )
    .parse()?)
}

struct Session {
    topic: String,
    rendered: Option<Value>,
    initial_charts: BTreeMap<String, Value>,
}

impl Session {
    fn new(topic: String) -> Self {
        Self {
            topic,
            rendered: None,
            initial_charts: BTreeMap::new(),
        }
    }

    fn accept(&mut self, frame: &Value, now: DateTime<Utc>) -> Result<Option<Snapshot>> {
        let fields = frame
            .as_array()
            .filter(|f| f.len() == 5)
            .ok_or("malformed Phoenix message")?;
        if fields[2] != self.topic || fields[0] != "1" {
            return Err("unexpected LiveView channel or join reference".into());
        }
        match fields[3].as_str() {
            Some("phx_reply") if self.rendered.is_none() => {
                if fields[1] != "1" || fields[4]["status"] != "ok" {
                    return Err("LiveView join rejected".into());
                }
                let rendered = fields[4]["response"]
                    .get("rendered")
                    .ok_or("join lacks rendered metrics")?
                    .clone();
                self.initial_charts = parser::readiness_charts(&liveview::render(&rendered)?)?;
                self.rendered = Some(rendered);
                Ok(None)
            }
            Some("diff") => {
                let current = self
                    .rendered
                    .as_mut()
                    .ok_or("metrics diff arrived before join")?;
                liveview::merge(current, &fields[4]);
                let html = liveview::render(current)?;
                let charts = parser::readiness_charts(&html)?;
                if !charts.keys().eq(self.initial_charts.keys()) {
                    return Err(
                        "server membership changed during collection; retry a fresh observation"
                            .into(),
                    );
                }
                // Each captured asynchronous result replaces the full server
                // batch, including activity fields and chart payloads. Require
                // that batch, and a replacement of every initial chart payload.
                // Counts may stay zero; neither chart length nor elapsed time
                // establishes completion. Indistinguishable payloads time out.
                let batch = liveview::updated_batches(current, &fields[4])?
                    .into_iter()
                    .any(|html| {
                        charts
                            .keys()
                            .all(|id| html.contains(&format!("id=\"server-stats-{id}\"")))
                    });
                let complete = charts
                    .iter()
                    .all(|(id, chart)| self.initial_charts.get(id) != Some(chart));
                if batch && complete {
                    Ok(Some(parser::parse_snapshot(&html, now)?))
                } else {
                    Ok(None)
                }
            }
            Some("phx_error" | "phx_close") => {
                Err("LiveView channel closed before metrics completed".into())
            }
            _ => Err("unexpected LiveView event".into()),
        }
    }
}

/// Replay saved protocol responses through the same readiness and validation
/// path as the network client. The fixture topic is deliberately fixed.
pub fn replay(frames: &[Value], observed_at: DateTime<Utc>) -> Result<Snapshot> {
    let mut session = Session::new("lv:fixture-liveview".into());
    for frame in frames {
        if let Some(snapshot) = session.accept(frame, observed_at)? {
            return Ok(snapshot);
        }
    }
    Err("incomplete fixture: asynchronous metrics have not completed for every server".into())
}

/// One HTTP initialization and one WebSocket session, with finite waits and no
/// retries or fallback source. The returned timestamp is taken after readiness.
pub fn collect(source: &str, timeout: Duration) -> Result<Snapshot> {
    let user_agent = user_agent(
        option_env!("MNM_STATS_BUILD_REVISION"),
        option_env!("MNM_STATS_BUILD_BRANCH"),
    )?;
    let url = Url::parse(source)?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err("source must use HTTP or HTTPS".into());
    }
    let cookies = Arc::new(Jar::default());
    let client = Client::builder()
        .user_agent(user_agent.clone())
        .cookie_provider(cookies.clone())
        .timeout(timeout)
        .redirect(reqwest::redirect::Policy::none())
        .build()?;
    let response = client
        .get(url.clone())
        .send()
        .map_err(|e| format!("HTTP initialization failed: {}", e.without_url()))?;
    if response.status() != reqwest::StatusCode::OK {
        return Err(format!("HTTP initialization returned {}", response.status()).into());
    }
    let remote = response
        .remote_addr()
        .ok_or("HTTP response has no peer address")?;
    let html = response
        .text()
        .map_err(|e| format!("HTTP response failed: {}", e.without_url()))?;
    let document = Html::parse_document(&html);
    let root = document.root_element();
    let csrf = parser::one(root, "meta[name='csrf-token']")?
        .value()
        .attr("content")
        .filter(|s| !s.is_empty())
        .ok_or("missing CSRF token")?;
    let main = parser::one(root, "[data-phx-main]")?;
    let id = main
        .value()
        .id()
        .filter(|s| !s.is_empty())
        .ok_or("missing LiveView identity")?;
    let signed_session = main
        .value()
        .attr("data-phx-session")
        .filter(|s| !s.is_empty())
        .ok_or("missing LiveView session")?;
    let static_token = main.value().attr("data-phx-static");
    let mut websocket_url = url.join("/live/websocket")?;
    let cookie = cookies
        .cookies(&websocket_url)
        .ok_or("missing anonymous session cookie")?;
    websocket_url
        .set_scheme(if url.scheme() == "https" { "wss" } else { "ws" })
        .map_err(|_| "invalid WebSocket URL")?;
    websocket_url
        .query_pairs_mut()
        .append_pair("_csrf_token", csrf)
        .append_pair("_mounts", "0")
        .append_pair("vsn", "2.0.0");
    let mut request = websocket_url.as_str().into_client_request()?;
    request.headers_mut().insert(USER_AGENT, user_agent);
    request.headers_mut().insert("Cookie", cookie);
    request
        .headers_mut()
        .insert("Origin", url.origin().ascii_serialization().parse()?);
    // The socket is on the same origin as HTTP. Reuse its resolved peer address
    // so connection and TLS/upgrade waits can use explicit socket timeouts.
    let stream = TcpStream::connect_timeout(&remote, timeout)?;
    stream.set_read_timeout(Some(timeout))?;
    stream.set_write_timeout(Some(timeout))?;
    let (mut socket, _) = tungstenite::client_tls_with_config(request, stream, None, None)
        .map_err(|_| "WebSocket TLS/upgrade handshake failed")?;
    let topic = format!("lv:{id}");
    let result = (|| -> Result<Snapshot> {
        socket.send(Message::Text(
            json!(["1", "1", topic, "phx_join", {
                "url":source, "params":{"_csrf_token":csrf,"_mounts":0,"_mount_attempts":0},
                "session":signed_session, "static":static_token
            }])
            .to_string()
            .into(),
        ))?;
        let mut session = Session::new(topic);
        let deadline = Instant::now() + timeout;
        loop {
            let remaining = deadline
                .checked_duration_since(Instant::now())
                .ok_or("timed out waiting for complete server metrics")?;
            match socket.get_ref() {
                MaybeTlsStream::Plain(stream) => stream.set_read_timeout(Some(remaining))?,
                MaybeTlsStream::Rustls(stream) => stream.sock.set_read_timeout(Some(remaining))?,
                _ => return Err("unsupported WebSocket transport".into()),
            }
            match socket
                .read()
                .map_err(|e| format!("WebSocket metrics read failed or timed out: {e}"))?
            {
                Message::Text(text) => {
                    let frame: Value = serde_json::from_str(&text)?;
                    if let Some(snapshot) = session.accept(&frame, Utc::now())? {
                        return Ok(snapshot);
                    }
                }
                Message::Ping(_) => socket.flush()?,
                Message::Pong(_) => (),
                Message::Close(_) => {
                    return Err("WebSocket disconnected before all server metrics completed".into());
                }
                _ => return Err("unexpected binary WebSocket message".into()),
            }
        }
    })();
    let _ = socket.close(None);
    result
}

#[cfg(test)]
mod tests {
    use super::user_agent;

    #[test]
    fn user_agent_identifies_revision_branch_and_project() {
        assert_eq!(
            user_agent(
                Some("abc123def4567890123456789012345678901234"),
                Some("main")
            )
            .unwrap(),
            "mnm-stats-collector/abc123def456 (branch=main; +https://github.com/Tiendil/monsters-and-memories-stats)"
        );
    }

    #[test]
    fn user_agent_marks_builds_without_metadata_as_dev() {
        for metadata in [(None, None), (Some(""), Some(""))] {
            assert_eq!(
                user_agent(metadata.0, metadata.1).unwrap(),
                "mnm-stats-collector/dev (+https://github.com/Tiendil/monsters-and-memories-stats)"
            );
        }
    }

    #[test]
    fn user_agent_escapes_comment_delimiters_in_branch_names() {
        assert_eq!(
            user_agent(Some("abc123def456"), Some("feature/(trial)")).unwrap(),
            "mnm-stats-collector/abc123def456 (branch=feature/\\(trial\\); +https://github.com/Tiendil/monsters-and-memories-stats)"
        );
    }
}
