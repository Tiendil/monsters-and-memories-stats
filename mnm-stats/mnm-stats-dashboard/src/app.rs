use crate::{analysis::*, charts};
use chrono::{DateTime, Datelike, Utc};
use leptos::prelude::*;
use mnm_stats_model::History;
use std::{collections::BTreeSet, sync::Arc};
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
            Self::Overview => "Overview",
            Self::Entities => "Compare entities",
            Self::Months => "Compare months",
            Self::Years => "Compare calendar years",
            Self::Intervals => "Compare equal-duration intervals",
        }
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
                    placeholder=if yearly { "YYYY" } else { "YYYY-MM" }
                    prop:value=move || value.get() on:input=move |ev| value.set(event_target_value(&ev))/>
            </label>
            <button id="add-period" on:click=add>"Add period"</button>
            <p class="error" role="alert">{move || error.get()}</p>
            <ul class="selections">{move || periods.get().into_iter().enumerate().map(|(i, period)| view! {
                <li><span>{period.label()}</span><button class="secondary remove-period" aria-label=format!("Remove {}", period.label()) on:click=move |_| periods.update(|p| { p.remove(i); })>"Remove"</button></li>
            }).collect_view()}</ul>
        </div>
    }
}

#[component]
fn InteractivePlot(plot: Arc<Plot>, metric: Metric) -> impl IntoView {
    let rendered = match charts::render(&plot, &metric) {
        Ok(rendered) => rendered,
        Err(error) => return view! { <p role="alert">"Chart unavailable: "{error}</p> }.into_any(),
    };
    let svg = rendered.svg.clone();
    let rendered = StoredValue::new(rendered);
    let hovered = RwSignal::new(Vec::<charts::PlotPoint>::new());
    let surface = NodeRef::<leptos::html::Div>::new();
    let tooltip_id = format!("tooltip-{}", metric.key());
    let description_id = tooltip_id.clone();
    let on_move = move |event: web_sys::MouseEvent| {
        let Some(svg) = surface
            .get()
            .and_then(|node| node.query_selector("svg").ok().flatten())
        else {
            return;
        };
        let bounds = svg.get_bounding_client_rect();
        let hits = rendered.with_value(|rendered| {
            rendered.nearby(
                f64::from(event.client_x()) - bounds.left(),
                f64::from(event.client_y()) - bounds.top(),
                bounds.width(),
                bounds.height(),
            )
        });
        hovered.set(hits);
    };
    view! {
        <div class="interactive-plot" on:mouseleave=move |_| hovered.set(Vec::new())>
            <div class="plot" role="img" aria-label=format!("{}; {}. Hover a point or inspect exact values below.", metric.title(), metric.unit())
                aria-describedby=move || (!hovered.get().is_empty()).then(|| description_id.clone())
                on:scroll=move |_| hovered.set(Vec::new())>
                <div class="plot-surface" node_ref=surface on:mousemove=on_move>
                    <div inner_html=svg></div>
                    {move || hovered.get().into_iter().map(|point| view! {
                        <span class="hover-marker" aria-hidden="true" style=format!("left:{}%;top:{}%;border-color:{}", f64::from(point.position.0) / f64::from(charts::WIDTH) * 100.0, f64::from(point.position.1) / f64::from(charts::HEIGHT) * 100.0, charts::css_color(point.series))></span>
                    }).collect_view()}
                </div>
            </div>
            <Show when=move || !hovered.get().is_empty()>
                <div class="plot-tooltip" id=tooltip_id.clone() role="tooltip">
                    <ul>{let plot = plot.clone(); move || hovered.get().into_iter().map(|hit| {
                        let series = &plot.series[hit.series];
                        let point = &series.points[hit.point];
                        view! {
                            <li data-series=hit.series data-at=utc(point.at)>
                                <span class="hover-series" style:color=charts::css_color(hit.series)>{format!("{}. {}", hit.series + 1, series.label)}</span>
                                <time class="hover-time" datetime=utc(point.at)>{utc(point.at)}" UTC"</time>
                                <strong class="hover-value">{point.value.map(MetricValue::display)}</strong>
                            </li>
                        }
                    }).collect_view()}</ul>
                </div>
            </Show>
        </div>
    }.into_any()
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
            .map(Arc::new)
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
            <p class="metric-description">{description}</p>
            {move || match plotted.get() {
                Err(error) => view! { <p class="error" role="alert">{error}</p> }.into_any(),
                Ok(plot) => {
                    let has_values = plot.series.iter().any(|s| s.points.iter().any(|p| p.value.is_some()));
                    let count = plot.series.iter().map(|s| s.points.iter().filter(|p| p.value.is_some()).count()).sum::<usize>();
                    view! {
                        <p class="chart-note">{plot.note.clone()}</p>
                        <ul class="legend">{plot.series.iter().enumerate().map(|(index, series)| view! {
                            <li><span class="swatch" style:background=charts::css_color(index)></span>{format!("{}. {}", index + 1, series.label)}</li>
                        }).collect_view()}</ul>
                        <p class="sample-count">{format!("{count} plotted observations")}</p>
                        {if has_values {
                            view! { <InteractivePlot plot=plot.clone() metric=chart_metric.clone()/> }.into_any()
                        } else {
                            view! { <p class="empty-chart">"No available observations for this selection."</p> }.into_any()
                        }}
                        <p class="axis-label">{plot.alignment.description()}" · Lines break across gaps longer than two hours."</p>
                    }.into_any()
                }
            }}
            <details class="exact-values" on:toggle=move |ev| open.set(event_target::<web_sys::HtmlDetailsElement>(&ev).open())>
                <summary>"Inspect exact values"</summary>
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
    view! {
        <section class="correlations" aria-labelledby="correlations-heading">
            <h2 id="correlations-heading">"Relationships over time"</h2>
            <p>"Pearson's r uses the last jointly available observation per UTC day in the shared time range. At least three paired days and variation in both counts are needed."</p>
            <p id="correlation-scope">{move || format!("{} · {}", scope.get().label(&names), range.get().label())}</p>
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
    let count = history.snapshots().len();
    let latest = history.snapshots().last().map(|s| s.observed_at);
    let first = history.snapshots().first().map(|s| s.observed_at);
    let names = servers(&history);
    let zone_names = zones(&history);
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
    let entities_names = names.clone();
    let selected_history = history.clone();
    let month_default = latest.unwrap_or_else(utc_now).format("%Y-%m").to_string();
    let year_default = latest.unwrap_or_else(utc_now).format("%Y").to_string();
    let mut metrics = vec![
        Metric::Daily,
        Metric::Monthly,
        Metric::Subscriptions,
        Metric::Online,
        Metric::StartingZones,
    ];
    metrics.extend(
        zone_names
            .into_iter()
            .map(|(id, name)| Metric::Zone(id, name)),
    );
    let ratio_metrics = [
        Metric::DailyMonthly,
        Metric::DailySubscriptions,
        Metric::MonthlySubscriptions,
    ];
    view! {
        <main>
            {matches!(env!("MNM_STATS_DEMO"), "1").then(|| view! {
                <p id="demo-notice" role="status">"Demonstration preview — all observations and server names are synthetic."</p>
            })}
            <header class="page-header">
                <div><p class="eyebrow">"Community statistics archive"</p><h1>"Monsters & Memories"</h1><p>"Hourly observations. A longer view of the world."</p></div>
                <button id="download-history" on:click=move |_| download_error.set(download(&download_history).err().map(|_| "The history download could not be created. Please try again.".into()))>"Download complete history (JSON)"</button>
            </header>
            <p class="error" role="alert">{move || download_error.get()}</p>
            <section class="history-summary" aria-label="Collection status">
                <div><p class="eyebrow">"Archive"</p><p id="history-count">{format!("{count} observations")}</p>
                    <p id="history-status">{first.zip(latest).map_or_else(|| "No observations have been collected yet.".into(), |(first, last)| format!("Available history: {} to {} UTC", utc(first), utc(last)))}</p>
                </div>
                <div><p class="eyebrow">"Latest collection"</p><p id="latest-collection">{latest.map_or_else(|| "Not available yet".into(), utc)}</p>
                    <p id="freshness" role="status" class:stale=move || latest.is_some_and(|t| crate::is_stale(t, now.get()))>{move || latest.map_or("Awaiting the first successful collection.", |t| if crate::is_stale(t, now.get()) { "Stale data: the latest collection is more than three hours old." } else { "The latest collection is within the last three hours." })}</p>
                </div>
            </section>
            <section class="controls" aria-labelledby="controls-heading">
                <h2 id="controls-heading">"Explore the archive"</h2>
                <div class="control-grid">
                    <label>"Time range"<select id="time-range" prop:value=move || range.get().key() on:change=move |ev| {
                        let value = event_target_value(&ev);
                        if let Some(selected) = TimeRange::ALL.into_iter().find(|r| r.key() == value) { range.set(selected); }
                    }>{TimeRange::ALL.into_iter().map(|r| view! { <option value=r.key()>{r.label()}</option> }).collect_view()}</select></label>
                    <label>"Server scope"<select id="server-scope" prop:value=move || match scope.get() { Scope::All => String::new(), Scope::Server(id) => id } on:change=move |ev| {
                        let value = event_target_value(&ev); scope.set(if value.is_empty() { Scope::All } else { Scope::Server(value) });
                    }><option value="">"All servers (sum)"</option>{names.into_iter().map(|(id, name)| view! { <option value=id.clone()>{format!("{name} [{id}]")}</option> }).collect_view()}</select></label>
                    <label>"View"<select id="comparison-mode" prop:value=move || mode.get().key() on:change=move |ev| {
                        let value = event_target_value(&ev); if let Some(selected) = Mode::ALL.into_iter().find(|m| m.key() == value) { mode.set(selected); }
                    }>{Mode::ALL.into_iter().map(|m| view! { <option value=m.key()>{m.label()}</option> }).collect_view()}</select></label>
                </div>
                <p id="selected-interval">{move || { let (start, end) = range.get().bounds(&range_history, now.get()); format!("Shared range: {} to {} UTC", utc(start), utc(end)) }}</p>
                <p id="selected-observations">{move || {
                    let (start, end) = range.get().bounds(&selected_history, now.get());
                    let count = selected_history.snapshots().iter().filter(|s| s.observed_at >= start && s.observed_at <= end).count();
                    if count == 0 { "No observations in the shared time range.".into() } else { format!("{count} observations in the shared time range.") }
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
                </Show>
                {move || match mode.get() {
                    Mode::Months => view! { <PeriodPicker periods=months yearly=false initial=month_default.clone()/> }.into_any(),
                    Mode::Years => view! { <PeriodPicker periods=years yearly=true initial=year_default.clone()/> }.into_any(),
                    Mode::Intervals => view! {
                        <div class="period-picker">
                            <label>"Duration for every interval (hours)"<input id="interval-hours" type="number" min="1" step="1" prop:value=move || hours.get() on:input=move |ev| hours.set(event_target_value(&ev))/></label>
                            <label>"Start (UTC, YYYY-MM-DDTHH:MM)"<input id="interval-start" type="text" prop:value=move || start_input.get() on:input=move |ev| start_input.set(event_target_value(&ev))/></label>
                            <button id="add-interval" on:click=move |_| {
                                let result = hours.get().parse::<u32>().map_err(|_| "Choose a positive duration in hours.".to_string()).and_then(|h| Period::interval(&start_input.get(), h));
                                match result {
                                    Ok(Period::Interval { start, .. }) => { starts.update(|s| { if !s.contains(&start) { s.push(start); } }); interval_error.set(None); }
                                    Err(error) => interval_error.set(Some(error)), _ => unreachable!(),
                                }
                            }>"Add interval"</button>
                            <p role="alert">{move || interval_error.get()}</p>
                            <ul class="selections">{move || starts.get().into_iter().enumerate().map(|(i, start)| view! {
                                <li><span>{utc(start)}</span><button class="secondary remove-period" aria-label=format!("Remove interval starting {}", utc(start)) on:click=move |_| starts.update(|s| { s.remove(i); })>"Remove"</button></li>
                            }).collect_view()}</ul>
                        </div>
                    }.into_any(),
                    _ => ().into_any(),
                }}
                <Show when=move || matches!(mode.get(), Mode::Months | Mode::Years | Mode::Intervals)>
                    <p class="comparison-explanation">"Charts use the selected periods and server scope. The shared range still controls correlations below. Calendar years align by month and day, unlike the rolling 365-day range. All boundaries are UTC; gaps and incomplete periods remain visible."</p>
                </Show>
            </section>
            <section class="metrics" aria-labelledby="counts-heading"><div class="section-heading"><h2 id="counts-heading">"Reported counts"</h2><p>"Source observations and clearly labeled sums"</p></div>
                <div class="chart-grid">{metrics.into_iter().map(|metric| view! { <ChartCard history=history.clone() metric scope range now comparison/> }).collect_view()}</div>
            </section>
            <section class="metrics" aria-labelledby="ratios-heading"><div class="section-heading"><h2 id="ratios-heading">"Ratios of reported counts"</h2><p>"Derived values · global subscription denominators"</p></div>
                <div class="chart-grid">{ratio_metrics.into_iter().map(|metric| view! { <ChartCard history=history.clone() metric scope range now comparison/> }).collect_view()}</div>
            </section>
            <Correlations history=history.clone() scope range now/>
            <section class="source-notes" aria-labelledby="source-heading"><h2 id="source-heading">"What these numbers can tell us"</h2>
                <p id="weekly-unavailable">"Weekly active (WAU): not available. The source does not publish a defensible weekly unique-activity count."</p>
                <p>"DAU and MAU retain the source's daily/monthly active values; their counting units and window boundaries are unverified. Adding observations cannot recover unique activity. Subscriptions are not assumed to represent unique people."</p>
                <p>"Per-server subscriptions and deduplicated global activity are unavailable. Starting-zone population is not a count of new players."</p>
                <p>"The archive starts with successful collections. Earlier history and missed intervals are unavailable. These hourly snapshots are not an exhaustive record of every change within the hour."</p>
                <p>"The download contains every observation in this dashboard build, across all servers and dates, regardless of the controls above."</p>
            </section>
            <footer><p>"Independent community archive · All times UTC"</p><p>"Source: "<a href="https://account.monstersandmemories.com/metrics">"Monsters & Memories public metrics"</a></p></footer>
        </main>
    }
}
