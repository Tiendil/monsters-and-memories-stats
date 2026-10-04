mod common;
use common::*;
use mnm_stats_collector::acquisition::{collect, replay};
use serde_json::{Value, json};
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    thread,
    time::{Duration, Instant},
};
use tungstenite::Message;

const INITIALIZATION: &str = r#"<html><head><meta name="csrf-token" content="fixture-csrf"></head><body><div id="fixture-liveview" data-phx-main data-phx-session="fixture-session"></div></body></html>"#;

fn rows(frame: &mut Value, join: bool) -> &mut Vec<Value> {
    let root = if join {
        &mut frame[4]["response"]["rendered"]
    } else {
        &mut frame[4]
    };
    root["0"]["1"]["0"]["1"]["0"]["d"].as_array_mut().unwrap()
}

#[test]
fn replay_waits_for_every_server_and_retains_only_current_state() {
    let frames = frames();
    for length in 0..frames.len() {
        assert!(
            replay(&frames[..length], time()).is_err(),
            "accepted incomplete prefix {length}"
        );
    }
    let snapshot = replay(&frames, time()).unwrap();
    assert_eq!(snapshot.active_subscriptions, 18_910);
    assert_eq!(snapshot.servers.len(), 6);
    assert_eq!(snapshot.servers.iter().map(|s| s.online).sum::<u64>(), 4720);
    assert_eq!(snapshot.observed_at, time());
    assert_eq!(
        snapshot
            .servers
            .iter()
            .find(|s| s.id == "kravvin")
            .unwrap()
            .daily_active,
        1172
    );
    assert!(!snapshot.to_jsonl_record().unwrap().contains("chart"));
}

#[test]
fn completed_zone_membership_is_discovered_per_server() {
    let mut added = NUNAVOTH_ZONES.to_vec();
    added.push(("new-zone", "A newly discovered zone", "17"));
    added.push(("another-zone", "Another new zone", "2"));
    let mut reordered = added.clone();
    reordered.reverse();
    for (zones, total) in [
        (NUNAVOTH_ZONES.to_vec(), "71"),
        (added, "90"),
        (reordered, "90"),
        (vec![("zero-zone", "Published zero", "0")], "0"),
        (vec![], "0"),
    ] {
        let snapshot = replay(&with_nunavoth_zones(&zones, total), time()).unwrap();
        let server = snapshot
            .servers
            .iter()
            .find(|s| s.id == "nunavoth")
            .unwrap();
        let mut expected = zones.clone();
        expected.sort_by_key(|zone| zone.0);
        assert_eq!(
            server
                .starting_zones
                .iter()
                .map(|z| (z.id.as_str(), z.name.as_str(), z.online))
                .collect::<Vec<_>>(),
            expected
                .iter()
                .map(|(id, name, count)| (*id, *name, count.parse::<u64>().unwrap()))
                .collect::<Vec<_>>()
        );
        assert!(server.starting_zones.iter().all(|z| z.id != "ailvorith"));
        assert!(
            snapshot
                .servers
                .iter()
                .filter(|s| s.id != "nunavoth")
                .all(|s| s.starting_zones.len() == 5)
        );
        assert_eq!(
            (server.daily_active, server.monthly_active, server.online),
            (745, 797, 271)
        );
    }
}

#[test]
fn invalid_dynamic_zones_explain_the_problem() {
    for (zones, total, diagnostic) in [
        (
            vec![("same", "First", "0"), ("same", "Second", "0")],
            "0",
            "duplicate zone identity \"same\"",
        ),
        (
            vec![("", "Missing identity", "0")],
            "0",
            "missing or invalid zone identity",
        ),
        (vec![("zone", "", "0")], "0", "zone name must not be empty"),
        (
            vec![("zone", "Zone", "-1")],
            "0",
            "expected an unsigned decimal integer",
        ),
        (
            NUNAVOTH_ZONES.to_vec(),
            "72",
            "published total 72 does not equal zone sum 71",
        ),
        (vec![], "1", "published total 1 does not equal zone sum 0"),
    ] {
        let error = replay(&with_nunavoth_zones(&zones, total), time())
            .unwrap_err()
            .to_string();
        for expected in [
            "server \"nunavoth\"",
            "#starting-zone-list-nunavoth",
            diagnostic,
        ] {
            assert!(error.contains(expected), "missing {expected:?}: {error}");
        }
        if total == "72" {
            for (id, _, _) in NUNAVOTH_ZONES {
                assert!(error.contains(id), "missing observed zone {id}: {error}");
            }
        }
    }
}

#[test]
fn temporary_zone_elements_wait_for_completed_metrics() {
    let mut frames = frames();
    let last = frames.len() - 1;
    for (index, frame) in frames[..last].iter_mut().enumerate() {
        let server = rows(frame, index == 0)
            .iter_mut()
            .find(|row| row[0] == "vespyra")
            .unwrap();
        // Synthetic loading state, not a captured claim about the source UI.
        server[14] = json!("<p>Loading starting zones</p>");
    }
    for length in 1..frames.len() {
        let error = replay(&frames[..length], time()).unwrap_err().to_string();
        assert!(error.contains("incomplete fixture"), "{error}");
    }
    let snapshot = replay(&frames, time()).unwrap();
    let server = snapshot.servers.iter().find(|s| s.id == "vespyra").unwrap();
    assert_eq!(
        (server.daily_active, server.monthly_active, server.online),
        (2003, 2106, 731)
    );
    assert_eq!(
        server
            .starting_zones
            .iter()
            .map(|z| (z.id.as_str(), z.online))
            .collect::<Vec<_>>(),
        [
            ("ailvorith", 53),
            ("evergrove", 73),
            ("nightharbore", 111),
            ("nightharborw", 177),
            ("underdocks", 123)
        ]
    );
}

#[test]
fn invalid_completed_zones_fail_with_element_context() {
    for (element, identity) in [
        ("<p>Starting zones unavailable</p>", "id None"),
        (
            "<p id=\"zone-status\">Starting zones unavailable</p>",
            "id Some(\"zone-status\")",
        ),
        (
            "<p id=\"starting-zone-vespyra-unknown\">Starting zones unavailable</p>",
            "id Some(\"starting-zone-vespyra-unknown\")",
        ),
    ] {
        let mut frames = frames();
        let valid = frames.last().unwrap().clone();
        let server = rows(frames.last_mut().unwrap(), false)
            .iter_mut()
            .find(|row| row[0] == "vespyra")
            .unwrap();
        server[14] = json!(element);
        // Once readiness is established, invalid fields fail immediately.
        frames.push(valid);
        let error = replay(&frames, time()).unwrap_err().to_string();
        for expected in [
            "server \"vespyra\"",
            "starting-zone-list-vespyra",
            "<p>",
            identity,
            "Starting zones unavailable",
        ] {
            assert!(error.contains(expected), "missing {expected:?}: {error}");
        }
    }
}

#[test]
fn zero_activity_and_empty_history_can_complete_without_point_count_rules() {
    let mut frames = frames();
    let baseline = rows(&mut frames[0], true).clone();
    for (index, frame) in frames.iter_mut().enumerate() {
        for (row, initial) in rows(frame, index == 0).iter_mut().zip(&baseline) {
            row[3] = json!("0");
            row[4] = json!("0");
            if row[7] != initial[7] {
                row[7] = json!(" data-chart-data=\"[]\"");
            }
        }
    }
    for length in 1..frames.len() {
        assert!(replay(&frames[..length], time()).is_err());
    }
    let snapshot = replay(&frames, time()).unwrap();
    assert!(
        snapshot
            .servers
            .iter()
            .all(|s| s.daily_active == 0 && s.monthly_active == 0)
    );
}

#[test]
fn positive_counts_and_duplicate_updates_do_not_establish_readiness() {
    let mut frames = frames();
    let baseline = rows(&mut frames[0], true).clone();
    for frame in &mut frames[1..] {
        for (row, initial) in rows(frame, false).iter_mut().zip(&baseline) {
            row[7] = initial[7].clone();
        }
    }
    assert!(replay(&frames, time()).is_err());
    let mut partial = common::frames()[..2].to_vec();
    partial.extend(vec![partial[1].clone(); 10]);
    assert!(replay(&partial, time()).is_err());
}

#[test]
fn malformed_frames_wrong_channels_rejected_joins_and_unsupported_rendering_fail() {
    for bad in [
        json!({}),
        json!([1, 2]),
        json!(["1","1","lv:fixture-liveview","phx_reply",{"status":"error","response":{}}]),
        json!(["1", null, "wrong-topic", "diff", {}]),
        json!(["1", null, "lv:fixture-liveview", "diff", {}]),
    ] {
        assert!(replay(&[bad], time()).is_err());
    }
    let mut frames = frames();
    frames[0][4]["response"]["rendered"]["c"] = json!({});
    assert!(replay(&frames, time()).is_err());
}

fn accept(listener: &TcpListener) -> TcpStream {
    listener.set_nonblocking(true).unwrap();
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        match listener.accept() {
            Ok((stream, _)) => {
                stream.set_nonblocking(false).unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(3)))
                    .unwrap();
                stream
                    .set_write_timeout(Some(Duration::from_secs(3)))
                    .unwrap();
                return stream;
            }
            Err(error)
                if error.kind() == std::io::ErrorKind::WouldBlock && Instant::now() < deadline =>
            {
                thread::sleep(Duration::from_millis(10));
            }
            Err(error) => panic!("local fixture server did not receive a connection: {error}"),
        }
    }
}

fn http_request(stream: &mut TcpStream) -> String {
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    let mut bytes = Vec::new();
    while !bytes.ends_with(b"\r\n\r\n") {
        let mut byte = [0];
        stream.read_exact(&mut byte).unwrap();
        bytes.push(byte[0]);
    }
    String::from_utf8(bytes).unwrap()
}

fn reply(stream: &mut TcpStream, status: &str, body: &str, cookie: bool) {
    let cookie = if cookie {
        "Set-Cookie: fixture=anonymous; Path=/\r\n"
    } else {
        ""
    };
    write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Length: {}\r\n{cookie}Connection: close\r\n\r\n{body}",
        body.len()
    )
    .unwrap();
}

#[test]
#[allow(clippy::result_large_err)] // Tungstenite fixes the handshake callback's error type.
fn initializes_http_session_then_receives_delayed_local_websocket_updates() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let source = format!("http://{}/metrics", listener.local_addr().unwrap());
    let source_in_server = source.clone();
    let server = thread::spawn(move || {
        let mut stream = accept(&listener);
        let request = http_request(&mut stream);
        assert!(request.starts_with("GET /metrics "));
        let user_agent = request
            .lines()
            .filter_map(|line| line.split_once(':'))
            .find(|(name, _)| name.eq_ignore_ascii_case("user-agent"))
            .expect("HTTP initialization must identify the collector")
            .1
            .trim()
            .to_owned();
        assert!(user_agent.starts_with("mnm-stats-collector/"));
        assert!(user_agent.ends_with("+https://github.com/Tiendil/monsters-and-memories-stats)"));
        // The build-metadata checks supply an independent expected wire value.
        if let Ok(expected) = std::env::var("MNM_STATS_EXPECT_USER_AGENT") {
            assert_eq!(user_agent, expected);
        }
        reply(&mut stream, "200 OK", INITIALIZATION, true);
        drop(stream);
        let stream = accept(&listener);
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let mut socket = tungstenite::accept_hdr(
            stream,
            |req: &tungstenite::handshake::server::Request, response| {
                assert!(req.uri().to_string().starts_with("/live/websocket?"));
                assert!(req.uri().to_string().contains("_csrf_token=fixture-csrf"));
                assert_eq!(req.headers()["Cookie"], "fixture=anonymous");
                assert_eq!(req.headers()["User-Agent"], user_agent);
                Ok(response)
            },
        )
        .unwrap();
        let join: Value = serde_json::from_str(socket.read().unwrap().to_text().unwrap()).unwrap();
        assert_eq!(join[2], "lv:fixture-liveview");
        assert_eq!(join[3], "phx_join");
        assert_eq!(join[4]["session"], "fixture-session");
        assert_eq!(join[4]["url"], source_in_server);
        for frame in frames() {
            socket
                .send(Message::Text(frame.to_string().into()))
                .unwrap();
            thread::sleep(Duration::from_millis(10));
        }
        assert!(matches!(socket.read().unwrap(), Message::Close(_)));
    });
    let snapshot = collect(&source, Duration::from_secs(2)).unwrap();
    assert_eq!(snapshot.active_subscriptions, 18_910);
    server.join().unwrap();
}

#[test]
fn http_status_session_errors_and_missing_cookie_fail_without_websocket_fallback() {
    for (status, body, cookie) in [
        ("503 Unavailable", INITIALIZATION, true),
        ("302 Found", INITIALIZATION, true),
        ("200 OK", "<html></html>", true),
        ("200 OK", INITIALIZATION, false),
    ] {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let source = format!("http://{}/metrics", listener.local_addr().unwrap());
        let server = thread::spawn(move || {
            let mut stream = accept(&listener);
            http_request(&mut stream);
            reply(&mut stream, status, body, cookie);
        });
        assert!(collect(&source, Duration::from_secs(1)).is_err());
        server.join().unwrap();
    }
}

#[test]
fn http_and_websocket_timeouts_disconnects_and_bad_payloads_fail() {
    for mode in [
        "http-timeout",
        "upgrade-error",
        "socket-timeout",
        "close",
        "bad-json",
    ] {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let source = format!("http://{}/metrics", listener.local_addr().unwrap());
        let server = thread::spawn(move || {
            let mut stream = accept(&listener);
            http_request(&mut stream);
            if mode == "http-timeout" {
                thread::sleep(Duration::from_secs(1));
                return;
            }
            reply(&mut stream, "200 OK", INITIALIZATION, true);
            drop(stream);
            let mut stream = accept(&listener);
            if mode == "upgrade-error" {
                http_request(&mut stream);
                reply(&mut stream, "403 Forbidden", "", false);
                return;
            }
            let mut socket = tungstenite::accept(stream).unwrap();
            socket.read().unwrap();
            match mode {
                "socket-timeout" => thread::sleep(Duration::from_secs(1)),
                "close" => socket.close(None).unwrap(),
                "bad-json" => socket.send(Message::Text("{broken".into())).unwrap(),
                _ => unreachable!(),
            }
        });
        assert!(
            collect(&source, Duration::from_millis(500)).is_err(),
            "{mode}"
        );
        server.join().unwrap();
    }
}
