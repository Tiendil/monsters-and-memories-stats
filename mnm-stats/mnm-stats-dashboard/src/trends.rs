//! Player-facing rankings from complete calendar days, without filling collection gaps.
use crate::{
    analysis::{Scope, display_name, servers},
    time::TimeZone,
};
use chrono::{DateTime, Datelike, Duration, NaiveDate, Timelike, Utc};
use mnm_stats_model::History;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TrendPeriod {
    #[default]
    Week,
    Month,
    Year,
}

impl TrendPeriod {
    pub const ALL: [Self; 3] = [Self::Week, Self::Month, Self::Year];
    pub fn label(self) -> &'static str {
        match self {
            Self::Week => "Week",
            Self::Month => "Month",
            Self::Year => "Year",
        }
    }
    pub fn days(self) -> usize {
        match self {
            Self::Week => 7,
            Self::Month => 30,
            Self::Year => 365,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BusyGrouping {
    #[default]
    AllDays,
    Weekday,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TrendInterval {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub days: usize,
}

impl TrendInterval {
    pub fn label(&self, zone: TimeZone) -> String {
        format!(
            "{} – {}",
            zone.format(self.start, "%d %b %Y"),
            zone.format(self.end - Duration::nanoseconds(1), "%d %b %Y")
        )
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PopulationSummary {
    pub value: Option<f64>,
    pub days: usize,
    pub observations: usize,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PopulationChange {
    pub count: f64,
    pub percent: Option<f64>,
}

pub fn change(
    current: &PopulationSummary,
    previous: &PopulationSummary,
) -> Option<PopulationChange> {
    let count = current.value? - previous.value?;
    Some(PopulationChange {
        count,
        percent: previous
            .value
            .filter(|v| *v > 0.0)
            .map(|v| count / v * 100.0),
    })
}

#[derive(Clone, Debug, PartialEq)]
pub struct ServerTrend {
    pub id: String,
    pub name: String,
    pub current: PopulationSummary,
    pub previous: PopulationSummary,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BusyHours {
    pub server_id: String,
    pub server: String,
    pub start_hour: u32,
    pub weekday: Option<u32>,
    pub current: PopulationSummary,
    pub previous: PopulationSummary,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Trends {
    pub current: TrendInterval,
    pub previous: TrendInterval,
    pub servers: Vec<ServerTrend>,
    pub hours: Vec<BusyHours>,
    pub areas: Vec<ServerTrend>,
}

type DailySamples = BTreeMap<NaiveDate, Vec<(DateTime<Utc>, u128)>>;

fn median(mut values: Vec<f64>) -> f64 {
    values.sort_by(f64::total_cmp);
    let middle = values.len() / 2;
    if values.len().is_multiple_of(2) {
        values[middle - 1] / 2.0 + values[middle] / 2.0
    } else {
        values[middle]
    }
}

fn summarize(
    samples: &DailySamples,
    interval: &TrendInterval,
    zone: TimeZone,
    window: bool,
) -> PopulationSummary {
    let mut daily = Vec::new();
    let mut observations = 0;
    for (date, points) in samples {
        let qualifies = if window {
            points
                .iter()
                .map(|(time, _)| zone.at(*time).hour())
                .collect::<BTreeSet<_>>()
                .len()
                >= 2
        } else {
            let hours =
                (zone.midnight(date.succ_opt().unwrap()) - zone.midnight(*date)).num_hours();
            let midnight = zone.midnight(*date);
            let covered = points
                .iter()
                .map(|(time, _)| (*time - midnight).num_hours())
                .collect::<BTreeSet<_>>()
                .len();
            covered >= (hours as usize).div_ceil(2)
        };
        if qualifies {
            observations += points.len();
            daily.push(median(
                points.iter().map(|(_, value)| *value as f64).collect(),
            ));
        }
    }
    let days = daily.len();
    let required = interval.days.div_ceil(2).max(if window { 2 } else { 1 });
    PopulationSummary {
        value: (days >= required).then(|| median(daily)),
        days,
        observations,
    }
}

#[derive(Default)]
struct Samples {
    population: DailySamples,
    areas: DailySamples,
    windows: BTreeMap<(Option<u32>, u32), DailySamples>,
}

fn collect(
    history: &History,
    selected: &BTreeMap<String, String>,
    interval: &TrendInterval,
    zone: TimeZone,
    grouping: BusyGrouping,
) -> BTreeMap<String, Samples> {
    let mut output = BTreeMap::<String, Samples>::new();
    for snapshot in history
        .snapshots()
        .iter()
        .filter(|s| s.observed_at >= interval.start && s.observed_at < interval.end)
    {
        let local = zone.at(snapshot.observed_at);
        for server in snapshot
            .servers
            .iter()
            .filter(|s| selected.contains_key(&s.id))
        {
            let samples = output.entry(server.id.clone()).or_default();
            samples
                .population
                .entry(local.date_naive())
                .or_default()
                .push((snapshot.observed_at, server.online.into()));
            let weekday =
                (grouping == BusyGrouping::Weekday).then(|| local.weekday().num_days_from_monday());
            samples
                .windows
                .entry((weekday, local.hour() / 3 * 3))
                .or_default()
                .entry(local.date_naive())
                .or_default()
                .push((snapshot.observed_at, server.online.into()));
            if let Some(total) = server
                .starting_zones
                .iter()
                .try_fold(0_u128, |sum, area| sum.checked_add(area.online.into()))
            {
                samples
                    .areas
                    .entry(local.date_naive())
                    .or_default()
                    .push((snapshot.observed_at, total));
            }
        }
    }
    output
}

// Available values sort first, descending; callers provide stable identity tie breakers.
fn descending(left: Option<f64>, right: Option<f64>) -> std::cmp::Ordering {
    match (left, right) {
        (Some(a), Some(b)) => b.total_cmp(&a),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    }
}

pub fn rankings(
    history: &History,
    scopes: &[Scope],
    period: TrendPeriod,
    grouping: BusyGrouping,
    now: DateTime<Utc>,
    zone: TimeZone,
) -> Trends {
    let end_date = zone.at(now).date_naive();
    let start_date = end_date - Duration::days(period.days() as i64);
    let current = TrendInterval {
        start: zone.midnight(start_date),
        end: zone.midnight(end_date),
        days: period.days(),
    };
    let previous = TrendInterval {
        start: zone.midnight(start_date - Duration::days(period.days() as i64)),
        end: current.start,
        days: period.days(),
    };
    let selected: BTreeMap<_, _> = servers(history)
        .into_iter()
        .filter(|(id, _)| {
            scopes.contains(&Scope::All) || scopes.contains(&Scope::Server(id.clone()))
        })
        .collect();
    let mut recent = collect(history, &selected, &current, zone, grouping);
    let mut earlier = collect(history, &selected, &previous, zone, grouping);
    let mut result = Trends {
        current,
        previous,
        servers: Vec::new(),
        hours: Vec::new(),
        areas: Vec::new(),
    };
    for (id, name) in selected {
        let name = display_name(&id, &name);
        let recent = recent.remove(&id).unwrap_or_default();
        let earlier = earlier.remove(&id).unwrap_or_default();
        result.servers.push(ServerTrend {
            id: id.clone(),
            name: name.clone(),
            current: summarize(&recent.population, &result.current, zone, false),
            previous: summarize(&earlier.population, &result.previous, zone, false),
        });
        let empty = DailySamples::new();
        result.areas.push(ServerTrend {
            id: id.clone(),
            name: name.clone(),
            current: summarize(&recent.areas, &result.current, zone, false),
            previous: summarize(&earlier.areas, &result.previous, zone, false),
        });
        let mut windows = recent
            .windows
            .iter()
            .map(|((weekday, start_hour), samples)| BusyHours {
                server_id: id.clone(),
                server: name.clone(),
                start_hour: *start_hour,
                weekday: *weekday,
                current: summarize_window(samples, &result.current, zone, *weekday),
                previous: summarize_window(
                    earlier
                        .windows
                        .get(&(*weekday, *start_hour))
                        .unwrap_or(&empty),
                    &result.previous,
                    zone,
                    *weekday,
                ),
            })
            .filter(|window| window.current.value.is_some())
            .collect::<Vec<_>>();
        windows.sort_by(|a, b| {
            descending(a.current.value, b.current.value)
                .then(a.weekday.cmp(&b.weekday))
                .then(a.start_hour.cmp(&b.start_hour))
        });
        result.hours.extend(windows.into_iter().take(3));
    }
    result.servers.sort_by(|a, b| {
        descending(
            change(&a.current, &a.previous).map(|c| c.count),
            change(&b.current, &b.previous).map(|c| c.count),
        )
        .then(a.id.cmp(&b.id))
    });
    result
        .areas
        .sort_by(|a, b| descending(a.current.value, b.current.value).then(a.id.cmp(&b.id)));
    result
}

fn summarize_window(
    samples: &DailySamples,
    interval: &TrendInterval,
    zone: TimeZone,
    weekday: Option<u32>,
) -> PopulationSummary {
    let mut selected = interval.clone();
    if let Some(weekday) = weekday {
        let start = zone.at(interval.start).date_naive();
        selected.days = (0..interval.days)
            .filter(|day| {
                (start + Duration::days(*day as i64))
                    .weekday()
                    .num_days_from_monday()
                    == weekday
            })
            .count();
    }
    summarize(samples, &selected, zone, true)
}
