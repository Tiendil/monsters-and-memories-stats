use crate::{analysis::*, charts, charts::browser::InteractivePlot};
use chrono::{DateTime, Datelike, Utc};
use leptos::prelude::*;
use mnm_stats_model::History;
use std::{
    collections::BTreeSet,
    sync::{Arc, Mutex},
};
use wasm_bindgen::{JsCast, JsValue};

const REPOSITORY_URL: &str = "https://github.com/Tiendil/monsters-and-memories-stats";

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

fn utc(time: DateTime<Utc>) -> String {
    time.to_rfc3339_opts(chrono::SecondsFormat::AutoSi, true)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Overview,
    Entities,
    Months,
    Years,
    Intervals,
}
impl Mode {
    const ALL: [Self; 5] = [
        Self::Overview,
        Self::Entities,
        Self::Months,
        Self::Years,
        Self::Intervals,
    ];
    fn key(self) -> &'static str {
        match self {
            Self::Overview => "overview",
            Self::Entities => "entities",
            Self::Months => "months",
            Self::Years => "years",
            Self::Intervals => "intervals",
        }
    }
    fn label(self) -> &'static str {
        match self {
            Self::Overview => "No comparison",
            Self::Entities => "Servers",
            Self::Months => "Periods · months",
            Self::Years => "Periods · years",
            Self::Intervals => "Periods · equal intervals",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Section {
    Overview,
    Activity,
    Population,
    Relationships,
}
impl Section {
    const ALL: [Self; 4] = [
        Self::Overview,
        Self::Activity,
        Self::Population,
        Self::Relationships,
    ];
    fn label(self) -> &'static str {
        match self {
            Self::Overview => "Overview",
            Self::Activity => "Activity",
            Self::Population => "Population",
            Self::Relationships => "Relationships",
        }
    }
    fn key(self) -> &'static str {
        match self {
            Self::Overview => "overview",
            Self::Activity => "activity",
            Self::Population => "population",
            Self::Relationships => "relationships",
        }
    }
}
fn readable(time: DateTime<Utc>) -> String {
    time.format("%d %b %Y, %H:%M UTC").to_string()
}

#[component]
fn Summary(
    history: Arc<History>,
    scope: RwSignal<Scope>,
    range: RwSignal<TimeRange>,
    now: RwSignal<DateTime<Utc>>,
    section: RwSignal<Section>,
) -> impl IntoView {
    let names = servers(&history);
    let selected = Memo::new(move |_| latest_in_range(&history, range.get(), now.get()).cloned());
    view! {
        <div class="summary-heading"><h2>"At a glance"</h2><p id="summary-time">{move || selected.get().map_or_else(|| "No observations in this interval".into(), |s| format!("Last in range · {}", readable(s.observed_at)))}</p></div>
        <p class="scope-caption">{move || if scope.get() == Scope::All { "Sum across servers; not deduplicated.".to_string() } else { scope.get().label(&names) }}</p>
        <div class="headline-grid">{[(Metric::Online, "Online population", Section::Population), (Metric::Daily, "Daily active", Section::Activity), (Metric::Monthly, "Monthly active", Section::Activity), (Metric::Subscriptions, "Global subscriptions", Section::Activity)].into_iter().map(|(metric, label, target)| {
            let key = metric.key();
            view! { <article class="headline" data-summary=key>
                <h3><button class="text-action" aria-label=format!("{label}: open {}", target.label()) on:click=move |_| { section.set(target); if let Some(el) = document().get_element_by_id(&format!("nav-{}", target.key())).and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok()) { let _ = el.focus(); } }>{label}" →"</button></h3><p class="headline-value">{move || selected.get().and_then(|s| metric.value(&s, &scope.get())).map_or_else(|| "Not available".into(), |v| match v { MetricValue::Count(n) => grouped_count(n), _ => unreachable!() })}</p>
            </article> }
        }).collect_view()}</div>
    }
}

#[component]
fn PeriodPicker(periods: RwSignal<Vec<Period>>, yearly: bool, initial: String) -> impl IntoView {
    let value = RwSignal::new(initial);
    let error = RwSignal::new(None::<String>);
    let add = move |_| {
        let result = if yearly {
            Period::year(&value.get())
        } else {
            Period::month(&value.get())
        };
        match result {
            Ok(period) => {
                periods.update(|p| {
                    if !p.contains(&period) {
                        p.push(period);
                    }
                });
                error.set(None);
            }
            Err(message) => error.set(Some(message)),
        }
    };
    view! {
        <div class="period-picker">
            <label> {if yearly { "Calendar year (UTC)" } else { "Calendar month (UTC)" }}
                <input id="period-input" type="text" inputmode=if yearly { "numeric" } else { "text" }
                    aria-describedby="period-error" aria-invalid=move || error.get().is_some().to_string()
                    placeholder=if yearly { "YYYY" } else { "YYYY-MM" }
                    prop:value=move || value.get() on:input=move |ev| value.set(event_target_value(&ev))/>
            </label>
            <button id="add-period" on:click=add>"Add period"</button>
            <p id="period-error" class="error" role="alert">{move || error.get()}</p>
            <ul class="selections">{move || periods.get().into_iter().enumerate().map(|(i, period)| view! {
                <li><span>{period.label()}</span><button class="secondary remove-period" aria-label=format!("Remove {}", period.label()) on:click=move |_| { periods.update(|p| { p.remove(i); }); if let Some(el) = document().get_element_by_id("period-input").and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok()) { let _ = el.focus(); } }>"Remove"</button></li>
            }).collect_view()}</ul>
        </div>
    }
}

#[component]
fn ChartCard(
    history: Arc<History>,
    metric: Metric,
    scope: RwSignal<Scope>,
    range: RwSignal<TimeRange>,
    now: RwSignal<DateTime<Utc>>,
    comparison: Memo<Result<Comparison, String>>,
) -> impl IntoView {
    let key = metric.key();
    let title = metric.title();
    let description = metric.description();
    let unit = metric.unit();
    let chart_metric = metric.clone();
    let styles = expect_context::<Arc<Mutex<charts::SeriesStyles>>>();
    let plotted = Memo::new(move |_| {
        comparison.get().and_then(|comparison| {
            plot(
                &history,
                &metric,
                &scope.get(),
                range.get(),
                now.get(),
                &comparison,
            )
            .map(|mut plot| {
                styles.lock().expect("series styles").assign(&mut plot);
                Arc::new(plot)
            })
        })
    });
    let open = RwSignal::new(false);
    let page = RwSignal::new(0_usize);
    Effect::new(move |_| {
        plotted.track();
        page.set(0);
    });
    view! {
        <article class="chart-card" data-metric=key>
            <div class="chart-heading"><h3>{title.clone()}</h3><span class="unit">{unit}</span></div>
            {move || match plotted.get() {
                Err(error) => view! { <p class="error" role="alert">{error}</p> }.into_any(),
                Ok(plot) => {
                    let has_values = plot.series.iter().any(|s| s.points.iter().any(|p| p.value.is_some()));
                    let count = plot.series.iter().map(|s| s.points.iter().filter(|p| p.value.is_some()).count()).sum::<usize>();
                    view! {
                        {if plot.series.len() > 1 { view! {
                            <ul class="legend">{plot.series.iter().enumerate().map(|(index, series)| view! {
                                <li><svg class="swatch" viewBox="0 0 48 8" aria-hidden="true"><line x1="0" y1="4" x2="48" y2="4" stroke=charts::css_color(series.style) stroke-dasharray=charts::dash_array(series.style)/></svg>{format!("{}. {}", index + 1, series.label)}</li>
                            }).collect_view()}</ul>
                        }.into_any() } else { view! { <p class="chart-scope">{plot.series.first().map(|s| s.label.clone())}</p> }.into_any() }}
                        <p class="chart-note">{plot.note.clone()}</p>
                        {if has_values {
                            view! { <InteractivePlot plot=plot.clone() metric=chart_metric.clone()/> }.into_any()
                        } else {
                            view! { <p class="empty-chart">"No available observations for this selection."</p> }.into_any()
                        }}
                        <p class="axis-label">{plot.alignment.description()}</p>
                        <p class="sample-count">{format!("{count} plotted observations")}</p>
                    }.into_any()
                }
            }}
            <details class="exact-values" prop:open=move || open.get() on:toggle=move |ev| open.set(event_target::<web_sys::HtmlDetailsElement>(&ev).open())>
                <summary>"View data"</summary>
                <p class="metric-description">{description}</p>
                <p>"Lines break across gaps longer than two hours. Daily/monthly counts retain the source’s unverified counting units and windows; starting-zone counts do not identify new players."</p>
                <Show when=move || open.get()>
                    {move || plotted.get().ok().map(|plot| {
                        let total = plot.series.iter().map(|s| s.points.len()).sum::<usize>();
                        let offset = page.get() * 50;
                        let rows = plot.series.iter().enumerate().flat_map(|(i, series)| series.points.iter().map(move |point| (i, &series.label, point)))
                            .skip(offset).take(50).map(|(index, label, point)| view! {
                                <tr data-series=index data-at=utc(point.at)>
                                    <td>{format!("{}. {label}", index + 1)}</td><td>{utc(point.at)}</td>
                                    <td class="exact-value">{point.value.map_or_else(|| "not available".into(), MetricValue::display)}</td>
                                </tr>
                            }).collect_view();
                        view! {
                            <p>"Counts are exact. Ratios show rounded percentages and their exact numerator / denominator."</p>
                            <div class="table-scroll" tabindex="0" aria-label="Scrollable exact observations">
                                <table><caption>"Selected observations, with original UTC timestamps"</caption><thead><tr><th scope="col">"Series"</th><th scope="col">"Observed at (UTC)"</th><th scope="col">"Value"</th></tr></thead><tbody>{rows}</tbody></table>
                            </div>
                            <div class="pagination">
                                <button class="secondary previous" disabled=move || page.get() == 0 on:click=move |_| page.update(|p| *p = p.saturating_sub(1))>"Previous"</button>
                                <span>{format!("{}–{} of {total}", if total == 0 { 0 } else { offset + 1 }, (offset + 50).min(total))}</span>
                                <button class="secondary next" disabled={offset + 50 >= total} on:click=move |_| page.update(|p| *p += 1)>"Next"</button>
                            </div>
                        }
                    })}
                </Show>
            </details>
        </article>
    }
}

#[component]
fn Correlations(
    history: Arc<History>,
    scope: RwSignal<Scope>,
    range: RwSignal<TimeRange>,
    now: RwSignal<DateTime<Utc>>,
) -> impl IntoView {
    let names = servers(&history);
    let range_history = history.clone();
    view! {
        <section class="correlations" aria-labelledby="correlations-heading">
            <h2 id="correlations-heading">"Correlations"</h2>
            <p>"Pearson's r uses the last jointly available observation per UTC day in the shared time range. At least three paired days and variation in both counts are needed."</p>
            <p id="correlation-scope">{move || { let (start, end) = range.get().bounds(&range_history, now.get()); format!("{} · {} · {} – {}", scope.get().label(&names), range.get().label(), readable(start), readable(end)) }}</p>
            <div class="correlation-grid">{move || {
                let (start, end) = range.get().bounds(&history, now.get());
                [(Metric::Daily, Metric::Monthly), (Metric::Daily, Metric::Subscriptions), (Metric::Monthly, Metric::Subscriptions)].into_iter().map(|(a, b)| {
                    let result = correlation(&history, &a, &b, &scope.get(), start, end);
                    view! { <div class="correlation-value"><h3>{format!("{} / {}", a.title(), b.title())}</h3>
                        <p class="coefficient">{result.r.map_or_else(|| "not available".into(), |r| format!("r = {r:.3}"))}</p>
                        <p>{format!("{} paired UTC days", result.paired_days)}</p>
                    </div> }
                }).collect_view()
            }}</div>
            <p class="muted">"Correlation does not establish causation. Overlapping source activity windows limit interpretation. Subscriptions remain global; activity sums are not deduplicated."</p>
        </section>
    }
}

#[component]
pub fn App() -> impl IntoView {
    let history = Arc::new(crate::embedded_history());
    provide_context(Arc::new(Mutex::new(charts::SeriesStyles::default())));
    let section = RwSignal::new(Section::Overview);
    let count = history.snapshots().len();
    let latest = history.snapshots().last().map(|s| s.observed_at);
    let first = history.snapshots().first().map(|s| s.observed_at);
    let names = servers(&history);
    let zone_names = zones(&history);
    let zone = RwSignal::new(zone_names.keys().next().cloned().unwrap_or_default());
    let utc_now =
        || DateTime::from_timestamp_millis(js_sys::Date::now() as i64).expect("browser timestamp");
    let now = RwSignal::new(utc_now());
    let timer = set_interval_with_handle(
        move || now.set(utc_now()),
        std::time::Duration::from_secs(60),
    )
    .expect("browser timer");
    on_cleanup(move || timer.clear());
    let scope = RwSignal::new(Scope::All);
    let range = RwSignal::new(TimeRange::default());
    let mode = RwSignal::new(Mode::Overview);
    let mut initial_entities = vec![Scope::All];
    initial_entities.extend(names.keys().take(2).cloned().map(Scope::Server));
    let entities = RwSignal::new(initial_entities);
    let distinct_months: BTreeSet<_> = history
        .snapshots()
        .iter()
        .map(|s| s.observed_at.format("%Y-%m").to_string())
        .collect();
    let distinct_years: BTreeSet<_> = history
        .snapshots()
        .iter()
        .map(|s| s.observed_at.year())
        .collect();
    let months = RwSignal::new(
        distinct_months
            .iter()
            .rev()
            .take(3)
            .rev()
            .map(|m| Period::month(m).unwrap())
            .collect::<Vec<_>>(),
    );
    let years = RwSignal::new(
        distinct_years
            .iter()
            .rev()
            .take(3)
            .rev()
            .map(|y| Period::Year(*y))
            .collect::<Vec<_>>(),
    );
    let starts = RwSignal::new(Vec::<DateTime<Utc>>::new());
    let start_input = RwSignal::new(
        latest
            .unwrap_or_else(utc_now)
            .format("%Y-%m-%dT%H:%M")
            .to_string(),
    );
    let hours = RwSignal::new("168".to_string());
    let interval_error = RwSignal::new(None::<String>);
    let download_error = RwSignal::new(None::<String>);
    let comparison = Memo::new(move |_| -> Result<Comparison, String> {
        Ok(match mode.get() {
            Mode::Overview => Comparison::None,
            Mode::Entities => Comparison::Entities(entities.get()),
            Mode::Months => Comparison::Periods(months.get()),
            Mode::Years => Comparison::Periods(years.get()),
            Mode::Intervals => {
                let hours = hours
                    .get()
                    .parse::<u32>()
                    .ok()
                    .filter(|h| *h > 0)
                    .ok_or("Choose a positive duration in hours.")?;
                Comparison::Periods(
                    starts
                        .get()
                        .into_iter()
                        .map(|start| Period::Interval { start, hours })
                        .collect(),
                )
            }
        })
    });
    let download_history = history.clone();
    let range_history = history.clone();
    let interval_history = history.clone();
    let entities_names = names.clone();
    let selected_history = history.clone();
    let month_default = latest.unwrap_or_else(utc_now).format("%Y-%m").to_string();
    let year_default = latest.unwrap_or_else(utc_now).format("%Y").to_string();
    let summary_history = history.clone();
    let empty_history = history.clone();
    let empty_range =
        Memo::new(move |_| latest_in_range(&empty_history, range.get(), now.get()).is_none());
    let chip_names = StoredValue::new(entities_names.clone());
    let summary_names = StoredValue::new(entities_names.clone());
    view! {
        <style>{include_str!(concat!(env!("OUT_DIR"), "/style.css"))}</style>
        <a class="skip-link" href="#content">"Skip to dashboard content"</a>
        <main>
            {matches!(env!("MNM_STATS_DEMO"), "1").then(|| view! {
                <p id="demo-notice" role="status">"Demo · synthetic data and server names."</p>
            })}
            <header class="page-header">
                <div><p class="eyebrow">"Independent community statistics"</p><h1>"Monsters & Memories"</h1></div>
                <div class="header-actions">
                    <div class="download"><button class="secondary" id="download-history" aria-describedby="download-help" on:click=move |_| download_error.set(download(&download_history).err().map(|_| "The history download could not be created. Please try again.".into()))>"Download JSON"</button><p id="download-help">"Complete archive"</p></div>
                    <a class="button-link secondary" href=REPOSITORY_URL>
                        <svg class="action-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" focusable="false"><polygon points="12 3 14.8 8.7 21 9.6 16.5 14 17.6 20.2 12 17.3 6.4 20.2 7.5 14 3 9.6 9.2 8.7"/></svg>
                        "Star on GitHub"
                    </a>
                </div>
            </header>
            <p class="error" role="alert">{move || download_error.get()}</p>
            <section class="history-summary" aria-label="Collection status">
                <p>"Hourly observations · Latest: "<time id="latest-collection" datetime=latest.map(utc)>{latest.map_or_else(|| "not available yet".into(), readable)}</time></p>
                <p id="freshness" role="status" class:stale=move || latest.is_some_and(|t| crate::is_stale(t, now.get()))>{move || latest.map_or("Awaiting the first successful collection.", |t| if crate::is_stale(t, now.get()) { "Stale data: the latest collection is more than three hours old." } else { "Collected within 3 hours." })}</p>
                <details id="archive-details"><summary>"Archive details"</summary><p id="history-count">{format!("{count} observations")}</p><p id="history-status">{first.zip(latest).map_or_else(|| "No observations have been collected yet.".into(), |(first, last)| format!("Available history: {} to {} UTC", utc(first), utc(last)))}</p></details>
            </section>
            <section class="controls" aria-labelledby="controls-heading">
                <h2 id="controls-heading" class="visually-hidden">"Explore the archive"</h2>
                <div class="control-grid">
                    <label class:hidden=move || mode.get() == Mode::Entities && section.get() != Section::Relationships>{move || if mode.get() == Mode::Entities { "Correlation scope" } else { "Server scope" }}<select id="server-scope" prop:value=move || match scope.get() { Scope::All => String::new(), Scope::Server(id) => id } on:change=move |ev| {
                        let value = event_target_value(&ev); scope.set(if value.is_empty() { Scope::All } else { Scope::Server(value) });
                    }><option value="">"All servers (sum)"</option>{names.into_iter().map(|(id, name)| view! { <option value=id.clone()>{format!("{name} [{id}]")}</option> }).collect_view()}</select></label>
                    <label class:hidden=move || matches!(mode.get(), Mode::Months | Mode::Years | Mode::Intervals) && section.get() != Section::Relationships>{move || if mode.get() == Mode::Overview || mode.get() == Mode::Entities { "Time range" } else { "Correlation range" }}<select id="time-range" prop:value=move || range.get().key() on:change=move |ev| {
                        let value = event_target_value(&ev);
                        if let Some(selected) = TimeRange::ALL.into_iter().find(|r| r.key() == value) { range.set(selected); }
                    }>{TimeRange::ALL.into_iter().map(|r| view! { <option value=r.key()>{r.label()}</option> }).collect_view()}</select></label>
                    <label>"Compare"<select id="comparison-mode" prop:value=move || mode.get().key() on:change=move |ev| {
                        let value = event_target_value(&ev); if let Some(selected) = Mode::ALL.into_iter().find(|m| m.key() == value) { mode.set(selected); }
                    }>{Mode::ALL.into_iter().map(|m| view! { <option value=m.key()>{m.label()}</option> }).collect_view()}</select></label>
                </div>
                <details class="range-details" class:hidden=move || matches!(mode.get(), Mode::Months | Mode::Years | Mode::Intervals) && section.get() != Section::Relationships><summary>{move || { let (start, end) = range.get().bounds(&interval_history, now.get()); { let format = if start.year() == end.year() { "%d %b" } else { "%d %b %Y" }; format!("{} – {} UTC", start.format(format), end.format(format)) } }}</summary><p id="selected-interval">{move || { let (start, end) = range.get().bounds(&range_history, now.get()); format!("Shared range: {} to {} UTC", utc(start), utc(end)) }}</p></details>
                <p id="selected-observations" class:hidden=move || matches!(mode.get(), Mode::Months | Mode::Years | Mode::Intervals) && section.get() != Section::Relationships>{move || {
                    let (start, end) = range.get().bounds(&selected_history, now.get());
                    let count = selected_history.snapshots().iter().filter(|s| s.observed_at >= start && s.observed_at <= end).count();
                    if count == 0 { "No observations in the shared time range.".into() } else { format!("{count} observations") }
                }}</p>
                <Show when=move || mode.get() == Mode::Entities>
                    <fieldset id="entity-choices"><legend>"Compare entities · select any number"</legend>
                        {std::iter::once(Scope::All).chain(entities_names.keys().cloned().map(Scope::Server)).map(|entity| {
                            let label = entity.label(&entities_names); let checked = entity.clone();
                            let value = match &entity { Scope::All => "all".to_string(), Scope::Server(id) => format!("server:{id}") };
                            view! { <label class="checkbox"><input type="checkbox" value=value prop:checked=move || entities.get().contains(&checked) on:change=move |_| entities.update(|list| {
                                if list.contains(&entity) { list.retain(|e| e != &entity); } else { list.push(entity.clone()); }
                            })/>{label}</label> }
                        }).collect_view()}
                        <p>"All-server sums include the selected individual servers; activity counts are not deduplicated."</p>
                    </fieldset>
                    <ul class="selections entity-selections">{move || entities.get().into_iter().map(|entity| {
                        let label = entity.label(&chip_names.get_value());
                        view! { <li><span>{label.clone()}</span><button class="secondary remove-entity" aria-label=format!("Remove {label}") on:click=move |_| { entities.update(|list| list.retain(|e| e != &entity)); if let Some(el) = document().get_element_by_id("comparison-mode").and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok()) { let _ = el.focus(); } }>"Remove"</button></li> }
                    }).collect_view()}</ul>
                </Show>
                {move || match mode.get() {
                    Mode::Months => view! { <PeriodPicker periods=months yearly=false initial=month_default.clone()/> }.into_any(),
                    Mode::Years => view! { <PeriodPicker periods=years yearly=true initial=year_default.clone()/> }.into_any(),
                    Mode::Intervals => view! {
                        <div class="period-picker">
                            <label>"Duration for every interval (hours)"<input id="interval-hours" aria-describedby="interval-error" aria-invalid=move || comparison.get().is_err().to_string() type="number" min="1" step="1" prop:value=move || hours.get() on:input=move |ev| hours.set(event_target_value(&ev))/></label>
                            <label>"Start (UTC, YYYY-MM-DDTHH:MM)"<input id="interval-start" aria-describedby="interval-error" aria-invalid=move || interval_error.get().is_some().to_string() type="text" prop:value=move || start_input.get() on:input=move |ev| start_input.set(event_target_value(&ev))/></label>
                            <button id="add-interval" on:click=move |_| {
                                let result = hours.get().parse::<u32>().map_err(|_| "Choose a positive duration in hours.".to_string()).and_then(|h| Period::interval(&start_input.get(), h));
                                match result {
                                    Ok(Period::Interval { start, .. }) => { starts.update(|s| { if !s.contains(&start) { s.push(start); } }); interval_error.set(None); }
                                    Err(error) => interval_error.set(Some(error)), _ => unreachable!(),
                                }
                            }>"Add interval"</button>
                            <p id="interval-error" role="alert">{move || interval_error.get().or_else(|| comparison.get().err())}</p>
                            <ul class="selections">{move || starts.get().into_iter().enumerate().map(|(i, start)| view! {
                                <li><span>{utc(start)}</span><button class="secondary remove-period" aria-label=format!("Remove interval starting {}", utc(start)) on:click=move |_| { starts.update(|s| { s.remove(i); }); if let Some(el) = document().get_element_by_id("interval-start").and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok()) { let _ = el.focus(); } }>"Remove"</button></li>
                            }).collect_view()}</ul>
                        </div>
                    }.into_any(),
                    _ => ().into_any(),
                }}
                <Show when=move || matches!(mode.get(), Mode::Months | Mode::Years | Mode::Intervals)>
                    <p class="comparison-explanation">"Periods align by calendar date or elapsed hours. Incomplete periods remain gaps. Correlations use their own shared range and scope."</p>
                </Show>
            </section>
            <nav class="section-nav" aria-label="Dashboard sections">{Section::ALL.into_iter().map(|item| view! {
                <button id=format!("nav-{}", item.key()) aria-pressed=move || (section.get() == item).to_string() on:click=move |_| section.set(item)>{item.label()}</button>
            }).collect_view()}</nav>
            <section id="content" tabindex="-1" class:comparing=move || mode.get() != Mode::Overview aria-label=move || section.get().label()>
                <Show when=move || mode.get() == Mode::Overview && empty_range.get()>
                    <div class="empty-selection" role="status"><p>{if count == 0 { "No history yet. The first successful collection will appear in a future dashboard build." } else { "No observations in this interval. Choose All time to explore the available archive." }}</p>
                    {(count > 0).then(|| view! { <button class="secondary" on:click=move |_| range.set(TimeRange::All)>"Show All time"</button> })}</div>
                </Show>
                <Show when=move || section.get() == Section::Overview && mode.get() == Mode::Overview>
                    <Summary history=summary_history.clone() scope range now section/>
                </Show>
                <Show when=move || mode.get() != Mode::Overview>
                    <div class="comparison-summary" role="status"><h2>"Comparison"</h2><p>{move || match comparison.get() {
                        Ok(Comparison::Entities(items)) => if items.is_empty() { "Select servers above to compare.".into() } else { items.iter().map(|s| s.label(&summary_names.get_value())).collect::<Vec<_>>().join(" · ") },
                        Ok(Comparison::Periods(items)) => if items.is_empty() { "Add a period above to compare.".into() } else { items.iter().map(Period::label).collect::<Vec<_>>().join(" · ") },
                        Err(e) => e, _ => String::new()
                    }}</p></div>
                </Show>
                {move || {
                    let selected = section.get();
                    let metrics = match selected {
                        Section::Overview => vec![Metric::Online],
                        Section::Activity => vec![Metric::Daily, Metric::Monthly, Metric::Subscriptions],
                        Section::Population => vec![Metric::Online, Metric::StartingZones],
                        Section::Relationships => vec![Metric::DailyMonthly, Metric::DailySubscriptions, Metric::MonthlySubscriptions],
                    };
                    let chart_history = history.clone();
                    let zone_history = history.clone();
                    let zone_options = zone_names.clone();
                    let zone_labels = zone_names.clone();
                    view! {
                        <h2 class="section-title">{match selected { Section::Overview => "Online over time", Section::Activity => "Activity over time", Section::Population => "Population over time", Section::Relationships => "Ratios of reported counts" }}</h2>
                        {(selected != Section::Overview).then(|| view! { <p class="scope-caption">{move || if scope.get() == Scope::All && mode.get() != Mode::Entities { "Sum across servers; activity is not deduplicated. Subscriptions are global." } else { "Daily/monthly activity retains the source’s counting units. Subscriptions are global." }}</p> })}
                        <div class="chart-grid" class:overview-chart=selected == Section::Overview>{metrics.into_iter().map(|metric| view! { <ChartCard history=chart_history.clone() metric scope range now comparison/> }).collect_view()}</div>
                        {(selected == Section::Population).then(move || view! {
                            <section class="zone-detail" aria-label="Individual starting zone"><h2>"Explore a starting zone"</h2>
                                <label>"Starting zone"<select id="zone-scope" prop:value=move || zone.get() on:change=move |ev| zone.set(event_target_value(&ev))>{zone_options.into_iter().map(|(id, name)| view! { <option value=id.clone()>{format!("{name} [{id}]")}</option> }).collect_view()}</select></label>
                                {move || zone_labels.get(&zone.get()).map(|name| view! { <ChartCard history=zone_history.clone() metric=Metric::Zone(zone.get(), name.clone()) scope range now comparison/> })}
                            </section>
                        })}
                        {(selected == Section::Relationships).then(|| view! {
                            <p class="correlation-explanation">"Correlations below always use the shared range and server scope, including when the charts compare different servers or periods."</p>
                            <Correlations history=history.clone() scope range now/>
                        })}
                    }
                }}
            </section>
            <details class="source-notes"><summary>"About the data"</summary>
                <p>"DAU and MAU retain the source's daily/monthly active values; their counting units and window boundaries are unverified. Adding observations cannot recover unique activity. Subscriptions are not assumed to represent unique people."</p>
                <p>"Per-server subscriptions and deduplicated global activity are unavailable. Starting-zone population is not a count of new players."</p>
                <p>"The archive starts with successful collections. Earlier history and missed intervals are unavailable. These hourly snapshots are not an exhaustive record of every change within the hour."</p>
                <p>"The download contains every observation in this dashboard build, across all servers and dates, regardless of the controls above."</p>
                <p>"Display type: IM Fell English by Igino Marini, "<a href="fonts/OFL.txt">"SIL Open Font License"</a>"."</p>
            </details>
            <footer>
                <p>"Independent community archive · All times UTC"</p>
                <nav class="footer-links" aria-label="Project links">
                    <a href="https://account.monstersandmemories.com/metrics">"Data source"</a>
                    <a href=REPOSITORY_URL>"GitHub"</a>
                    <a href="https://plotly.com/javascript/">"Charts by Plotly"</a>
                </nav>
            </footer>
        </main>
    }
}
