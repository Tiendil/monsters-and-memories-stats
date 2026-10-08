//! Rates from original observations, independent of display alignment and time zone.
use crate::analysis::{Metric, MetricValue, Plot, Scope};
use chrono::{DateTime, Duration, Utc};
use mnm_stats_model::{History, Snapshot};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RateMode {
    #[default]
    Value,
    Change,
    Trend,
}
impl RateMode {
    pub const ALL: [Self; 3] = [Self::Value, Self::Change, Self::Trend];
    pub fn key(self) -> &'static str {
        match self {
            Self::Value => "value",
            Self::Change => "change",
            Self::Trend => "trend",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Value => "Value",
            Self::Change => "Rate of change",
            Self::Trend => "Trend rate",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TrendWindow {
    Hours6,
    #[default]
    Hours24,
    Days7,
}
impl TrendWindow {
    pub const ALL: [Self; 3] = [Self::Hours6, Self::Hours24, Self::Days7];
    pub fn hours(self) -> i64 {
        match self {
            Self::Hours6 => 6,
            Self::Hours24 => 24,
            Self::Days7 => 168,
        }
    }
    pub fn key(self) -> &'static str {
        match self {
            Self::Hours6 => "6h",
            Self::Hours24 => "24h",
            Self::Days7 => "7d",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Hours6 => "Last 6 hours",
            Self::Hours24 => "Last 24 hours",
            Self::Days7 => "Last 7 days",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Aggregation {
    #[default]
    Snapshot,
    Average24,
}
impl Aggregation {
    pub const ALL: [Self; 2] = [Self::Snapshot, Self::Average24];
    pub fn key(self) -> &'static str {
        match self {
            Self::Snapshot => "snapshot",
            Self::Average24 => "24h",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Snapshot => "Snapshot",
            Self::Average24 => "24-hour average",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ChartView {
    pub aggregation: Aggregation,
    pub mode: RateMode,
    pub window: TrendWindow,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum LineChart {
    Online,
    Daily,
    Monthly,
    Subscriptions,
    StartingZones,
    OnlineShare,
    DailyMonthly,
    OnlinePresence,
    SubscriberActivity,
}
impl LineChart {
    pub const ALL: [Self; 9] = [
        Self::Online,
        Self::Daily,
        Self::Monthly,
        Self::Subscriptions,
        Self::StartingZones,
        Self::OnlineShare,
        Self::DailyMonthly,
        Self::OnlinePresence,
        Self::SubscriberActivity,
    ];
    pub fn key(self) -> &'static str {
        match self {
            Self::Online => "online",
            Self::Daily => "daily",
            Self::Monthly => "monthly",
            Self::Subscriptions => "subscriptions",
            Self::StartingZones => "starting-zones",
            Self::OnlineShare => "online-share",
            Self::DailyMonthly => "daily-monthly",
            Self::OnlinePresence => "online-presence",
            Self::SubscriberActivity => "subscriber-activity",
        }
    }
    pub fn supports_average(self) -> bool {
        matches!(
            self,
            Self::Online
                | Self::StartingZones
                | Self::OnlineShare
                | Self::OnlinePresence
                | Self::SubscriberActivity
        )
    }
    pub fn default_view(self) -> ChartView {
        ChartView {
            aggregation: Aggregation::Snapshot,
            mode: RateMode::Value,
            window: if matches!(self, Self::Online | Self::StartingZones) {
                TrendWindow::Hours6
            } else {
                TrendWindow::Hours24
            },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RateUnit {
    PlayersHour,
    CountDay,
    SubscriptionsDay,
    PointsDay,
}
impl RateUnit {
    pub fn for_metric(metric: &Metric) -> Self {
        match metric {
            Metric::Online | Metric::StartingZones | Metric::Zone(..) => Self::PlayersHour,
            Metric::Subscriptions => Self::SubscriptionsDay,
            _ if metric.is_ratio() => Self::PointsDay,
            _ => Self::CountDay,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::PlayersHour => "Players/hour",
            Self::CountDay => "Count/day",
            Self::SubscriptionsDay => "Subscriptions/day",
            Self::PointsDay => "Percentage points/day",
        }
    }
    pub fn suffix(self) -> &'static str {
        match self {
            Self::PlayersHour => "players/hour",
            Self::CountDay => "count/day",
            Self::SubscriptionsDay => "subscriptions/day",
            Self::PointsDay => "pp/day",
        }
    }
    fn seconds(self) -> f64 {
        if self == Self::PlayersHour {
            3600.0
        } else {
            86400.0
        }
    }
}

fn elapsed(start: DateTime<Utc>, end: DateTime<Utc>) -> f64 {
    (end - start).num_milliseconds() as f64 / 1000.0
}

// Subtract integer counts before conversion so a large baseline does not erase small changes.
fn difference(current: MetricValue, previous: MetricValue) -> f64 {
    match (current, previous) {
        (MetricValue::Count(a), MetricValue::Count(b)) => {
            if a >= b {
                (a - b) as f64
            } else {
                -((b - a) as f64)
            }
        }
        _ => current.number() - previous.number(),
    }
}

fn contributors(snapshot: &Snapshot, metric: &Metric, scope: &Scope) -> Vec<String> {
    if *metric == Metric::Subscriptions {
        return Vec::new();
    }
    let mut ids: Vec<_> = snapshot
        .servers
        .iter()
        .filter(|s| {
            *metric == Metric::OnlineShare
                || *scope == Scope::All
                || matches!(scope, Scope::Server(id) if id == &s.id)
        })
        // All Zones is a validated server total, even when its zone list changes.
        // Missing individual zones still break rates through their unavailable values.
        .map(|s| s.id.clone())
        .collect();
    ids.sort();
    ids
}

/// Integrate short linear intervals over the trailing day, excluding uncovered time.
/// Retain a boundary observation for clipping; never bridge missing values.
fn averages(
    history: &History,
    metric: &Metric,
    scope: &Scope,
    source: &[Option<MetricValue>],
) -> Vec<Option<MetricValue>> {
    let mut window = std::collections::VecDeque::new();
    let mut previous_contributors = Vec::new();
    history
        .snapshots()
        .iter()
        .zip(source)
        .map(|(snapshot, value)| {
            let ids = contributors(snapshot, metric, scope);
            if ids != previous_contributors {
                window.clear();
            }
            previous_contributors = ids;
            let cutoff = snapshot.observed_at - Duration::hours(24);
            window.push_back((snapshot.observed_at, value.map(MetricValue::number)));
            while window.get(1).is_some_and(|(at, _)| *at <= cutoff) {
                window.pop_front();
            }
            value.as_ref()?;
            let mut area = 0.0;
            let mut covered = Duration::zero();
            for (&(left, a), &(right, b)) in window.iter().zip(window.iter().skip(1)) {
                let (Some(a), Some(b)) = (a, b) else { continue };
                if right - left > Duration::hours(3) {
                    continue;
                }
                let start = left.max(cutoff);
                let duration = right - start;
                let at_start = a + (b - a) * elapsed(left, start) / elapsed(left, right);
                area += (at_start + b) / 2.0 * elapsed(start, right);
                covered += duration;
            }
            (covered >= Duration::hours(18)).then(|| MetricValue::Average {
                value: area / (covered.num_milliseconds() as f64 / 1000.0),
                percentage: metric.is_ratio(),
                covered,
                start: cutoff,
            })
        })
        .collect()
}

/// Evaluate at observation timestamps only. Missing metrics, changed contributors, and
/// long gaps separate runs; no fit crosses such a boundary or uses future samples.
pub fn values(
    history: &History,
    metric: &Metric,
    scope: &Scope,
    view: ChartView,
) -> Vec<Option<MetricValue>> {
    let snapshots = history.snapshots();
    let source: Vec<_> = snapshots.iter().map(|s| metric.value(s, scope)).collect();
    let source = if view.aggregation == Aggregation::Average24 && metric.supports_average() {
        averages(history, metric, scope, &source)
    } else {
        source
    };
    if view.mode == RateMode::Value {
        return source;
    }
    let unit = RateUnit::for_metric(metric);
    let mut result = vec![None; snapshots.len()];
    let mut run_start = 0;
    let mut window_start = 0;
    let mut previous_contributors = Vec::new();
    for (i, snapshot) in snapshots.iter().enumerate() {
        let ids = contributors(snapshot, metric, scope);
        if i == 0
            || source[i].is_none()
            || source[i - 1].is_none()
            || ids != previous_contributors
            || snapshot.observed_at - snapshots[i - 1].observed_at >= Duration::hours(24)
        {
            run_start = i;
        }
        previous_contributors = ids;
        let Some(current) = source[i] else { continue };
        if i == run_start {
            continue;
        }
        let (start, slope) = if view.mode == RateMode::Change {
            let start = snapshots[i - 1].observed_at;
            (
                start,
                difference(current, source[i - 1].unwrap()) / elapsed(start, snapshot.observed_at),
            )
        } else {
            window_start = window_start.max(run_start);
            while snapshots[window_start].observed_at
                < snapshot.observed_at - Duration::hours(view.window.hours())
            {
                window_start += 1;
            }
            let points = &snapshots[window_start..=i];
            let start = points[0].observed_at;
            if points.len() < (view.window.hours() as usize / 2).max(4)
                || snapshot.observed_at - start < Duration::hours(view.window.hours() / 2)
                || points.windows(2).any(|pair| {
                    pair[1].observed_at - pair[0].observed_at
                        > Duration::hours((view.window.hours() / 4).clamp(3, 24))
                })
            {
                continue;
            }
            let baseline = source[window_start].unwrap();
            let n = points.len() as f64;
            let mean_x = points
                .iter()
                .map(|p| elapsed(start, p.observed_at))
                .sum::<f64>()
                / n;
            let mean_y = source[window_start..=i]
                .iter()
                .map(|v| difference(v.unwrap(), baseline))
                .sum::<f64>()
                / n;
            let mut numerator = 0.0;
            let mut denominator = 0.0;
            for (point, value) in points.iter().zip(&source[window_start..=i]) {
                let x = elapsed(start, point.observed_at) - mean_x;
                numerator += x * (difference(value.unwrap(), baseline) - mean_y);
                denominator += x * x;
            }
            (start, numerator / denominator)
        };
        let value = slope * unit.seconds();
        if value.is_finite() {
            result[i] = Some(MetricValue::Rate { value, unit, start });
        }
    }
    result
}

/// Transform original-time values, then retain the existing comparison coordinates.
pub fn apply(plot: &mut Plot, history: &History, view: ChartView) {
    plot.view = view;
    if view.mode == RateMode::Value && view.aggregation == Aggregation::Snapshot {
        return;
    }
    let mut cache = BTreeMap::new();
    for series in &mut plot.series {
        let key = (
            series.metric.key(),
            match &series.scope {
                Scope::All => None,
                Scope::Server(id) => Some(id.clone()),
            },
        );
        let computed = cache
            .entry(key)
            .or_insert_with(|| values(history, &series.metric, &series.scope, view));
        for point in &mut series.points {
            let i = history
                .snapshots()
                .binary_search_by_key(&point.at, |s| s.observed_at)
                .expect("plot observation belongs to history");
            point.value = computed[i];
        }
    }
}
