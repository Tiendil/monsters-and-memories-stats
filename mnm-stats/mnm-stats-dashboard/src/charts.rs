//! Plotters SVG rendering. Source labels and exact values are rendered by Leptos.
use crate::analysis::{Alignment, Metric, Plot};
use chrono::{Datelike, NaiveDate};
use plotters::coord::{
    combinators::WithKeyPoints,
    ranged1d::{DefaultFormatting, KeyPointHint},
    types::RangedCoordf64,
};
use plotters::prelude::*;

// Plotters 0.3.7 does not forward f64's formatter through WithKeyPoints.
// Keep its coordinate mapping and ticks; labels are supplied by this renderer.
struct TimeAxis(WithKeyPoints<RangedCoordf64>);
impl Ranged for TimeAxis {
    type ValueType = f64;
    type FormatOption = DefaultFormatting;
    fn range(&self) -> std::ops::Range<f64> {
        self.0.range()
    }
    fn map(&self, value: &f64, limit: (i32, i32)) -> i32 {
        self.0.map(value, limit)
    }
    fn key_points<H: KeyPointHint>(&self, hint: H) -> Vec<f64> {
        self.0.key_points(hint)
    }
}

pub fn color(index: usize) -> RGBColor {
    const COLORS: [RGBColor; 6] = [
        RGBColor(139, 217, 198),
        RGBColor(255, 199, 120),
        RGBColor(160, 185, 255),
        RGBColor(239, 157, 193),
        RGBColor(203, 220, 136),
        RGBColor(144, 211, 242),
    ];
    if let Some(color) = COLORS.get(index) {
        return *color;
    }
    let (r, g, b) = HSLColor((index as f64 * 0.61803398875).fract(), 0.65, 0.7).rgb();
    RGBColor(r, g, b)
}

pub fn css_color(index: usize) -> String {
    let RGBColor(r, g, b) = color(index);
    format!("rgb({r}, {g}, {b})")
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

pub fn svg(plot: &Plot, metric: &Metric) -> Result<String, String> {
    let mut output = String::new();
    {
        let root = SVGBackend::with_string(&mut output, (640, 280)).into_drawing_area();
        let maximum = plot
            .series
            .iter()
            .flat_map(|s| &s.points)
            .filter_map(|p| p.value)
            .map(|v| v.number())
            .fold(0.0_f64, f64::max);
        let mut chart = ChartBuilder::on(&root)
            .margin(12)
            .x_label_area_size(52)
            .y_label_area_size(64)
            .build_cartesian_2d(
                TimeAxis((plot.x_bounds.0..plot.x_bounds.1).with_key_points(time_ticks(plot))),
                0.0..(maximum * 1.12).max(1.0),
            )
            .map_err(|e| e.to_string())?;
        chart
            .configure_mesh()
            .x_labels(4)
            .y_labels(5)
            .x_label_formatter(&|x| {
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
            .y_label_formatter(&|y| {
                if metric.is_ratio() {
                    format!("{y:.0}%")
                } else {
                    format!("{y:.0}")
                }
            })
            .label_style(
                ("sans-serif", 14)
                    .into_font()
                    .color(&RGBColor(185, 201, 213)),
            )
            .axis_style(RGBColor(102, 123, 139))
            .bold_line_style(RGBColor(59, 76, 88))
            .light_line_style(TRANSPARENT)
            .draw()
            .map_err(|e| e.to_string())?;
        for (index, series) in plot.series.iter().enumerate() {
            let color = color(index);
            for segment in series.segments() {
                chart
                    .draw_series(LineSeries::new(
                        segment.iter().copied(),
                        color.stroke_width(2),
                    ))
                    .map_err(|e| e.to_string())?;
                // Singleton observations remain visible even when every interval is a gap.
                if series.points.len() <= 200 || segment.len() == 1 {
                    chart
                        .draw_series(segment.iter().map(|p| Circle::new(*p, 3, color.filled())))
                        .map_err(|e| e.to_string())?;
                }
            }
        }
        root.present().map_err(|e| e.to_string())?;
    }
    Ok(output)
}
