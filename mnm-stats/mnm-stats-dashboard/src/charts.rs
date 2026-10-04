//! Plotly figure construction from Rust-owned observations and presentation tokens.
use crate::{
    analysis::{Alignment, Metric, Plot},
    tokens,
};
use chrono::{Datelike, NaiveDate};
use plotly::{
    Configuration, Layout, Plot as Figure, Scatter,
    common::{Font, Label, Line, Marker, Mode},
    configuration::DisplayModeBar,
    layout::{Axis, AxisType, HoverMode, Margin},
};

pub fn css_color(index: usize) -> String {
    if let Some(color) = tokens::CHART_PALETTE.get(index) {
        return (*color).to_owned();
    }
    format!(
        "hsla({}, {}%, {}%, {})",
        (index as f64 * tokens::T_CHART_SERIES_FALLBACK_HUE_STEP).fract() * 360.0,
        tokens::T_CHART_SERIES_FALLBACK_SATURATION * 100.0,
        tokens::T_CHART_SERIES_FALLBACK_LIGHTNESS * 100.0,
        tokens::T_CHART_SERIES_FALLBACK_ALPHA
    )
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\n', " ")
}

fn time_ticks(plot: &Plot) -> Vec<f64> {
    match plot.alignment {
        Alignment::Month => [0, 7, 14, 21, 28]
            .map(|day| f64::from(day * 86400))
            .to_vec(),
        Alignment::Year => [1, 4, 7, 10]
            .map(|month| {
                f64::from(NaiveDate::from_ymd_opt(2000, month, 1).unwrap().ordinal0() * 86400)
            })
            .to_vec(),
        _ => {
            let (start, end) = plot.x_bounds;
            let target = (end - start) / 4.0;
            // Round ticks to seconds, minutes, hours or whole UTC days. Unix
            // numeric ticks alone can label arbitrary times such as 16:53.
            let step = [
                1, 5, 15, 30, 60, 300, 900, 1800, 3600, 10800, 21600, 43200, 86400, 172800, 604800,
                1209600, 2592000, 7776000, 31536000,
            ]
            .into_iter()
            .map(f64::from)
            .find(|step| *step >= target)
            .unwrap_or_else(|| (target / 31536000.0).ceil() * 31536000.0);
            let first = (start / step).ceil() * step;
            (0..=4)
                .map(|i| first + f64::from(i) * step)
                .filter(|x| *x <= end)
                .collect()
        }
    }
}

fn height(plot: &Plot) -> usize {
    (tokens::T_CHART_VIEWPORT_MIN_HEIGHT.pixels() as usize).max(
        plot.series.len() * tokens::T_CHART_HOVER_SERIES_MIN_HEIGHT.pixels() as usize
            + tokens::T_CHART_VIEWPORT_MARGIN.pixels() as usize * 2
            + tokens::T_CHART_AXIS_X_LABEL_AREA.pixels() as usize,
    )
}

pub fn render(plot: &Plot, metric: &Metric) -> Figure {
    let mut figure = Figure::new();
    let maximum = plot
        .series
        .iter()
        .flat_map(|s| &s.points)
        .filter_map(|p| p.value)
        .map(|v| v.number())
        .fold(0.0_f64, f64::max);
    for (index, series) in plot.series.iter().enumerate() {
        let mut x = Vec::new();
        let mut y = Vec::new();
        let mut text = Vec::new();
        let mut marker_sizes = Vec::new();
        let dense = series.points.len() > 200;
        let mut previous: Option<&crate::analysis::Point> = None;
        for (point_index, point) in series.points.iter().enumerate() {
            if previous.is_some_and(|p| point.has_gap_from(p)) {
                x.push(None);
                y.push(None);
                text.push(String::new());
                marker_sizes.push(0);
            }
            x.push(Some(point.x));
            y.push(point.value.map(|v| v.number()));
            text.push(point.value.map_or_else(String::new, |value| {
                format!(
                    "<b>{}. {}</b><br>{} UTC<br>{}",
                    index + 1,
                    escape(&series.label),
                    point
                        .at
                        .to_rfc3339_opts(chrono::SecondsFormat::AutoSi, true),
                    value.display()
                )
            }));
            // Dense lines omit markers except where an observation has no connected neighbor.
            let connected_before =
                previous.is_some_and(|p| p.value.is_some() && !point.has_gap_from(p));
            let connected_after = series
                .points
                .get(point_index + 1)
                .is_some_and(|p| p.value.is_some() && !p.has_gap_from(point));
            marker_sizes.push(
                if point.value.is_some() && (!dense || (!connected_before && !connected_after)) {
                    tokens::T_CHART_SERIES_POINT_RADIUS.pixels() as usize * 2
                } else {
                    0
                },
            );
            previous = Some(point);
        }
        let markers = marker_sizes.iter().any(|size| *size > 0);
        let color = css_color(index);
        figure.add_trace(
            Scatter::new(x, y)
                .name(escape(&series.label))
                .mode(if markers {
                    Mode::LinesMarkers
                } else {
                    Mode::Lines
                })
                .connect_gaps(false)
                .line(
                    Line::new()
                        .color(color.clone())
                        .width(tokens::T_CHART_SERIES_LINE_WIDTH.px())
                        .simplify(false),
                )
                .marker(Marker::new().color(color.clone()).size_array(marker_sizes))
                .text_array(text)
                .hover_template("%{text}<extra></extra>")
                .hover_label(
                    Label::new()
                        .align("left")
                        .background_color(tokens::T_COLOR_SURFACE_TOOLTIP)
                        .border_color(color)
                        .font(Font::new().color(tokens::T_COLOR_TEXT_PRIMARY)),
                ),
        );
    }
    let font = Font::new()
        .family(tokens::T_CHART_AXIS_LABEL_FONT_FAMILY_CSS)
        .size(tokens::T_CHART_AXIS_LABEL_FONT_SIZE.pixels() as usize)
        .color(tokens::T_CHART_AXIS_LABEL_COLOR);
    let axis = || {
        Axis::new()
            .type_(AxisType::Linear)
            .fixed_range(true)
            .zero_line(false)
            .show_line(true)
            .line_color(tokens::T_CHART_AXIS_LINE_COLOR)
            .grid_color(tokens::T_CHART_GRID_LINE_COLOR)
            .tick_font(font.clone())
    };
    let ticks = time_ticks(plot);
    let labels = ticks
        .iter()
        .map(|x| {
            if plot.alignment == Alignment::Utc {
                let span = plot.x_bounds.1 - plot.x_bounds.0;
                let format = if span > 180.0 * 86400.0 {
                    "%b %Y"
                } else if span > 2.0 * 86400.0 {
                    "%d %b"
                } else {
                    "%d %b %H:%M"
                };
                chrono::DateTime::from_timestamp(*x as i64, 0)
                    .map_or_else(String::new, |t| t.format(format).to_string())
            } else {
                plot.alignment.tick(*x)
            }
        })
        .collect();
    // Leave a small gutter around boundary observations so their markers and
    // native hover targets are inside the plotting area. Data filtering is unchanged.
    let x_padding = (plot.x_bounds.1 - plot.x_bounds.0) * tokens::T_CHART_AXIS_X_RANGE_PADDING;
    let margin = tokens::T_CHART_VIEWPORT_MARGIN.pixels() as usize;
    figure.set_layout(
        Layout::new()
            .auto_size(true)
            .height(height(plot))
            .show_legend(false)
            .font(font.clone())
            .paper_background_color(tokens::T_COLOR_SURFACE_CHART)
            .plot_background_color(tokens::T_COLOR_SURFACE_CHART)
            .margin(
                Margin::new()
                    .left(margin + tokens::T_CHART_AXIS_Y_LABEL_AREA.pixels() as usize)
                    .right(margin)
                    .top(margin)
                    .bottom(margin + tokens::T_CHART_AXIS_X_LABEL_AREA.pixels() as usize),
            )
            .x_axis(
                axis()
                    .range(vec![
                        plot.x_bounds.0 - x_padding,
                        plot.x_bounds.1 + x_padding,
                    ])
                    .tick_values(ticks)
                    .tick_text(labels)
                    .show_spikes(false),
            )
            .y_axis(
                axis()
                    .range(vec![0.0, (maximum * 1.12).max(1.0)])
                    .n_ticks(5)
                    .tick_format(".0f")
                    .tick_suffix(if metric.is_ratio() { "%" } else { "" }),
            )
            .hover_mode(HoverMode::X)
            .hover_distance(tokens::T_CHART_HOVER_HIT_RADIUS.pixels() as i32),
    );
    // Range selection is shared by all charts. Hover remains interactive.
    figure.set_configuration(
        Configuration::new()
            .responsive(true)
            .display_mode_bar(DisplayModeBar::False)
            .display_logo(false)
            .scroll_zoom(false),
    );
    figure
}

#[cfg(target_arch = "wasm32")]
pub mod browser {
    use super::*;
    use leptos::prelude::*;
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };
    use wasm_bindgen::{JsCast, prelude::*};

    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(catch, js_namespace = Plotly, js_name = newPlot)]
        async fn new_plot(
            node: &web_sys::HtmlElement,
            figure: &JsValue,
        ) -> Result<JsValue, JsValue>;
        #[wasm_bindgen(catch, js_namespace = Plotly)]
        fn purge(node: &web_sys::HtmlElement) -> Result<(), JsValue>;
    }

    #[component]
    pub fn InteractivePlot(plot: Arc<Plot>, metric: Metric) -> impl IntoView {
        let height = format!("{}px", height(&plot));
        let node = NodeRef::<leptos::html::Div>::new();
        let error = RwSignal::new(None::<String>);
        let mounted = StoredValue::new_local(None::<web_sys::HtmlElement>);
        let disposed = Arc::new(AtomicBool::new(false));
        let cleanup = disposed.clone();
        on_cleanup(move || {
            cleanup.store(true, Ordering::Relaxed);
            if let Some(node) = mounted.get_value() {
                let _ = purge(&node);
            }
        });
        let label = format!(
            "{}; {}. Hover a point or inspect exact values below.",
            metric.title(),
            metric.unit()
        );
        Effect::new(move |_| {
            let Some(element) = node.get() else {
                return;
            };
            let element: web_sys::HtmlElement = element.unchecked_into();
            mounted.set_value(Some(element.clone()));
            let figure = render(&plot, &metric).to_js_object();
            let disposed = disposed.clone();
            leptos::task::spawn_local(async move {
                let result = new_plot(&element, &figure).await;
                if disposed.load(Ordering::Relaxed) {
                    let _ = purge(&element);
                    return;
                }
                match result {
                    Ok(_) => {
                        let _ = element.set_attribute("data-ready", "true");
                    }
                    Err(reason) => error.set(Some(format!("Chart unavailable: {reason:?}"))),
                }
            });
        });
        view! {
            <div class="interactive-plot">
                <div class="plot" role="img" aria-label=label>
                    <div class="plot-surface" node_ref=node style:height=height></div>
                </div>
                <p class="error" role="alert">{move || error.get()}</p>
            </div>
        }
    }
}
