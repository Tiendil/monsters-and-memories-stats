//! Plotly figure construction from Rust-owned observations and presentation tokens.
use crate::{
    analysis::{ActivityHeatmap, Alignment, Connection, Metric, Plot},
    time::TimeZone,
    tokens,
};
use plotly::{
    Configuration, HeatMap, Layout, Plot as Figure, Scatter,
    common::{
        ColorBar, ColorScale, ColorScaleElement, DashType, Font, HoverInfo, Label, Line, Marker,
        Mode, Orientation, Side, Title,
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

/// Only the axis depends on width; observations and comparison alignment stay fixed.
#[derive(Clone, Copy)]
struct TimeAxis {
    bounds: (f64, f64),
    alignment: Alignment,
    zone: TimeZone,
}

#[derive(Clone, Copy)]
enum TickStep {
    Seconds(i64),
    Months(u32),
}

impl TimeAxis {
    fn from_plot(plot: &Plot) -> Self {
        Self {
            bounds: plot.x_bounds,
            alignment: plot.alignment,
            zone: plot.zone,
        }
    }

    fn ticks(self, width: f64) -> (Vec<f64>, Vec<String>) {
        let span = self.bounds.1 - self.bounds.0;
        let target = (width / tokens::T_CHART_AXIS_X_TICK_SPACING.px() + 2.0)
            .ceil()
            .clamp(3.0, 8.0) as usize;
        let mut best = (Vec::new(), Vec::new());
        let mut score = usize::MAX;
        let steps = [
            1, 5, 15, 30, 60, 300, 900, 1800, 3600, 7200, 10800, 21600, 43200, 86400, 172800,
            259200, 432000, 604800, 1209600,
        ]
        .into_iter()
        .map(TickStep::Seconds)
        .chain(
            [1, 2, 3, 6, 12, 24, 60, 120, 600, 1200, 6000, 12000]
                .into_iter()
                .map(TickStep::Months),
        );
        for step in steps {
            let approximate = match step {
                TickStep::Seconds(s) => s as f64,
                TickStep::Months(m) => f64::from(m) * 30.0 * 86400.0,
            };
            // Avoid generating fine-grained ticks over large archives.
            if span / approximate > 16.0 || approximate > span {
                continue;
            }
            let values = self.values(step);
            if values.len() < 2 {
                continue;
            }
            let labels = values
                .iter()
                .map(|x| self.label(*x, step))
                .collect::<Vec<_>>();
            let label_widths = labels
                .iter()
                .map(|s| {
                    s.split("<br>")
                        .map(|line| line.chars().count())
                        .max()
                        .unwrap_or(0) as f64
                        * tokens::T_CHART_AXIS_LABEL_FONT_SIZE.px()
                        * 0.65
                })
                .collect::<Vec<_>>();
            let fits = values
                .windows(2)
                .zip(label_widths.windows(2))
                .all(|(x, label)| {
                    (x[1] - x[0]) / span * width
                        >= (label[0] + label[1]) / 2.0 + tokens::T_CHART_AXIS_LABEL_FONT_SIZE.px()
                });
            let difference = values.len().abs_diff(target);
            if fits && difference < score {
                score = difference;
                best = (values, labels);
            }
        }
        if best.0.is_empty() {
            // Very short or unusually narrow ranges still need reference labels.
            let values = vec![self.bounds.0, self.bounds.1];
            let labels = values
                .iter()
                .map(|x| self.label(*x, TickStep::Seconds(1)))
                .collect();
            return (values, labels);
        }
        best
    }

    fn values(self, step: TickStep) -> Vec<f64> {
        use chrono::{DateTime, Datelike, Duration, Months, NaiveDate};
        let origin = NaiveDate::from_ymd_opt(2000, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        let start = if self.alignment == Alignment::Chronological {
            self.zone
                .at(DateTime::from_timestamp(self.bounds.0 as i64, 0).unwrap())
                .naive_local()
        } else {
            origin + Duration::seconds(self.bounds.0 as i64)
        };
        if self.alignment == Alignment::Elapsed {
            let seconds = match step {
                TickStep::Seconds(s) => s as f64,
                TickStep::Months(m) => f64::from(m) * 30.0 * 86400.0,
            };
            let first = (self.bounds.0 / seconds).ceil() * seconds;
            return (0..=16)
                .map(|i| first + f64::from(i) * seconds)
                .take_while(|x| *x <= self.bounds.1)
                .collect();
        }
        let first = match step {
            TickStep::Seconds(seconds) => {
                let wall = start.and_utc().timestamp();
                DateTime::from_timestamp(wall.div_euclid(seconds) * seconds, 0)
                    .unwrap()
                    .naive_utc()
            }
            TickStep::Months(months) => {
                let month = start.year() * 12 + start.month0() as i32;
                let rounded = month.div_euclid(months as i32) * months as i32;
                NaiveDate::from_ymd_opt(
                    rounded.div_euclid(12),
                    rounded.rem_euclid(12) as u32 + 1,
                    1,
                )
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap()
            }
        };
        let mut values = Vec::new();
        for i in 0..=17 {
            let local = match step {
                TickStep::Seconds(seconds) => {
                    first.checked_add_signed(Duration::seconds(seconds * i))
                }
                TickStep::Months(months) => {
                    first.checked_add_months(Months::new(months * i as u32))
                }
            };
            let Some(local) = local else {
                break;
            };
            let x = if self.alignment == Alignment::Chronological {
                let Some(time) = self.zone.resolve(local) else {
                    continue;
                };
                time.timestamp() as f64
            } else {
                (local - origin).num_seconds() as f64
            };
            if x > self.bounds.1 {
                break;
            }
            // A month comparison has day 1 through day 31, never a synthetic day 32.
            if x >= self.bounds.0
                && !(self.alignment == Alignment::Month && x >= 31.0 * 86400.0)
                && values.last().is_none_or(|last| *last < x)
            {
                values.push(x);
            }
        }
        values
    }

    fn label(self, x: f64, step: TickStep) -> String {
        if self.alignment == Alignment::Month && x % 86400.0 == 0.0 {
            return format!("{}", x as u32 / 86400 + 1);
        }
        if self.alignment == Alignment::Year && matches!(step, TickStep::Months(_)) {
            return self.alignment.tick(x).split_once(' ').unwrap().1.to_owned();
        }
        if self.alignment != Alignment::Chronological {
            return self.alignment.tick(x);
        }
        let format = match step {
            TickStep::Months(_) => "%b %Y",
            TickStep::Seconds(s) if s < 60 => "%d %b<br>%H:%M:%S",
            TickStep::Seconds(s) if s < 86400 => "%d %b<br>%H:%M",
            _ if self.bounds.1 - self.bounds.0 > 180.0 * 86400.0 => "%d %b<br>%Y",
            _ => "%d %b",
        };
        chrono::DateTime::from_timestamp(x as i64, 0)
            .map_or_else(String::new, |t| self.zone.format(t, format))
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

fn hover_text(point: &crate::analysis::Point, label: &str, zone: TimeZone) -> String {
    point.value.map_or_else(String::new, |value| {
        format!(
            "<b>{} {}</b><br>{}",
            value.display(),
            escape(label),
            zone.timestamp(point.at),
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
            text.push(hover_text(point, &series.label, plot.zone));
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
    let (ticks, labels) = TimeAxis::from_plot(plot).ticks(800.0);
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
        .tick_angle(0.0)
        .show_spikes(false);
    if plot.alignment != Alignment::Chronological {
        x_axis = x_axis
            .title(Title::with_text(plot.alignment.description(plot.zone)).font(font.clone()));
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
pub fn render_heatmap(map: &ActivityHeatmap, bounds: (f64, f64)) -> Figure {
    let weekdays = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
    let mut figure = Figure::new();
    // Constant selections need a nonzero span; all panels use the same fallback.
    let (minimum, maximum) = bounds;
    let (scale_minimum, scale_maximum) = if maximum > minimum {
        bounds
    } else if maximum > 0.0 {
        (maximum * 0.95, maximum * 1.05)
    } else {
        (0.0, 1.0)
    };
    let step = (scale_maximum - scale_minimum) / 2.0;
    // Precision follows the span so nearby fractional endpoints stay distinct.
    let precision = (1.0 - step.log10().floor()).max(2.0) as usize;
    let colors = [
        tokens::T_CHART_HEATMAP_COLOR_00,
        tokens::T_CHART_HEATMAP_COLOR_01,
        tokens::T_CHART_HEATMAP_COLOR_02,
        tokens::T_CHART_HEATMAP_COLOR_03,
        tokens::T_CHART_HEATMAP_COLOR_04,
        tokens::T_CHART_HEATMAP_COLOR_05,
        tokens::T_CHART_HEATMAP_COLOR_06,
        tokens::T_CHART_HEATMAP_COLOR_07,
        tokens::T_CHART_HEATMAP_COLOR_08,
        tokens::T_CHART_HEATMAP_COLOR_09,
    ];
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
                            "<b>{mean:.2} mean online</b><br>{}<br>{} {hour:02}:00–{:02}:00 {}<br>{} {} · sum {}",
                            escape(&map.label), weekdays[day], hour + 1, escape(map.zone.name()), cell.samples,
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
        .zmin(scale_minimum)
        .zmax(scale_maximum)
        .color_scale(ColorScale::Vector(
            colors
                .iter()
                .enumerate()
                .map(|(index, color)| {
                    ColorScaleElement(index as f64 / (colors.len() - 1) as f64, (*color).into())
                })
                .collect(),
        ))
        .x_gap(tokens::T_CHART_HEATMAP_CELL_GAP.pixels())
        .y_gap(tokens::T_CHART_HEATMAP_CELL_GAP.pixels())
        .color_bar(
            ColorBar::new()
                .orientation(Orientation::Horizontal)
                .title(
                    Title::with_text("Mean online")
                        .font(font.clone())
                        .side(Side::Top),
                )
                .tick_font(font.clone())
                .tick_format(format!(",.{precision}~f"))
                .tick_vals(vec![scale_minimum, scale_minimum + step, scale_maximum])
                .thickness(tokens::T_CHART_HEATMAP_SCALE_THICKNESS.pixels() as usize)
                .x_pad(tokens::T_CHART_HEATMAP_SCALE_PADDING_INLINE.pixels().into())
                .outline_width(0)
                .y(1.05),
        )
        .hover_label(
            Label::new()
                .background_color(tokens::T_COLOR_SURFACE_CHART)
                .border_color(tokens::T_CHART_HEATMAP_HOVER_BORDER_COLOR)
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
                    .show_line(true)
                    .line_color(tokens::T_CHART_AXIS_LINE_COLOR)
                    .tick_values(vec![0.0, 6.0, 12.0, 18.0, 23.0])
                    .tick_text(
                        ["00:00", "06:00", "12:00", "18:00", "23:00"]
                            .map(str::to_owned)
                            .to_vec(),
                    )
                    .title(
                        Title::with_text(format!("Time of day ({})", map.zone.short_label()))
                            .font(font.clone()),
                    ),
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
    use std::cell::{Cell, RefCell};
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
        #[wasm_bindgen(catch, js_namespace = Plotly)]
        async fn relayout(
            node: &web_sys::HtmlElement,
            update: &JsValue,
        ) -> Result<JsValue, JsValue>;

        type PlotNode;
        #[wasm_bindgen(method, js_name = on)]
        fn on(this: &PlotNode, event: &str, callback: &js_sys::Function);
        #[wasm_bindgen(method, js_name = removeListener)]
        fn remove_listener(this: &PlotNode, event: &str, callback: &js_sys::Function);
    }

    #[component]
    pub fn InteractivePlot(plot: Arc<Plot>, metric: Metric) -> impl IntoView {
        let label = format!(
            "{}; {}. {} Hover a point for its exact value and timestamp in {}, or use Download JSONL for all recorded observations.",
            metric.title(),
            metric.unit(),
            metric.description(),
            plot.zone.name()
        );
        view! { <PlotSurface figure=render(&plot, &metric) height=height(&plot) label time_axis=TimeAxis::from_plot(&plot)/> }
    }

    #[component]
    pub fn InteractiveHeatmap(map: ActivityHeatmap, bounds: (f64, f64)) -> impl IntoView {
        let label = format!(
            "Activity heatmap; {}. Average online population by day of the week and hour ({}). Hover for the average, total and number of records used. Blank cells mean no data. Download JSONL contains the original records.",
            map.label,
            map.zone.name()
        );
        view! { <PlotSurface figure=render_heatmap(&map, bounds) height=tokens::T_CHART_HEATMAP_HEIGHT.pixels() as usize label/> }
    }

    #[component]
    fn PlotSurface(
        figure: Figure,
        height: usize,
        label: String,
        #[prop(optional)] time_axis: Option<TimeAxis>,
    ) -> impl IntoView {
        let height = format!("{height}px");
        let node = NodeRef::<leptos::html::Div>::new();
        let error = RwSignal::new(None::<String>);
        let details = RwSignal::new(Vec::<(String, String)>::new());
        let hover = StoredValue::new_local(None::<Closure<dyn FnMut(JsValue)>>);
        let resize = StoredValue::new_local(None::<Closure<dyn FnMut()>>);
        let mounted = StoredValue::new_local(None::<web_sys::HtmlElement>);
        let disposed = Arc::new(AtomicBool::new(false));
        let cleanup = disposed.clone();
        on_cleanup(move || {
            cleanup.store(true, Ordering::Relaxed);
            if let Some(node) = mounted.get_value() {
                hover.with_value(|callback| {
                    if let Some(callback) = callback {
                        for event in ["plotly_hover", "plotly_click"] {
                            node.unchecked_ref::<PlotNode>()
                                .remove_listener(event, callback.as_ref().unchecked_ref());
                        }
                    }
                });
                resize.with_value(|callback| {
                    if let Some(callback) = callback {
                        node.unchecked_ref::<PlotNode>()
                            .remove_listener("plotly_afterplot", callback.as_ref().unchecked_ref());
                    }
                });
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
                        let callback = Closure::<dyn FnMut(JsValue)>::new(move |event| {
                            let Ok(points) = js_sys::Reflect::get(&event, &"points".into()) else {
                                return;
                            };
                            let rows = js_sys::Array::from(&points).iter().filter_map(|point| {
                                let text = js_sys::Reflect::get(&point, &"text".into()).ok()?.as_string()?;
                                let trace = js_sys::Reflect::get(&point, &"fullData".into()).ok()?;
                                let label = js_sys::Reflect::get(&trace, &"hoverlabel".into()).ok()?;
                                let color = js_sys::Reflect::get(&label, &"bordercolor".into()).ok()?.as_string()?;
                                Some((text, color))
                            }).collect();
                            // Reuse the exact, escaped Rust-generated hover text. Keeping the
                            // selection after unhover lets touch users scroll through details.
                            details.set(rows);
                        });
                        for event in ["plotly_hover", "plotly_click"] {
                            element.unchecked_ref::<PlotNode>().on(event, callback.as_ref().unchecked_ref());
                        }
                        hover.set_value(Some(callback));
                        if let Some(axis) = time_axis {
                            let target = element.clone();
                            let last = RefCell::new(None);
                            let last_width = Cell::new(0);
                            let callback = Closure::<dyn FnMut()>::new(move || {
                                if disposed.load(Ordering::Relaxed) { return; }
                                // Tick labels can change Plotly's automatic margins. Only an
                                // external width change may trigger another density update.
                                let container_width = target.client_width();
                                if container_width <= 0 || last_width.replace(container_width) == container_width { return; }
                                let width = js_sys::Reflect::get(&target, &"_fullLayout".into())
                                    .and_then(|layout| js_sys::Reflect::get(&layout, &"xaxis".into()))
                                    .and_then(|axis| js_sys::Reflect::get(&axis, &"_length".into()))
                                    .ok().and_then(|width| width.as_f64());
                                let Some(width) = width.filter(|w| *w > 0.0) else { return; };
                                let ticks = axis.ticks(width);
                                // relayout also emits afterplot. Only update when labels change.
                                if last.borrow().as_ref() == Some(&ticks) { return; }
                                let update = js_sys::JSON::parse(&serde_json::json!({
                                    "xaxis.tickvals": ticks.0, "xaxis.ticktext": ticks.1
                                }).to_string()).unwrap();
                                *last.borrow_mut() = Some(ticks);
                                let target = target.clone();
                                let disposed = disposed.clone();
                                leptos::task::spawn_local(async move {
                                    if disposed.load(Ordering::Relaxed) { return; }
                                    if relayout(&target, &update).await.is_err() && !disposed.load(Ordering::Relaxed) {
                                        error.set(Some("Chart unavailable. Reload to retry the chart engine.".into()));
                                    }
                                });
                            });
                            element.unchecked_ref::<PlotNode>().on("plotly_afterplot", callback.as_ref().unchecked_ref());
                            let _ = callback.as_ref().unchecked_ref::<js_sys::Function>().call0(&JsValue::NULL);
                            resize.set_value(Some(callback));
                        }
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
                <Show when=move || !details.get().is_empty()>
                    <div class="point-details" role="group" aria-label="Selected point details">
                        {move || details.get().into_iter().map(|(text, color)| view! {
                            <p class="point-detail" style:border-left-color=color inner_html=text></p>
                        }).collect_view()}
                    </div>
                </Show>
                <p class="error" role="alert">{move || error.get()}</p>
            </div>
        }
    }
}

#[cfg(test)]
mod tick_tests {
    use super::*;

    fn chronological(start: &str, end: &str, zone: TimeZone) -> TimeAxis {
        TimeAxis {
            bounds: (
                chrono::DateTime::parse_from_rfc3339(start)
                    .unwrap()
                    .timestamp() as f64,
                chrono::DateTime::parse_from_rfc3339(end)
                    .unwrap()
                    .timestamp() as f64,
            ),
            alignment: Alignment::Chronological,
            zone,
        }
    }

    #[test]
    fn a_week_has_two_day_phone_ticks_and_daily_desktop_ticks() {
        let axis = chronological(
            "2026-09-29T16:00:00Z",
            "2026-10-06T16:00:00Z",
            TimeZone::UTC,
        );
        let (phone, labels) = axis.ticks(286.0);
        assert_eq!(labels, ["30 Sep", "02 Oct", "04 Oct", "06 Oct"]);
        assert!(
            phone
                .windows(2)
                .all(|pair| pair[1] - pair[0] == 2.0 * 86400.0)
        );
        let (desktop, labels) = axis.ticks(1100.0);
        assert_eq!(
            labels,
            [
                "30 Sep", "01 Oct", "02 Oct", "03 Oct", "04 Oct", "05 Oct", "06 Oct"
            ]
        );
        assert!(desktop.windows(2).all(|pair| pair[1] - pair[0] == 86400.0));
    }

    #[test]
    fn calendar_ticks_use_local_midnight_across_dst_and_real_month_starts() {
        let zone = TimeZone::from_name("Europe/Berlin").unwrap();
        let axis = chronological("2026-10-22T12:00:00Z", "2026-10-29T12:00:00Z", zone);
        let (values, _) = axis.ticks(1100.0);
        assert_eq!(values.len(), 7);
        assert!(
            values
                .windows(2)
                .any(|pair| pair[1] - pair[0] == 25.0 * 3600.0)
        );
        for value in values {
            assert_eq!(
                zone.format(
                    chrono::DateTime::from_timestamp(value as i64, 0).unwrap(),
                    "%H:%M"
                ),
                "00:00"
            );
        }
        let axis = chronological(
            "2024-01-01T00:00:00Z",
            "2026-01-01T00:00:00Z",
            TimeZone::UTC,
        );
        let (values, labels) = axis.ticks(1100.0);
        assert!((6..=9).contains(&values.len()));
        assert!(labels.iter().all(|s| s.len() == 8));
        for value in values {
            assert_eq!(
                TimeZone::UTC.format(
                    chrono::DateTime::from_timestamp(value as i64, 0).unwrap(),
                    "%d %H:%M"
                ),
                "01 00:00"
            );
        }
    }

    #[test]
    fn comparison_labels_fit_the_range_and_remain_unique() {
        for (alignment, end) in [
            (Alignment::Month, 31.0 * 86400.0),
            (Alignment::Year, 366.0 * 86400.0 - 1.0),
            (Alignment::Elapsed, 7.0 * 86400.0),
        ] {
            let axis = TimeAxis {
                bounds: (0.0, end),
                alignment,
                zone: TimeZone::UTC,
            };
            for width in [250.0, 400.0, 800.0, 1200.0] {
                let (values, labels) = axis.ticks(width);
                assert!(values.len() >= 3, "{alignment:?} at {width}: {labels:?}");
                assert!(values.windows(2).all(|pair| pair[0] < pair[1]));
                assert!(values.iter().all(|x| *x >= 0.0 && *x <= end));
                assert_eq!(
                    labels
                        .iter()
                        .collect::<std::collections::HashSet<_>>()
                        .len(),
                    labels.len()
                );
                if alignment == Alignment::Month {
                    assert!(labels.iter().all(|s| !s.starts_with("32 ")));
                }
            }
        }
    }

    #[test]
    fn short_ranges_keep_distinct_clock_labels() {
        let axis = chronological(
            "2026-10-06T12:10:00Z",
            "2026-10-06T13:10:00Z",
            TimeZone::UTC,
        );
        let (phone, labels) = axis.ticks(286.0);
        assert!((3..=4).contains(&phone.len()));
        assert_eq!(
            labels
                .iter()
                .collect::<std::collections::HashSet<_>>()
                .len(),
            labels.len()
        );
        assert!(labels.iter().all(|s| s.contains("<br>")));
    }
}
