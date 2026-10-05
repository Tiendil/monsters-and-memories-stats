//! Plotly figure construction from Rust-owned observations and presentation tokens.
use crate::{
    analysis::{ActivityHeatmap, Alignment, Connection, Metric, Plot},
    tokens,
};
use plotly::{
    Configuration, HeatMap, Layout, Plot as Figure, Scatter,
    common::{
        ColorBar, ColorScale, ColorScaleElement, DashType, Font, HoverInfo, Label, Line, Marker,
        Mode, Orientation, Title,
    },
    configuration::DisplayModeBar,
    layout::{Axis, AxisType, HoverMode, Margin},
};

/// Assign an encoding once per identity and retain it throughout an exploration session.
#[derive(Default)]
pub struct SeriesStyles(Vec<String>);
impl SeriesStyles {
    pub fn assign(&mut self, plot: &mut Plot) {
        for series in &mut plot.series {
            series.style = self
                .0
                .iter()
                .position(|key| *key == series.identity)
                .unwrap_or_else(|| {
                    self.0.push(series.identity.clone());
                    self.0.len() - 1
                });
        }
    }
}

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
        Alignment::Month => [0, 14, 28].map(|day| f64::from(day * 86400)).to_vec(),
        _ => {
            let (start, end) = plot.x_bounds;
            let target = (end - start) / 3.0;
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
            (0..=3)
                .map(|i| first + f64::from(i) * step)
                .filter(|x| *x <= end)
                .collect()
        }
    }
}

fn height(plot: &Plot) -> usize {
    let hover_height =
        plot.series.len() * tokens::T_CHART_HOVER_SERIES_MIN_HEIGHT.pixels() as usize;
    (tokens::T_CHART_VIEWPORT_MIN_HEIGHT.pixels() as usize).max(
        hover_height
            + tokens::T_CHART_VIEWPORT_MARGIN.pixels() as usize * 2
            + tokens::T_CHART_AXIS_X_LABEL_AREA.pixels() as usize,
    )
}

fn hover_text(point: &crate::analysis::Point, label: &str) -> String {
    point.value.map_or_else(String::new, |value| {
        format!(
            "<b>{} {}</b><br>{}",
            value.display(),
            escape(label),
            point.at.format("%d %b %Y, %H:%M UTC"),
        )
    })
}

pub fn render(plot: &Plot, metric: &Metric) -> Figure {
    let mut figure = Figure::new();
    let mut sparse_traces = Vec::new();
    let maximum = plot
        .series
        .iter()
        .flat_map(|s| &s.points)
        .filter_map(|p| p.value)
        .map(|v| v.number())
        .fold(0.0_f64, f64::max);
    for series in &plot.series {
        let mut x = Vec::new();
        let mut y = Vec::new();
        let mut text = Vec::new();
        let mut marker_sizes = Vec::new();
        let mut sparse_x = Vec::new();
        let mut sparse_y = Vec::new();
        let dense = series.points.len() > 200;
        let mut previous: Option<&crate::analysis::Point> = None;
        for (point_index, point) in series.points.iter().enumerate() {
            if previous.is_some_and(|p| {
                p.value.is_some()
                    && point.value.is_some()
                    && point.connection_from(p) != Some(Connection::Regular)
            }) {
                x.push(None);
                y.push(None);
                text.push(String::new());
                marker_sizes.push(0);
            }
            if let Some(p) = previous
                && point.connection_from(p) == Some(Connection::Sparse)
            {
                sparse_x.extend([Some(p.x), Some(point.x), None]);
                sparse_y.extend([
                    p.value.map(|v| v.number()),
                    point.value.map(|v| v.number()),
                    None,
                ]);
            }
            x.push(Some(point.x));
            y.push(point.value.map(|v| v.number()));
            text.push(hover_text(point, &series.label));
            // Dense lines omit markers except where an observation has no connected neighbor.
            let connected_before = previous.is_some_and(|p| point.connection_from(p).is_some());
            let connected_after = series
                .points
                .get(point_index + 1)
                .is_some_and(|p| p.connection_from(point).is_some());
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
        let color = css_color(series.style);
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
                        .dash(DashType::Solid)
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
        if !sparse_x.is_empty() {
            // Only the observation trace participates in hover, so endpoints
            // shared with these visual connections never produce duplicate labels.
            sparse_traces.push(
                Scatter::new(sparse_x, sparse_y)
                    .name(escape(&series.label))
                    .mode(Mode::Lines)
                    .connect_gaps(false)
                    .hover_info(HoverInfo::Skip)
                    .opacity(tokens::T_CHART_SERIES_GAP_OPACITY)
                    .show_legend(false)
                    .line(
                        Line::new()
                            .color(tokens::T_CHART_SERIES_GAP_COLOR)
                            .dash(DashType::Solid)
                            .width(tokens::T_CHART_SERIES_LINE_WIDTH.px())
                            .simplify(false),
                    ),
            );
        }
    }
    for trace in sparse_traces {
        figure.add_trace(trace);
    }
    let font = Font::new()
        .family(tokens::T_CHART_AXIS_LABEL_FONT_FAMILY_CSS)
        .size(tokens::T_CHART_AXIS_LABEL_FONT_SIZE.pixels() as usize)
        .color(tokens::T_CHART_AXIS_LABEL_COLOR);
    let axis = || {
        Axis::new()
            .type_(AxisType::Linear)
            .fixed_range(true)
            .auto_margin(true)
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
                    "%d %b<br>%H:%M"
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
    let mut x_axis = axis()
        .range(vec![
            plot.x_bounds.0 - x_padding,
            plot.x_bounds.1 + x_padding,
        ])
        .tick_values(ticks)
        .tick_text(labels)
        .show_spikes(false);
    if plot.alignment != Alignment::Utc {
        x_axis = x_axis.title(Title::with_text(plot.alignment.description()).font(font.clone()));
    }
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
            .x_axis(x_axis)
            .y_axis(
                axis()
                    .title(Title::with_text(metric.unit()).font(font.clone()))
                    .range(vec![
                        0.0,
                        if *metric == Metric::OnlineShare {
                            100.0
                        } else {
                            (maximum * 1.12).max(1.0)
                        },
                    ])
                    .n_ticks(5)
                    .tick_format(",.0f")
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

/// Render an observed weekday/hour mean without interpolation or hover on empty buckets.
pub fn render_heatmap(map: &ActivityHeatmap, maximum: f64) -> Figure {
    let weekdays = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
    let mut figure = Figure::new();
    let font = Font::new()
        .family(tokens::T_CHART_AXIS_LABEL_FONT_FAMILY_CSS)
        .size(tokens::T_CHART_AXIS_LABEL_FONT_SIZE.pixels() as usize)
        .color(tokens::T_CHART_AXIS_LABEL_COLOR);
    let values = map
        .cells
        .iter()
        .map(|row| row.iter().map(|cell| cell.mean()).collect::<Vec<_>>())
        .collect();
    let text = map
        .cells
        .iter()
        .enumerate()
        .map(|(day, row)| {
            row.iter()
                .enumerate()
                .map(|(hour, cell)| {
                    cell.mean().map_or_else(String::new, |mean| {
                        format!(
                            "<b>{mean:.2} mean online</b><br>{}<br>{} {hour:02}:00–{:02}:00 UTC<br>{} {} · sum {}",
                            escape(&map.label), weekdays[day], hour + 1, cell.samples,
                            if cell.samples == 1 { "record" } else { "records" }, cell.total
                        )
                    })
                })
                .collect::<Vec<_>>()
        })
        .collect();
    figure.add_trace(
        HeatMap::new(
            (0..24).collect::<Vec<_>>(),
            (0..7).collect::<Vec<_>>(),
            values,
        )
        .text_matrix(text)
        .hover_template("%{text}<extra></extra>")
        .hover_on_gaps(false)
        .connect_gaps(false)
        .zauto(false)
        .zmin(0.0)
        // A positive upper bound also keeps an all-zero selection visibly measured.
        .zmax(if maximum > 0.0 { maximum } else { 1.0 })
        .color_scale(ColorScale::Vector(vec![
            ColorScaleElement(0.0, tokens::T_CHART_HEATMAP_COLOR_LOW.into()),
            ColorScaleElement(1.0 / 3.0, tokens::T_CHART_HEATMAP_COLOR_MID_LOW.into()),
            ColorScaleElement(2.0 / 3.0, tokens::T_CHART_HEATMAP_COLOR_MID_HIGH.into()),
            ColorScaleElement(1.0, tokens::T_CHART_HEATMAP_COLOR_HIGH.into()),
        ]))
        .x_gap(tokens::T_CHART_HEATMAP_CELL_GAP.pixels())
        .y_gap(tokens::T_CHART_HEATMAP_CELL_GAP.pixels())
        .color_bar(
            ColorBar::new()
                .orientation(Orientation::Horizontal)
                .title(Title::with_text("Mean online").font(font.clone()))
                .tick_font(font.clone())
                .tick_format(",.3~g")
                .n_ticks(3)
                .thickness(tokens::T_CHART_HEATMAP_SCALE_THICKNESS.pixels() as usize)
                .outline_width(0)
                .y(1.05),
        )
        .hover_label(
            Label::new()
                .background_color(tokens::T_COLOR_SURFACE_CHART)
                .border_color(tokens::T_CHART_HEATMAP_COLOR_HIGH)
                .font(font.clone()),
        ),
    );
    let axis = || {
        Axis::new()
            .type_(AxisType::Linear)
            .fixed_range(true)
            .auto_margin(true)
            .zero_line(false)
            .show_grid(false)
            .tick_font(font.clone())
    };
    let margin = tokens::T_CHART_VIEWPORT_MARGIN.pixels() as usize;
    figure.set_layout(
        Layout::new()
            .auto_size(true)
            .height(tokens::T_CHART_HEATMAP_HEIGHT.pixels() as usize)
            .font(font.clone())
            .paper_background_color(tokens::T_COLOR_SURFACE_CHART)
            .plot_background_color(tokens::T_COLOR_SURFACE_CHART)
            .margin(
                Margin::new()
                    .left(margin + tokens::T_CHART_AXIS_Y_LABEL_AREA.pixels() as usize)
                    .right(margin)
                    .top(tokens::T_CHART_HEATMAP_SCALE_AREA.pixels() as usize)
                    .bottom(margin + tokens::T_CHART_AXIS_X_LABEL_AREA.pixels() as usize),
            )
            .x_axis(
                axis()
                    .range(vec![-0.5, 23.5])
                    .tick_values(vec![0.0, 6.0, 12.0, 18.0, 23.0])
                    .tick_text(["00", "06", "12", "18", "23"].map(str::to_owned).to_vec())
                    .title(Title::with_text("Hour (UTC)").font(font.clone())),
            )
            .y_axis(
                axis()
                    .range(vec![6.5, -0.5])
                    .tick_values((0..7).map(f64::from).collect())
                    .tick_text(weekdays.map(str::to_owned).to_vec()),
            )
            .hover_mode(HoverMode::Closest),
    );
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
        let label = format!(
            "{}; {}. {} Hover a point for its exact value and UTC timestamp, or use Download JSON for all recorded observations.",
            metric.title(),
            metric.unit(),
            metric.description()
        );
        view! { <PlotSurface figure=render(&plot, &metric) height=height(&plot) label/> }
    }

    #[component]
    pub fn InteractiveHeatmap(map: ActivityHeatmap, maximum: f64) -> impl IntoView {
        let label = format!(
            "Activity heatmap; {}. Mean observed online population by UTC weekday and hour. Hover for mean, sum and sample count; blank cells have no observations. Download JSON contains the original observations.",
            map.label
        );
        view! { <PlotSurface figure=render_heatmap(&map, maximum) height=tokens::T_CHART_HEATMAP_HEIGHT.pixels() as usize label/> }
    }

    #[component]
    fn PlotSurface(figure: Figure, height: usize, label: String) -> impl IntoView {
        let height = format!("{height}px");
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
        Effect::new(move |_| {
            let Some(element) = node.get() else {
                return;
            };
            let element: web_sys::HtmlElement = element.unchecked_into();
            mounted.set_value(Some(element.clone()));
            let figure = figure.to_js_object();
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
                    Err(_) => error.set(Some("Chart unavailable. Download the history or reload to retry the chart engine.".into())),
                }
            });
        });
        view! {
            <div class="interactive-plot">
                <div class="plot" role="region" tabindex="0" aria-label=label>
                    <div class="plot-surface" node_ref=node style:height=height></div>
                </div>
                <p class="error" role="alert">{move || error.get()}</p>
            </div>
        }
    }
}
