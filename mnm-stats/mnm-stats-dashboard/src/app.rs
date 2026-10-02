use leptos::prelude::*;
use mnm_stats_model::History;
use std::rc::Rc;
use wasm_bindgen::{JsCast, JsValue};

fn download(history: &History) -> Result<(), JsValue> {
    let json = history
        .to_json()
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    let parts = js_sys::Array::of1(&JsValue::from_str(&json));
    let options = web_sys::BlobPropertyBag::new();
    options.set_type("application/json");
    let blob = web_sys::Blob::new_with_str_sequence_and_options(&parts, &options)?;
    let url = web_sys::Url::create_object_url_with_blob(&blob)?;
    let result = (|| {
        let link = document()
            .create_element("a")?
            .dyn_into::<web_sys::HtmlAnchorElement>()?;
        link.set_href(&url);
        link.set_download("history.json");
        link.click();
        Ok(())
    })();
    web_sys::Url::revoke_object_url(&url)?;
    result
}

#[component]
pub fn App() -> impl IntoView {
    let history = Rc::new(crate::embedded_history());
    let count = history.snapshots().len();
    let latest = history.snapshots().last().map(|s| s.observed_at);
    let utc_now = || {
        chrono::DateTime::from_timestamp_millis(js_sys::Date::now() as i64)
            .expect("browser timestamp")
    };
    let now = RwSignal::new(utc_now());
    let timer = set_interval_with_handle(
        move || now.set(utc_now()),
        std::time::Duration::from_secs(60),
    )
    .expect("browser timer");
    on_cleanup(move || timer.clear());
    let interval = history
        .snapshots()
        .first()
        .zip(history.snapshots().last())
        .map(|(first, last)| {
            format!(
                "{} to {} UTC",
                first.observed_at.format("%Y-%m-%d %H:%M:%S"),
                last.observed_at.format("%Y-%m-%d %H:%M:%S")
            )
        });
    let error = RwSignal::new(None::<String>);
    view! {
        <main>
            <header>
                <p class="eyebrow">"Community statistics archive"</p>
                <h1>"Monsters & Memories"</h1>
                <p>"Hourly observations of the public game statistics."</p>
            </header>
            <section aria-labelledby="history-heading">
                <h2 id="history-heading">"Collected history"</h2>
                <p id="history-count">{format!("{count} observations")}</p>
                {match interval {
                    None => view! { <p id="history-status">"No observations have been collected yet."</p> }.into_any(),
                    Some(interval) => view! {
                        <p id="history-status">"Available history: "{interval}</p>
                        <p>"Charts and comparisons are under development. Download the complete history below."</p>
                    }.into_any(),
                }}
                {latest.map(|timestamp| view! {
                    <p>"Latest collection: "{timestamp.format("%Y-%m-%d %H:%M:%S UTC").to_string()}</p>
                    <p role="status">{move || if crate::is_stale(timestamp, now.get()) {
                        "Stale data: the latest collection is more than three hours old."
                    } else { "The latest collection is within the last three hours." }}</p>
                })}
                <button id="download-history" on:click=move |_| {
                    error.set(download(&history).err().map(|_| "The history download could not be created. Please try again.".to_string()));
                }>"Download complete history (JSON)"</button>
                <p role="alert">{move || error.get()}</p>
            </section>
            <footer>
                <p>"The archive starts with successful collections. Earlier history and missed intervals are unavailable."</p>
                <p>"Weekly active counts, deduplicated global activity, and per-server subscriptions are not available from the source."</p>
            </footer>
        </main>
    }
}
