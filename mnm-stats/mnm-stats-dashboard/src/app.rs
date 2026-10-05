use crate::{
    analysis::*,
    charts,
    charts::browser::{InteractiveHeatmap, InteractivePlot},
};
use chrono::{DateTime, Utc};
use leptos::{ev, prelude::*};
use mnm_stats_model::History;
use std::{
    collections::BTreeMap,
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
enum Section {
    Overview,
    Population,
    Relationships,
}
impl Section {
    const ALL: [Self; 3] = [Self::Overview, Self::Population, Self::Relationships];
    fn label(self) -> &'static str {
        match self {
            Self::Overview => "Overview",
            Self::Population => "Player activity",
            Self::Relationships => "Relationships",
        }
    }
    fn key(self) -> &'static str {
        match self {
            Self::Overview => "overview",
            Self::Population => "population",
            Self::Relationships => "relationships",
        }
    }
}
fn readable(time: DateTime<Utc>) -> String {
    time.format("%d %b %Y, %H:%M UTC").to_string()
}

#[component]
fn Summary(history: Arc<History>, section: RwSignal<Section>) -> impl IntoView {
    let latest = history.snapshots().last();
    view! {
        <section class="now-summary" aria-labelledby="now-heading">
        <h2 class="summary-heading" id="now-heading">"Now"</h2>
        <div class="headline-grid">{[Metric::Online, Metric::Daily, Metric::Monthly, Metric::Subscriptions].into_iter().map(|metric| {
            let key = metric.key();
            let label = metric.title();
            let link_label = format!("{label}: show chart");
            let value = latest.and_then(|s| metric.value(s, &Scope::All)).map_or_else(|| "Not available".into(), |v| match v { MetricValue::Count(n) => grouped_count(n), _ => unreachable!() });
            let target = format!("chart-{key}");
            view! { <article class="headline" data-summary=key>
                <h3><a class="text-action" href=format!("#{target}") aria-label=link_label on:click=move |_| {
                    section.set(Section::Overview);
                    let target = target.clone();
                    request_animation_frame(move || {
                        if let Some(element) = document().get_element_by_id(&target).and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok()) {
                            let _ = element.focus();
                            element.scroll_into_view();
                        }
                    });
                }>{label}</a></h3><p class="headline-value">{value}</p>
            </article> }
        }).collect_view()}</div>
        </section>
    }
}

fn close_menu(node: NodeRef<leptos::html::Details>, id: &str) {
    if let Some(element) = node.get() {
        let _ = element.remove_attribute("open");
    }
    if let Some(toggle) = document()
        .get_element_by_id(id)
        .and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok())
    {
        let _ = toggle.focus();
    }
}

fn close_menu_on_focus_out(node: NodeRef<leptos::html::Details>, event: web_sys::FocusEvent) {
    if let (Some(element), Some(target)) = (
        node.get(),
        event.related_target().and_then(|t| t.dyn_into::<web_sys::Node>().ok()),
    ) && !element.contains(Some(&target))
        // Pressing a label can focus an outside ancestor before activating its
        // checkbox. Hiding the menu during that native activation crashes Chrome.
        // Outside clicks still close it through the separate click handler.
        && !element.matches(":active").unwrap_or(false)
    {
        let _ = element.remove_attribute("open");
    }
}

#[component]
fn DateMenu(
    id: &'static str,
    node: NodeRef<leptos::html::Details>,
    label: Signal<String>,
    value: Signal<String>,
    children: Children,
) -> impl IntoView {
    let outside = window_event_listener(ev::click, move |event| {
        // A Remove button may already be detached by its click handler. The
        // event path still identifies the menu where that click originated.
        if let Some(element) = node.get()
            && !event.composed_path().includes(element.as_ref(), 0)
        {
            let _ = element.remove_attribute("open");
        }
    });
    on_cleanup(move || outside.remove());
    view! {
        <details class="date-menu" node_ref=node on:keydown=move |event| {
            if event.key() == "Escape" { event.prevent_default(); close_menu(node, id); }
        } on:focusout=move |event| close_menu_on_focus_out(node, event)>
            <summary id=id data-value=move || value.get() aria-labelledby=format!("{id}-label {id}-selection")>
                <svg class="action-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true"><rect x="3" y="5" width="18" height="16" rx="2"/><path d="M7 2v6M17 2v6M3 11h18"/></svg>
                <span id=format!("{id}-selection")>{move || label.get()}</span><span aria-hidden="true">"⌄"</span>
            </summary>
            <div class="date-menu-panel" id=format!("{id}-options")>{children()}</div>
        </details>
    }
}

#[component]
fn DateFields(
    prefix: &'static str,
    start: RwSignal<String>,
    end: RwSignal<String>,
    error: RwSignal<Option<String>>,
) -> impl IntoView {
    view! {
        <div class="date-fields">
            <label>"From (UTC)"<input id=format!("{prefix}-start") type="date" min="0001-01-01" max="9998-12-31" prop:value=move || start.get() aria-describedby=format!("{prefix}-error") aria-invalid=move || error.get().is_some().to_string() on:input=move |ev| start.set(event_target_value(&ev))/></label>
            <label>"To (UTC)"<input id=format!("{prefix}-end") type="date" min="0001-01-01" max="9998-12-31" prop:value=move || end.get() aria-describedby=format!("{prefix}-error") aria-invalid=move || error.get().is_some().to_string() on:input=move |ev| end.set(event_target_value(&ev))/></label>
        </div>
        <p id=format!("{prefix}-error") role="alert">{move || error.get()}</p>
    }
}

#[component]
fn DateControls(
    history: Arc<History>,
    range: RwSignal<TimeRange>,
    now: RwSignal<DateTime<Utc>>,
    mode: RwSignal<ComparisonMode>,
    matching: RwSignal<DateMatching>,
    custom: RwSignal<Vec<Period>>,
    comparison: Memo<Result<Comparison, String>>,
) -> impl IntoView {
    let primary_node = NodeRef::<leptos::html::Details>::new();
    let comparison_node = NodeRef::<leptos::html::Details>::new();
    let editing_primary = RwSignal::new(false);
    let start = RwSignal::new(String::new());
    let end = RwSignal::new(String::new());
    let error = RwSignal::new(None::<String>);
    let compare_start = RwSignal::new(String::new());
    let compare_end = RwSignal::new(String::new());
    let compare_error = RwSignal::new(None::<String>);
    view! {
        <div class="date-control">
            <div class="date-controls">
                <div class="date-field">
                <span id="time-range-label" class="control-label">"Time range"</span>
                <DateMenu id="time-range" node=primary_node label=Signal::derive(move || range.get().label()) value=Signal::derive(move || range.get().key().into())>
                    <div class="menu-choices" role="group" aria-label="Time range presets">
                        {TimeRange::ALL.into_iter().map(|item| view! {
                            <button class="menu-choice" data-range=item.key() aria-pressed=move || (range.get() == item).to_string() on:click=move |_| { range.set(item); editing_primary.set(false); close_menu(primary_node, "time-range"); }>{item.label()}</button>
                        }).collect_view()}
                        <button class="menu-choice" id="custom-range" aria-pressed=move || matches!(range.get(), TimeRange::Custom { .. }).to_string() on:click=move |_| {
                            let (first, last) = range.get().bounds(&history, now.get());
                            start.set(first.format("%Y-%m-%d").to_string()); end.set(last.format("%Y-%m-%d").to_string()); error.set(None); editing_primary.set(true);
                        }>"Custom range"</button>
                    </div>
                    <Show when=move || editing_primary.get()>
                        <div class="date-form">
                            <DateFields prefix="range" start end error/>
                            <button id="apply-range" on:click=move |_| match TimeRange::custom(&start.get(), &end.get()) {
                                Ok(selected) => { range.set(selected); error.set(None); editing_primary.set(false); close_menu(primary_node, "time-range"); }
                                Err(message) => error.set(Some(message)),
                            }>"Apply range"</button>
                        </div>
                    </Show>
                    <div class="menu-divider"><button class="menu-choice" id="toggle-comparison" on:click=move |_| {
                        mode.set(if mode.get() == ComparisonMode::Disabled { ComparisonMode::Previous } else { ComparisonMode::Disabled }); close_menu(primary_node, "time-range");
                    }>{move || if mode.get() == ComparisonMode::Disabled { "Enable comparison" } else { "Disable comparison" }}</button></div>
                </DateMenu>
                </div>
                <div class="date-field comparison-control">
                <span id="comparison-mode-label" class="control-label">"Comparison"</span>
                <DateMenu id="comparison-mode" node=comparison_node label=Signal::derive(move || {
                    match mode.get() {
                        ComparisonMode::Disabled => "Compare".into(),
                        ComparisonMode::Custom if custom.get().len() > 1 => format!("{} custom periods", custom.get().len()),
                        selected => selected.label().into(),
                    }
                }) value=Signal::derive(move || mode.get().key().into())>
                    <div class="menu-choices" role="group" aria-label="Comparison period">
                        {ComparisonMode::ALL.into_iter().map(|item| view! {
                            <button class="menu-choice" data-comparison=item.key() aria-pressed=move || (mode.get() == item).to_string() on:click=move |_| {
                                mode.set(item); compare_error.set(None);
                                if item != ComparisonMode::Custom { close_menu(comparison_node, "comparison-mode"); }
                            }>{item.label()}</button>
                        }).collect_view()}
                    </div>
                    <Show when=move || mode.get() == ComparisonMode::Custom>
                        <div class="date-form">
                            <DateFields prefix="compare" start=compare_start end=compare_end error=compare_error/>
                            <button id="add-period" on:click=move |_| match Period::custom(&compare_start.get(), &compare_end.get()) {
                                Ok(period) => { custom.update(|selected| { if !selected.contains(&period) { selected.push(period); } }); compare_error.set(None); }
                                Err(message) => compare_error.set(Some(message)),
                            }>"Add period"</button>
                            <ul class="selected-periods">{move || custom.get().into_iter().enumerate().map(|(i, period)| view! {
                                <li><span>{period.label()}</span><button class="secondary remove-period" aria-label=format!("Remove {}", period.label()) on:click=move |_| {
                                    if let Some(input) = document().get_element_by_id("compare-start").and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok()) { let _ = input.focus(); }
                                    custom.update(|selected| { selected.remove(i); });
                                }>"Remove"</button></li>
                            }).collect_view()}</ul>
                        </div>
                    </Show>
                    <div class="menu-divider" role="group" aria-label="Date matching">
                        <button class="menu-choice" id="match-weekday" aria-pressed=move || (matching.get() == DateMatching::Weekday).to_string() on:click=move |_| { matching.set(DateMatching::Weekday); close_menu(comparison_node, "comparison-mode"); }>"Match day of week"</button>
                        <button class="menu-choice" id="match-date" aria-pressed=move || (matching.get() == DateMatching::ExactDate).to_string() on:click=move |_| { matching.set(DateMatching::ExactDate); close_menu(comparison_node, "comparison-mode"); }>"Match exact date"</button>
                    </div>
                    <Show when=move || mode.get() != ComparisonMode::Disabled && (mode.get() != ComparisonMode::Custom || matching.get() == DateMatching::Weekday || custom.get().is_empty())>
                        <div class="resolved-periods" aria-label="Compared dates">{move || match comparison.get() {
                            Ok(Comparison::Periods(periods)) => periods.into_iter().skip(1).map(|p| view! { <p>{p.period.label()}</p> }).collect_view().into_any(),
                            Err(message) => view! { <p role="alert">{message}</p> }.into_any(),
                            _ => view! { <p>"Choose dates to add a comparison."</p> }.into_any(),
                        }}</div>
                    </Show>
                </DateMenu>
                </div>
            </div>
        </div>
    }
}

#[component]
fn ChartCard(
    history: Arc<History>,
    metric: Metric,
    scopes: RwSignal<Vec<Scope>>,
    range: RwSignal<TimeRange>,
    now: RwSignal<DateTime<Utc>>,
    comparison: Memo<Result<Comparison, String>>,
    zone_scopes: Option<RwSignal<Vec<ZoneScope>>>,
) -> impl IntoView {
    let key = metric.key();
    let title = metric.title();
    let is_share = metric == Metric::OnlineShare;
    let chart_metric = metric.clone();
    let styles = expect_context::<Arc<Mutex<charts::SeriesStyles>>>();
    let plotted = Memo::new(move |_| {
        comparison.get().and_then(|comparison| {
            let result = if let Some(selected) = zone_scopes {
                population_plot(
                    &history,
                    &selected.get(),
                    &scopes.get(),
                    range.get(),
                    now.get(),
                    &comparison,
                )
            } else {
                plot(
                    &history,
                    &metric,
                    &scopes.get(),
                    range.get(),
                    now.get(),
                    &comparison,
                )
            };
            result.map(|mut plot| {
                styles.lock().expect("series styles").assign(&mut plot);
                Arc::new(plot)
            })
        })
    });
    view! {
        <article class="chart-card" id=format!("chart-{key}") tabindex="-1" data-metric=key>
            <h3 class="chart-heading">{title.clone()}</h3>
            {is_share.then(|| view! { <p class="chart-note">"The “All Servers” option shows a separate line for every server. Uncheck it and select individual servers to see only their shares of all online players."</p> })}
            {move || match plotted.get() {
                Err(error) => view! { <p class="error" role="alert">{error}</p> }.into_any(),
                Ok(plot) => {
                    let has_values = plot.series.iter().any(|s| s.points.iter().any(|p| p.value.is_some()));
                    view! {
                        <ul class="legend" aria-label="Chart series">{plot.series.iter().map(|series| view! {
                            <li><svg class="swatch" viewBox="0 0 48 8" aria-hidden="true"><line x1="0" y1="4" x2="48" y2="4" stroke=charts::css_color(series.style)/></svg><span>{series.label.clone()}</span></li>
                        }).collect_view()}</ul>
                        <p class="chart-note">{plot.note.clone()}</p>
                        {if has_values {
                            view! { <InteractivePlot plot=plot.clone() metric=chart_metric.clone()/> }.into_any()
                        } else {
                            view! { <p class="empty-chart">"No available observations for this selection."</p> }.into_any()
                        }}
                    }.into_any()
                }
            }}
        </article>
    }
}

#[component]
fn ActivityCard(
    history: Arc<History>,
    scopes: RwSignal<Vec<Scope>>,
    range: RwSignal<TimeRange>,
    now: RwSignal<DateTime<Utc>>,
    comparison: Memo<Result<Comparison, String>>,
) -> impl IntoView {
    let maps = Memo::new(move |_| {
        comparison.get().and_then(|comparison| {
            activity_heatmaps(&history, &scopes.get(), range.get(), now.get(), &comparison)
        })
    });
    view! {
        <article class="chart-card" id="chart-activity-heatmap" data-metric="activity-heatmap" tabindex="-1">
            <h3 class="chart-heading">"Activity heatmap"</h3>
            <p class="chart-note">"Average observed online count by weekday and hour UTC. Coverage can be uneven; blank cells have no records."</p>
            {move || match maps.get() {
                Err(error) => view! { <p class="error" role="alert">{error}</p> }.into_any(),
                Ok(maps) if maps.is_empty() => view! { <p class="empty-chart">"Select servers and a period to show activity."</p> }.into_any(),
                Ok(maps) => {
                    let maximum = heatmap_maximum(&maps);
                    maps.into_iter().map(|map| {
                        let label = map.label.clone();
                        let available = map.cells.iter().flatten().any(|cell| cell.samples > 0);
                        view! {
                            <section class="heatmap-panel" aria-label=label.clone()>
                                <h4 class="heatmap-label">{label.clone()}</h4>
                                {if available {
                                    view! { <InteractiveHeatmap map maximum/> }.into_any()
                                } else {
                                    view! { <p class="empty-chart">"No available observations for this selection."</p> }.into_any()
                                }}
                            </section>
                        }
                    }).collect_view().into_any()
                }
            }}
        </article>
    }
}

#[component]
fn CheckboxPicker(
    id: &'static str,
    label: &'static str,
    choices: Vec<(String, String)>,
    selected: Signal<Vec<String>>,
    summary: Signal<String>,
    on_toggle: Callback<String>,
) -> impl IntoView {
    let node = NodeRef::<leptos::html::Details>::new();
    let outside = window_event_listener(ev::click, move |event| {
        if let Some(element) = node.get()
            && !event.composed_path().includes(element.as_ref(), 0)
        {
            let _ = element.remove_attribute("open");
        }
    });
    on_cleanup(move || outside.remove());
    view! {
        <div class="selection-control">
            <span id=format!("{id}-label") class="control-label">{label}</span>
            <details class="selection-picker" node_ref=node on:keydown=move |event| {
                if event.key() == "Escape" { event.prevent_default(); close_menu(node, &format!("{id}-toggle")); }
            } on:focusout=move |event| close_menu_on_focus_out(node, event)>
                <summary id=format!("{id}-toggle") aria-labelledby=format!("{id}-label {id}-selection")><span class="selection-summary" id=format!("{id}-selection")>{move || summary.get()}</span></summary>
                <div class="selection-options" id=format!("{id}-options") role="group" aria-labelledby=format!("{id}-label")>
                    {choices.into_iter().map(|(value, label)| {
                        let checked = value.clone();
                        view! { <label class="checkbox"><input type="checkbox" value=value.clone() prop:checked=move || selected.get().contains(&checked) on:change=move |_| on_toggle.run(value.clone())/><span>{label}</span></label> }
                    }).collect_view()}
                </div>
            </details>
        </div>
    }
}

#[component]
fn ServerPicker(names: BTreeMap<String, String>, scopes: RwSignal<Vec<Scope>>) -> impl IntoView {
    let choices = std::iter::once(("all".into(), "All Servers".into()))
        .chain(
            names
                .iter()
                .map(|(id, name)| (format!("server:{id}"), display_name(id, name))),
        )
        .collect();
    let summary = Signal::derive(move || {
        let selected = scopes.get();
        match selected.as_slice() {
            [] => "Choose servers".into(),
            [scope] => scope.label(&names),
            _ if selected.contains(&Scope::All) => format!("All Servers + {}", selected.len() - 1),
            _ => format!("{} servers", selected.len()),
        }
    });
    let selected = Signal::derive(move || {
        scopes
            .get()
            .iter()
            .map(|scope| match scope {
                Scope::All => "all".into(),
                Scope::Server(id) => format!("server:{id}"),
            })
            .collect()
    });
    let on_toggle = Callback::new(move |value: String| {
        let scope = value
            .strip_prefix("server:")
            .map_or(Scope::All, |id| Scope::Server(id.into()));
        scopes.update(|selected| {
            if selected.contains(&scope) {
                selected.retain(|s| s != &scope);
            } else {
                selected.push(scope);
            }
        });
    });
    view! { <CheckboxPicker id="servers" label="Servers" choices selected summary on_toggle/> }
}

#[component]
fn ZonePicker(names: BTreeMap<String, String>, zones: RwSignal<Vec<ZoneScope>>) -> impl IntoView {
    let choices = std::iter::once(("all".into(), "All Zones".into()))
        .chain(
            names
                .iter()
                .map(|(id, name)| (format!("zone:{id}"), display_name(id, name))),
        )
        .collect();
    let summary = Signal::derive(move || {
        let selected = zones.get();
        match selected.as_slice() {
            [] => "Choose zones".into(),
            [zone] => zone.label(&names),
            _ if selected.contains(&ZoneScope::All) => {
                format!("All Zones + {}", selected.len() - 1)
            }
            _ => format!("{} zones", selected.len()),
        }
    });
    let selected = Signal::derive(move || {
        zones
            .get()
            .iter()
            .map(|zone| match zone {
                ZoneScope::All => "all".into(),
                ZoneScope::Zone(id) => format!("zone:{id}"),
            })
            .collect()
    });
    let on_toggle = Callback::new(move |value: String| {
        let zone = value
            .strip_prefix("zone:")
            .map_or(ZoneScope::All, |id| ZoneScope::Zone(id.into()));
        zones.update(|selected| {
            if selected.contains(&zone) {
                selected.retain(|z| z != &zone);
            } else {
                selected.push(zone);
            }
        });
    });
    view! { <CheckboxPicker id="zones" label="Starting zones" choices selected summary on_toggle/> }
}

#[component]
fn Correlations(
    history: Arc<History>,
    scopes: RwSignal<Vec<Scope>>,
    range: RwSignal<TimeRange>,
    now: RwSignal<DateTime<Utc>>,
) -> impl IntoView {
    let names = servers(&history);
    view! {
        <section class="correlations" aria-labelledby="correlations-heading">
            <h2 id="correlations-heading">"Correlations"</h2>
            <p>"Pearson's r uses the last jointly available observation per UTC day in the shared time range. At least three paired days and variation in both counts are needed."</p>
            {move || {
                let (start, end) = range.get().bounds(&history, now.get());
                scopes.get().into_iter().map(|scope| view! {
                    <section class="correlation-entity">
                        <h3 class="correlation-scope">{format!("{} · {} · {} – {}", scope.label(&names), range.get().label(), readable(start), readable(end))}</h3>
                        <div class="correlation-grid">{[(Metric::Daily, Metric::Monthly), (Metric::Daily, Metric::Subscriptions), (Metric::Monthly, Metric::Subscriptions)].into_iter().map(|(a, b)| {
                            let result = correlation(&history, &a, &b, &scope, start, end);
                            view! { <div class="correlation-value"><h4>{format!("{} / {}", a.title(), b.title())}</h4>
                                <p class="coefficient">{result.r.map_or_else(|| "not available".into(), |r| format!("r = {r:.3}"))}</p>
                                <p>{format!("{} paired UTC days", result.paired_days)}</p>
                            </div> }
                        }).collect_view()}</div>
                    </section>
                }).collect_view()
            }}
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
    let zone_scopes = RwSignal::new(vec![ZoneScope::All]);
    let utc_now =
        || DateTime::from_timestamp_millis(js_sys::Date::now() as i64).expect("browser timestamp");
    let now = RwSignal::new(utc_now());
    let timer = set_interval_with_handle(
        move || now.set(utc_now()),
        std::time::Duration::from_secs(60),
    )
    .expect("browser timer");
    on_cleanup(move || timer.clear());
    let scopes = RwSignal::new(vec![Scope::All]);
    let range = RwSignal::new(TimeRange::default());
    let mode = RwSignal::new(ComparisonMode::default());
    let matching = RwSignal::new(DateMatching::default());
    let custom = RwSignal::new(Vec::<Period>::new());
    let download_error = RwSignal::new(None::<String>);
    let comparison_history = history.clone();
    let comparison = Memo::new(move |_| {
        comparison_for(
            &comparison_history,
            range.get(),
            now.get(),
            mode.get(),
            matching.get(),
            &custom.get(),
        )
    });
    let download_history = history.clone();
    let empty_history = history.clone();
    let empty_range =
        Memo::new(move |_| latest_in_range(&empty_history, range.get(), now.get()).is_none());
    let comparing =
        Memo::new(move |_| mode.get() != ComparisonMode::Disabled || scopes.get().len() > 1);
    view! {
        <style>{include_str!(concat!(env!("OUT_DIR"), "/style.css"))}</style>
        <a class="skip-link" href="#content">"Skip to dashboard content"</a>
        <main>
            {matches!(env!("MNM_STATS_DEMO"), "1").then(|| view! {
                <p id="demo-notice" role="status">"Demo · synthetic data and server names."</p>
            })}
            <header class="page-header">
                <div><p class="eyebrow">"Made with love and curiosity by "<a href="https://tiendil.org" target="_blank" rel="noopener">"Tiendil"</a></p><h1>"Statistics for Monsters & Memories"</h1></div>
                <div class="header-actions">
                    <button class="secondary" id="download-history" on:click=move |_| download_error.set(download(&download_history).err().map(|_| "The history download could not be created. Please try again.".into()))>"Download JSON"</button>
                    <div class="community-actions">
                        <a class="button-link secondary" href=REPOSITORY_URL>
                            <svg class="action-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" focusable="false"><polygon points="12 3 14.8 8.7 21 9.6 16.5 14 17.6 20.2 12 17.3 6.4 20.2 7.5 14 3 9.6 9.2 8.7"/></svg>
                            "Star on GitHub"
                        </a>
                        <a class="button-link secondary feature-request" href=format!("{REPOSITORY_URL}/issues/new")>"Request a feature"</a>
                    </div>
                </div>
            </header>
            <p class="error" role="alert">{move || download_error.get()}</p>
            <section class="history-summary" aria-label="Statistics coverage">
                <p id="history-status">
                    {first.zip(latest).map_or_else(|| "No statistics collected yet".into_any(), |(first, last)| view! {
                        "Data from "<time id="first-collection" datetime=utc(first)>{first.format("%d %b %Y").to_string()}</time>" to "<time id="latest-collection" datetime=utc(last)>{readable(last)}</time>
                    }.into_any())}
                    " · "<span id="history-count">{format!("{} {}", grouped_count(count as u128), if count == 1 { "record" } else { "records" })}</span>" · collected roughly hourly from "<a href="https://account.monstersandmemories.com/metrics" target="_blank" rel="noopener">"M&M’s public statistics"</a>
                </p>
            </section>
            <Summary history=history.clone() section/>
            <section class="controls" aria-labelledby="controls-heading">
                <h2 id="controls-heading" class="visually-hidden">"Explore the archive"</h2>
                <div class="control-grid">
                    <ServerPicker names scopes/>
                    <DateControls history=history.clone() range now mode matching custom comparison/>
                </div>
            </section>
            <nav class="section-nav" aria-label="Dashboard sections">{Section::ALL.into_iter().map(|item| view! {
                <button id=format!("nav-{}", item.key()) aria-pressed=move || (section.get() == item).to_string() on:click=move |_| section.set(item)>{item.label()}</button>
            }).collect_view()}</nav>
            <section id="content" tabindex="-1" class:comparing=move || comparing.get() aria-label=move || section.get().label()>
                <Show when=move || scopes.get().is_empty()>
                    <div class="empty-servers" role="status"><p>"Select at least one server to show statistics."</p><button class="secondary" on:click=move |_| scopes.set(vec![Scope::All])>"Show All Servers"</button></div>
                </Show>
                <Show when=move || mode.get() == ComparisonMode::Disabled && !scopes.get().is_empty() && empty_range.get()>
                    <div class="empty-selection" role="status"><p>{if count == 0 { "No history yet. The first successful collection will appear in a future dashboard build." } else { "No observations in this interval. Choose All time to explore the available archive." }}</p>
                    {(count > 0).then(|| view! { <button class="secondary" on:click=move |_| range.set(TimeRange::All)>"Show All time"</button> })}</div>
                </Show>
                {move || {
                    let selected = section.get();
                    let metrics = match selected {
                        Section::Overview => vec![Metric::Online, Metric::Daily, Metric::Monthly, Metric::Subscriptions],
                        Section::Population => vec![Metric::StartingZones, Metric::OnlineShare],
                        Section::Relationships => vec![Metric::DailyMonthly, Metric::DailySubscriptions, Metric::MonthlySubscriptions],
                    };
                    let chart_history = history.clone();
                    let zone_options = zone_names.clone();
                    view! {
                        <h2 class="section-title">{match selected { Section::Overview => "Trends over time", Section::Population => "Player activity", Section::Relationships => "Ratios of reported counts" }}</h2>
                        {(selected == Section::Population).then(move || view! {
                            <div class="zone-control"><ZonePicker names=zone_options zones=zone_scopes/></div>
                            <Show when=move || zone_scopes.get().is_empty()>
                                <div class="empty-zones" role="status"><p>"Select at least one starting zone to show statistics."</p><button class="secondary" on:click=move |_| zone_scopes.set(vec![ZoneScope::All])>"Show All Zones"</button></div>
                            </Show>
                        })}
                        <div class="chart-grid" class:overview-chart=selected == Section::Overview class:population-chart=selected == Section::Population>{metrics.into_iter().map(|metric| {
                            let selected_zones = (metric == Metric::StartingZones).then_some(zone_scopes);
                            view! { <ChartCard history=chart_history.clone() metric scopes range now comparison zone_scopes=selected_zones/> }
                        }).collect_view()}
                        {(selected == Section::Population).then(|| view! { <ActivityCard history=history.clone() scopes range now comparison/> })}
                        </div>
                        {(selected == Section::Relationships).then(|| view! {
                            <p class="correlation-explanation">"Correlations below use the primary time range for each selected server."</p>
                            <Correlations history=history.clone() scopes range now/>
                        })}
                    }
                }}
            </section>
            <footer>
                <a href="https://plotly.com/javascript/">"Charts by Plotly"</a>
            </footer>
        </main>
    }
}
