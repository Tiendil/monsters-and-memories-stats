use crate::{
    analysis::*,
    charts,
    charts::browser::{InteractiveHeatmap, InteractivePlot},
    view_state::*,
};
use chrono::{DateTime, Utc};
use leptos::{ev, prelude::*};
use mnm_stats_model::History;
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};
use wasm_bindgen::JsCast;

const REPOSITORY_URL: &str = "https://github.com/Tiendil/monsters-and-memories-stats";

#[component]
fn ChartExplanation(metric: Metric) -> impl IntoView {
    let (formula, text) = match metric {
        Metric::Online => (None, "Number of players online at the same time."),
        Metric::Daily => (
            None,
            "Daily active players, reflecting day-to-day participation.",
        ),
        Metric::Monthly => (
            None,
            "Monthly active players, reflecting the size of the active community over a longer period.",
        ),
        Metric::Subscriptions => (None, "Active subscriptions across the game."),
        Metric::StartingZones | Metric::Zone(..) => (
            None,
            "Players in the starting areas. “All Zones” is the combined population of those areas.",
        ),
        Metric::OnlineShare => (
            Some("Server online / total online × 100%"),
            "Each server’s share of the total online population. The “All Servers” option shows a separate line for every server. Uncheck it and select individual servers to show only their shares.",
        ),
        Metric::DailyMonthly => (
            Some("DAU / MAU × 100%"),
            "Daily activity as a percentage of monthly activity, commonly called stickiness. Higher values suggest more frequent participation.",
        ),
        Metric::OnlineDaily | Metric::OnlineMonthly => (
            Some("Online / (DAU or MAU) × 100%"),
            "Players online as a percentage of the daily or monthly active audience.",
        ),
        _ => (
            Some("(DAU, MAU, or online) / global subscribers × 100%"),
            "Daily, monthly, or online activity relative to the game’s total subscriptions. Values above 100% mean the activity count exceeds the subscription count.",
        ),
    };
    view! {
        <p class="chart-note chart-explanation">
            {formula.map(|formula| view! { <span class="chart-formula">{formula}</span> })}
            {text}
        </p>
    }
}

fn utc(time: DateTime<Utc>) -> String {
    time.to_rfc3339_opts(chrono::SecondsFormat::AutoSi, true)
}

#[derive(Clone, Copy)]
struct ViewSignals {
    target: RwSignal<String>,
    scopes: RwSignal<Vec<Scope>>,
    range: RwSignal<TimeRange>,
    comparison: RwSignal<ComparisonMode>,
    matching: RwSignal<DateMatching>,
    periods: RwSignal<Vec<Period>>,
    zones: RwSignal<Vec<ZoneScope>>,
    online_metrics: RwSignal<Vec<Metric>>,
    subscriber_metrics: RwSignal<Vec<Metric>>,
    last_url_state: StoredValue<ViewState>,
}

impl ViewSignals {
    fn new(state: ViewState) -> Self {
        Self {
            target: RwSignal::new(state.target.clone()),
            scopes: RwSignal::new(state.scopes.clone()),
            range: RwSignal::new(state.range),
            comparison: RwSignal::new(state.comparison),
            matching: RwSignal::new(state.matching),
            periods: RwSignal::new(state.periods.clone()),
            zones: RwSignal::new(state.zones.clone()),
            online_metrics: RwSignal::new(state.online_metrics.clone()),
            subscriber_metrics: RwSignal::new(state.subscriber_metrics.clone()),
            last_url_state: StoredValue::new(state),
        }
    }

    fn snapshot(self) -> ViewState {
        ViewState {
            target: self.target.get(),
            scopes: self.scopes.get(),
            range: self.range.get(),
            comparison: self.comparison.get(),
            matching: self.matching.get(),
            periods: self.periods.get(),
            zones: self.zones.get(),
            online_metrics: self.online_metrics.get(),
            subscriber_metrics: self.subscriber_metrics.get(),
        }
    }

    fn restore(self) {
        let fragment = window().location().hash().unwrap_or_default();
        if fragment == "#content" {
            focus_target("content");
            return;
        }
        let state = ViewState::from_fragment(&fragment);
        let changed_target = self.target.get_untracked() != state.target;
        // Set the last URL state before notifying signals: restoring history must
        // never create a new history entry or discard the Forward stack.
        self.last_url_state.set_value(state.clone());
        macro_rules! restore {
            ($signal:ident, $value:ident) => {
                if self.$signal.get_untracked() != state.$value {
                    self.$signal.set(state.$value);
                }
            };
        }
        let target = state.target.clone();
        restore!(target, target);
        restore!(scopes, scopes);
        restore!(range, range);
        restore!(comparison, comparison);
        restore!(matching, matching);
        restore!(periods, periods);
        restore!(zones, zones);
        restore!(online_metrics, online_metrics);
        restore!(subscriber_metrics, subscriber_metrics);
        if changed_target {
            self.scroll_to_plot(target);
        }
    }

    fn navigate(self, target: &str, event: web_sys::MouseEvent) {
        if event.button() != 0
            || event.ctrl_key()
            || event.meta_key()
            || event.shift_key()
            || event.alt_key()
        {
            return;
        }
        event.prevent_default();
        self.target.set(target.to_owned());
        self.scroll_to_plot(target.to_owned());
    }

    fn scroll_to_plot(self, target: String) {
        if !target.starts_with("chart-") {
            return;
        }
        request_animation_frame(move || {
            if self.target.get_untracked() == target {
                focus_target(&target);
            }
        });
    }
}

fn focus_target(target: &str) {
    if let Some(element) = document()
        .get_element_by_id(target)
        .and_then(|element| element.dyn_into::<web_sys::HtmlElement>().ok())
    {
        let _ = element.focus();
        element.scroll_into_view();
    }
}

fn readable(time: DateTime<Utc>) -> String {
    time.format("%d %b %Y, %H:%M UTC").to_string()
}

#[component]
fn Summary(history: Arc<History>) -> impl IntoView {
    let navigation = expect_context::<ViewSignals>();
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
            let link_target = target.clone();
            view! { <article class="headline" data-summary=key>
                <h3><a class="text-action" href=move || navigation.snapshot().link(&link_target) on:click=move |event| navigation.navigate(&target, event) aria-label=link_label>{label}</a></h3><p class="headline-value">{value}</p>
            </article> }
        }).collect_view()}</div>
        </section>
    }
}

#[component]
fn ChartHeading(target: String, title: String) -> impl IntoView {
    let navigation = expect_context::<ViewSignals>();
    let link_target = target.clone();
    let link_label = format!("Link to {title}");
    view! {
        <div class="chart-header">
            <h3 class="chart-heading">{title}</h3>
            <a class="chart-permalink" href=move || navigation.snapshot().link(&link_target) on:click=move |event| navigation.navigate(&target, event) aria-label=link_label.clone() title=link_label>"#"</a>
        </div>
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
    metric_choices: Option<RwSignal<Vec<Metric>>>,
) -> impl IntoView {
    let (key, title) = if metric_choices.is_some() {
        if metric == Metric::OnlineDaily {
            ("online-presence".into(), "Online presence".into())
        } else {
            (
                "subscriber-activity".into(),
                "Activity relative to subscribers".into(),
            )
        }
    } else if metric == Metric::DailyMonthly {
        (metric.key(), "Daily participation".into())
    } else {
        (metric.key(), metric.title())
    };
    let explanation_metric = metric.clone();
    let picker_metric = metric.clone();
    let chart_metric = metric.clone();
    let zone_options = zone_scopes.map(|selected| (selected, zones(&history)));
    let styles = expect_context::<Arc<Mutex<charts::SeriesStyles>>>();
    let plotted = Memo::new(move |_| {
        comparison.get().and_then(|comparison| {
            let result = if let Some(selected) = metric_choices {
                engagement_plot(
                    &history,
                    &selected.get(),
                    &scopes.get(),
                    range.get(),
                    now.get(),
                    &comparison,
                )
            } else if let Some(selected) = zone_scopes {
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
        <article class="chart-card" id=format!("chart-{key}") tabindex="-1" data-metric=key.clone()>
            <ChartHeading target=format!("chart-{key}") title=title.clone()/>
            <ChartExplanation metric=explanation_metric/>
            {zone_options.map(|(selected, names)| view! {
                <ZonePicker names zones=selected/>
                <Show when=move || selected.get().is_empty()>
                    <div class="empty-zones" role="status"><p>"Select at least one starting zone to show statistics."</p><button class="secondary" on:click=move |_| selected.set(vec![ZoneScope::All])>"Show All Zones"</button></div>
                </Show>
            })}
            {metric_choices.map(|selected| {
                let online = picker_metric == Metric::OnlineDaily;
                view! {
                    <MetricPicker online selected/>
                    <Show when=move || selected.get().is_empty()>
                        <p class="chart-note">"Choose at least one metric."</p>
                        <button class="secondary" on:click=move |_| selected.set(if online { ONLINE_METRICS.to_vec() } else { SUBSCRIBER_METRICS.to_vec() })>"Restore default metrics"</button>
                    </Show>
                }
            })}
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
            <ChartHeading target="chart-activity-heatmap".into() title="Activity heatmap".into()/>
            <p class="chart-note chart-explanation">"Average online population by weekday and hour (UTC). Brighter colors indicate more players; blank cells have no records. Individual servers share a scale that includes unchecked servers. “All Servers” uses a separate scale so larger totals don’t hide differences between individual servers. Both scales cover the selected periods."</p>
            {move || match maps.get() {
                Err(error) => view! { <p class="error" role="alert">{error}</p> }.into_any(),
                Ok(maps) if maps.is_empty() => view! { <p class="empty-chart">"Select servers and a period to show activity."</p> }.into_any(),
                Ok(maps) => {
                    maps.into_iter().map(|map| {
                        let bounds = map.color_bounds.unwrap_or_default();
                        let label = map.label.clone();
                        let available = map.cells.iter().flatten().any(|cell| cell.samples > 0);
                        view! {
                            <section class="heatmap-panel" aria-label=label.clone()>
                                <h4 class="heatmap-label">{label.clone()}</h4>
                                {if available {
                                    view! { <InteractiveHeatmap map bounds/> }.into_any()
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
    choices: Memo<Vec<(String, String)>>,
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
                    <For each=move || choices.get() key=|(value, _)| value.clone() children=move |(value, label)| {
                        let checked = value.clone();
                        view! { <label class="checkbox"><input type="checkbox" value=value.clone() prop:checked=move || selected.get().contains(&checked) on:change=move |_| on_toggle.run(value.clone())/><span>{label}</span></label> }
                    }/>
                </div>
            </details>
        </div>
    }
}

#[component]
fn ServerPicker(names: BTreeMap<String, String>, scopes: RwSignal<Vec<Scope>>) -> impl IntoView {
    let choice_names = names.clone();
    let choices = Memo::new(move |_| {
        std::iter::once(("all".into(), "All Servers".into()))
            .chain(
                choice_names
                    .iter()
                    .map(|(id, name)| (format!("server:{id}"), display_name(id, name))),
            )
            .chain(scopes.get().into_iter().filter_map(|value| match value {
                Scope::Server(id) if !choice_names.contains_key(&id) => {
                    Some((format!("server:{id}"), format!("{id} (unavailable)")))
                }
                _ => None,
            }))
            .collect()
    });
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
    let choice_names = names.clone();
    let choices = Memo::new(move |_| {
        std::iter::once(("all".into(), "All Zones".into()))
            .chain(
                choice_names
                    .iter()
                    .map(|(id, name)| (format!("zone:{id}"), display_name(id, name))),
            )
            .chain(zones.get().into_iter().filter_map(|value| match value {
                ZoneScope::Zone(id) if !choice_names.contains_key(&id) => {
                    Some((format!("zone:{id}"), format!("{id} (unavailable)")))
                }
                _ => None,
            }))
            .collect()
    });
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
    view! { <div class="chart-control"><CheckboxPicker id="zones" label="Show zones" choices selected summary on_toggle/></div> }
}

#[component]
fn MetricPicker(online: bool, selected: RwSignal<Vec<Metric>>) -> impl IntoView {
    let options = if online {
        ONLINE_METRICS.to_vec()
    } else {
        SUBSCRIBER_METRICS.to_vec()
    };
    let choice_metrics = options.clone();
    let choices = Memo::new(move |_| {
        choice_metrics
            .iter()
            .map(|m| (m.key(), m.title()))
            .collect()
    });
    let summary = Signal::derive(move || match selected.get().as_slice() {
        [] => "Choose metrics".into(),
        [metric] => metric.title(),
        metrics => format!("{} metrics", metrics.len()),
    });
    let selected_keys = Signal::derive(move || selected.get().iter().map(Metric::key).collect());
    let on_toggle = Callback::new(move |key: String| {
        if let Some(metric) = options.iter().find(|m| m.key() == key) {
            selected.update(|current| {
                if current.contains(metric) {
                    current.retain(|m| m != metric);
                } else {
                    current.push(metric.clone());
                }
            });
        }
    });
    view! { <div class="chart-control"><CheckboxPicker id=if online { "online-metrics" } else { "subscriber-metrics" } label="Show metrics" choices selected=selected_keys summary on_toggle/></div> }
}

#[component]
pub fn App() -> impl IntoView {
    let history = Arc::new(crate::embedded_history());
    provide_context(Arc::new(Mutex::new(charts::SeriesStyles::default())));
    let initial = ViewState::from_fragment(&window().location().hash().unwrap_or_default());
    let navigation = ViewSignals::new(initial);
    provide_context(navigation);
    let section = Memo::new(move |_| {
        Section::from_fragment(&format!("#{}", navigation.target.get()))
            .unwrap_or(Section::Overview)
    });
    let hash_listener = window_event_listener_untyped("hashchange", move |_| navigation.restore());
    let history_listener = window_event_listener_untyped("popstate", move |_| navigation.restore());
    on_cleanup(move || {
        hash_listener.remove();
        history_listener.remove();
    });
    Effect::new(move |_| {
        let state = navigation.snapshot();
        if state != navigation.last_url_state.get_value() {
            if let Ok(history) = window().history() {
                let _ = history.push_state_with_url(
                    &wasm_bindgen::JsValue::NULL,
                    "",
                    Some(&state.fragment()),
                );
            }
            navigation.last_url_state.set_value(state);
        }
    });
    navigation.scroll_to_plot(navigation.target.get_untracked());
    let count = history.snapshots().len();
    let latest = history.snapshots().last().map(|s| s.observed_at);
    let first = history.snapshots().first().map(|s| s.observed_at);
    let names = servers(&history);
    let zone_scopes = navigation.zones;
    let online_metrics = navigation.online_metrics;
    let subscriber_metrics = navigation.subscriber_metrics;
    let utc_now =
        || DateTime::from_timestamp_millis(js_sys::Date::now() as i64).expect("browser timestamp");
    let now = RwSignal::new(utc_now());
    let timer = set_interval_with_handle(
        move || now.set(utc_now()),
        std::time::Duration::from_secs(60),
    )
    .expect("browser timer");
    on_cleanup(move || timer.clear());
    let scopes = navigation.scopes;
    let range = navigation.range;
    let mode = navigation.comparison;
    let matching = navigation.matching;
    let custom = navigation.periods;
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
    let empty_history = history.clone();
    let empty_range =
        Memo::new(move |_| latest_in_range(&empty_history, range.get(), now.get()).is_none());
    let comparing =
        Memo::new(move |_| mode.get() != ComparisonMode::Disabled || scopes.get().len() > 1);
    view! {
        <style>{include_str!(concat!(env!("OUT_DIR"), "/style.css"))}</style>
        <a class="skip-link" href="#content" on:click=move |event| { event.prevent_default(); focus_target("content"); }>"Skip to dashboard content"</a>
        <main>
            {matches!(env!("MNM_STATS_DEMO"), "1").then(|| view! {
                <p id="demo-notice" role="status">"Demo · synthetic data and server names."</p>
            })}
            <header class="page-header">
                <div><p class="eyebrow">"Made with love and curiosity by "<a href="https://tiendil.org" target="_blank" rel="noopener">"Tiendil"</a></p><h1>"Statistics for Monsters & Memories"</h1></div>
                <div class="header-actions">
                    <a class="button-link secondary" id="download-history" href="history.jsonl" download="history.jsonl">"Download JSONL"</a>
                    <div class="community-actions">
                        <a class="button-link secondary" href=REPOSITORY_URL>
                            <svg class="action-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" focusable="false"><polygon points="12 3 14.8 8.7 21 9.6 16.5 14 17.6 20.2 12 17.3 6.4 20.2 7.5 14 3 9.6 9.2 8.7"/></svg>
                            "Star on GitHub"
                        </a>
                        <a class="button-link secondary feature-request" href=format!("{REPOSITORY_URL}/issues/new")>"Request a feature"</a>
                    </div>
                </div>
            </header>
            <section class="history-summary" aria-label="Statistics coverage">
                <p id="history-status">
                    {first.zip(latest).map_or_else(|| "No statistics collected yet".into_any(), |(first, last)| view! {
                        "Data from "<time id="first-collection" datetime=utc(first)>{first.format("%d %b %Y").to_string()}</time>" to "<time id="latest-collection" datetime=utc(last)>{readable(last)}</time>
                    }.into_any())}
                    " · "<span id="history-count">{format!("{} {}", grouped_count(count as u128), if count == 1 { "record" } else { "records" })}</span>" · collected roughly hourly from "<a href="https://account.monstersandmemories.com/metrics" target="_blank" rel="noopener">"M&M’s public statistics"</a>
                </p>
            </section>
            <Summary history=history.clone()/>
            <section class="controls" aria-labelledby="controls-heading">
                <h2 id="controls-heading" class="visually-hidden">"Explore the archive"</h2>
                <div class="control-grid">
                    <ServerPicker names scopes/>
                    <DateControls history=history.clone() range now mode matching custom comparison/>
                </div>
            </section>
            <nav class="section-nav" aria-label="Dashboard sections">{Section::ALL.into_iter().map(|item| view! {
                <a class="button-link" id=format!("nav-{}", item.key()) href=move || navigation.snapshot().link(&item.fragment()[1..]) on:click=move |event| navigation.navigate(&item.fragment()[1..], event) aria-current=move || (section.get() == item).then_some("page")>{item.label()}</a>
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
                        Section::Relationships => vec![Metric::DailyMonthly, Metric::OnlineDaily, Metric::DailySubscriptions],
                    };
                    let chart_history = history.clone();
                    view! {
                        <h2 class="section-title" class:visually-hidden=selected == Section::Relationships>{match selected { Section::Overview => "Trends over time", Section::Population => "Player activity", Section::Relationships => "Engagement" }}</h2>
                        <div class="chart-grid" class:overview-chart=selected == Section::Overview class:population-chart=selected == Section::Population>{metrics.into_iter().map(|metric| {
                            let selected_zones = (metric == Metric::StartingZones).then_some(zone_scopes);
                            let metric_choices = match metric {
                                Metric::OnlineDaily => Some(online_metrics),
                                Metric::DailySubscriptions => Some(subscriber_metrics),
                                _ => None,
                            };
                            view! { <ChartCard history=chart_history.clone() metric scopes range now comparison zone_scopes=selected_zones metric_choices/> }
                        }).collect_view()}
                        {(selected == Section::Population).then(|| view! { <ActivityCard history=history.clone() scopes range now comparison/> })}
                        </div>
                    }
                }}
            </section>
            <footer>
                <a href="https://plotly.com/javascript/">"Charts by Plotly"</a>
            </footer>
        </main>
    }
}
