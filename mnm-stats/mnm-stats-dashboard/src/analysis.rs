//! Dashboard calculations over collected observations, with no resampling or backfill.
use chrono::{DateTime, Datelike, Duration, NaiveDate, Timelike, Utc};
use mnm_stats_model::{History, Snapshot};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Scope {
    All,
    Server(String),
}

impl Scope {
    pub fn label(&self, names: &BTreeMap<String, String>) -> String {
        match self {
            Self::All => "All Servers".into(),
            Self::Server(id) => format!("{} [{id}]", names.get(id).unwrap_or(id)),
        }
    }
}

pub fn servers(history: &History) -> BTreeMap<String, String> {
    history
        .snapshots()
        .iter()
        .flat_map(|s| &s.servers)
        .map(|s| (s.id.clone(), s.name.clone()))
        .collect()
}

pub fn zones(history: &History) -> BTreeMap<String, String> {
    history
        .snapshots()
        .iter()
        .flat_map(|s| &s.servers)
        .flat_map(|s| &s.starting_zones)
        .map(|z| (z.id.clone(), z.name.clone()))
        .collect()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Metric {
    Daily,
    Monthly,
    Subscriptions,
    Online,
    StartingZones,
    Zone(String, String),
    DailyMonthly,
    DailySubscriptions,
    MonthlySubscriptions,
}

impl Metric {
    pub fn key(&self) -> String {
        match self {
            Self::Daily => "daily",
            Self::Monthly => "monthly",
            Self::Subscriptions => "subscriptions",
            Self::Online => "online",
            Self::StartingZones => "starting-zones",
            Self::Zone(id, _) => return format!("zone-{id}"),
            Self::DailyMonthly => "daily-monthly",
            Self::DailySubscriptions => "daily-subscriptions",
            Self::MonthlySubscriptions => "monthly-subscriptions",
        }
        .into()
    }

    pub fn title(&self) -> String {
        match self {
            Self::Daily => "Daily active (DAU)",
            Self::Monthly => "Monthly active (MAU)",
            Self::Subscriptions => "Active subscriptions",
            Self::Online => "Online population",
            Self::StartingZones => "Starting-zone population",
            Self::Zone(_, name) => return name.clone(),
            Self::DailyMonthly => "Daily / monthly activity",
            Self::DailySubscriptions => "Daily activity / global subscriptions",
            Self::MonthlySubscriptions => "Monthly activity / global subscriptions",
        }
        .into()
    }

    pub fn is_ratio(&self) -> bool {
        matches!(
            self,
            Self::DailyMonthly | Self::DailySubscriptions | Self::MonthlySubscriptions
        )
    }

    pub fn unit(&self) -> &'static str {
        if self.is_ratio() {
            "Percent (%)"
        } else {
            "Reported count"
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::Daily | Self::Monthly => {
                "Source-reported activity. All-server values are sums without deduplication, not unique game-wide players. The source's counting semantics are unverified; published zeros are preserved."
            }
            Self::Subscriptions => {
                "Global source-reported subscriptions, not necessarily distinct people. Per-server subscriptions are unavailable."
            }
            Self::Online => {
                "Source-reported concurrent population; all-server values sum the observed servers."
            }
            Self::StartingZones | Self::Zone(..) => {
                "Current population in starting areas, not new players or character creations. Totals sum the selected zones and servers."
            }
            Self::DailyMonthly => {
                "Derived ratio of reported daily and monthly counts. A zero denominator is not available. Values may exceed 100%."
            }
            _ => {
                "Derived ratio of reported activity to global subscriptions, not a proven fraction of subscribers playing. The denominator stays global in server views. Values may exceed 100%."
            }
        }
    }

    pub fn value(&self, snapshot: &Snapshot, scope: &Scope) -> Option<MetricValue> {
        if *self == Self::Subscriptions {
            return Some(MetricValue::Count(snapshot.active_subscriptions.into()));
        }
        let ratio = match self {
            Self::DailyMonthly => Some((Self::Daily, Self::Monthly)),
            Self::DailySubscriptions => Some((Self::Daily, Self::Subscriptions)),
            Self::MonthlySubscriptions => Some((Self::Monthly, Self::Subscriptions)),
            _ => None,
        };
        if let Some((numerator, denominator)) = ratio {
            let MetricValue::Count(numerator) = numerator.value(snapshot, scope)? else {
                return None;
            };
            let MetricValue::Count(denominator) = denominator.value(snapshot, scope)? else {
                return None;
            };
            return (denominator != 0).then_some(MetricValue::Ratio {
                numerator,
                denominator,
            });
        }
        let mut count = 0_u128;
        let mut found = false;
        for server in &snapshot.servers {
            if matches!(scope, Scope::Server(id) if id != &server.id) {
                continue;
            }
            found = true;
            let value = match self {
                Self::Daily => u128::from(server.daily_active),
                Self::Monthly => u128::from(server.monthly_active),
                Self::Online => u128::from(server.online),
                // Collection validates an empty list against a published zero total.
                Self::StartingZones => server
                    .starting_zones
                    .iter()
                    .try_fold(0_u128, |sum, z| sum.checked_add(z.online.into()))?,
                Self::Zone(id, _) => server
                    .starting_zones
                    .iter()
                    .find(|z| &z.id == id)?
                    .online
                    .into(),
                _ => unreachable!(),
            };
            count = count.checked_add(value)?;
        }
        found.then_some(MetricValue::Count(count))
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MetricValue {
    Count(u128),
    Ratio { numerator: u128, denominator: u128 },
}

impl MetricValue {
    pub fn number(self) -> f64 {
        match self {
            Self::Count(n) => n as f64,
            Self::Ratio {
                numerator,
                denominator,
            } => 100.0 * numerator as f64 / denominator as f64,
        }
    }

    pub fn display(self) -> String {
        match self {
            Self::Count(n) => n.to_string(),
            Self::Ratio {
                numerator,
                denominator,
            } => format!("{:.2}% ({numerator} / {denominator})", self.number()),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TimeRange {
    Days7,
    #[default]
    Days30,
    Days90,
    Days180,
    Year,
    All,
}

impl TimeRange {
    pub const ALL: [Self; 6] = [
        Self::Days7,
        Self::Days30,
        Self::Days90,
        Self::Days180,
        Self::Year,
        Self::All,
    ];
    pub fn key(self) -> &'static str {
        match self {
            Self::Days7 => "7",
            Self::Days30 => "30",
            Self::Days90 => "90",
            Self::Days180 => "180",
            Self::Year => "365",
            Self::All => "all",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Days7 => "Last 7 days",
            Self::Days30 => "Last 30 days",
            Self::Days90 => "Last 90 days",
            Self::Days180 => "Last 180 days",
            Self::Year => "Last year (365 days)",
            Self::All => "All time",
        }
    }
    pub fn bounds(self, history: &History, now: DateTime<Utc>) -> (DateTime<Utc>, DateTime<Utc>) {
        let days = match self {
            Self::Days7 => 7,
            Self::Days30 => 30,
            Self::Days90 => 90,
            Self::Days180 => 180,
            Self::Year => 365,
            Self::All => {
                return (
                    history.snapshots().first().map_or(now, |s| s.observed_at),
                    now,
                );
            }
        };
        (now - Duration::days(days), now)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Period {
    Month(NaiveDate),
    Year(i32),
    Interval { start: DateTime<Utc>, hours: u32 },
}

impl Period {
    pub fn month(input: &str) -> Result<Self, String> {
        let date = NaiveDate::parse_from_str(&format!("{input}-01"), "%Y-%m-%d")
            .map_err(|_| "Choose a valid month (YYYY-MM).")?;
        let result = Self::Month(date);
        result
            .bounds()
            .ok_or("Month is outside the supported calendar.")?;
        Ok(result)
    }
    pub fn year(input: &str) -> Result<Self, String> {
        let year = input.parse().map_err(|_| "Choose a valid year.")?;
        if !(1..=9998).contains(&year) {
            return Err("Year must be between 1 and 9998.".into());
        }
        Ok(Self::Year(year))
    }
    pub fn interval(input: &str, hours: u32) -> Result<Self, String> {
        let start = chrono::NaiveDateTime::parse_from_str(input, "%Y-%m-%dT%H:%M")
            .map_err(|_| "Choose a valid UTC start date and time.")?
            .and_utc();
        let result = Self::Interval { start, hours };
        result
            .bounds()
            .ok_or("Duration must be positive and fit the supported calendar.")?;
        Ok(result)
    }
    pub fn bounds(&self) -> Option<(DateTime<Utc>, DateTime<Utc>)> {
        let midnight = |date: NaiveDate| date.and_hms_opt(0, 0, 0).map(|d| d.and_utc());
        match self {
            Self::Month(date) => {
                if date.day() != 1 {
                    return None;
                }
                let end = date.checked_add_months(chrono::Months::new(1))?;
                Some((midnight(*date)?, midnight(end)?))
            }
            Self::Year(year) => Some((
                midnight(NaiveDate::from_ymd_opt(*year, 1, 1)?)?,
                midnight(NaiveDate::from_ymd_opt(year.checked_add(1)?, 1, 1)?)?,
            )),
            Self::Interval { start, hours } if *hours > 0 => Some((
                *start,
                start.checked_add_signed(Duration::hours((*hours).into()))?,
            )),
            _ => None,
        }
    }
    pub fn label(&self) -> String {
        match self {
            Self::Month(date) => date.format("%B %Y (UTC)").to_string(),
            Self::Year(year) => format!("Calendar {year} (UTC)"),
            Self::Interval { start, hours } => {
                format!("{} UTC · {hours} hours", start.format("%Y-%m-%d %H:%M"))
            }
        }
    }
    pub fn alignment(&self) -> Alignment {
        match self {
            Self::Month(_) => Alignment::Month,
            Self::Year(_) => Alignment::Year,
            Self::Interval { .. } => Alignment::Elapsed,
        }
    }
    pub fn x(&self, time: DateTime<Utc>) -> f64 {
        let fraction = f64::from(time.nanosecond()) / 1_000_000_000.0;
        match self {
            Self::Month(_) => {
                f64::from((time.day() - 1) * 86400 + time.num_seconds_from_midnight()) + fraction
            }
            Self::Year(_) => {
                // A leap-year axis reserves February 29 even for non-leap years.
                let day = NaiveDate::from_ymd_opt(2000, time.month(), time.day())
                    .unwrap()
                    .ordinal0();
                f64::from(day * 86400 + time.num_seconds_from_midnight()) + fraction
            }
            Self::Interval { start, .. } => seconds(time) - seconds(*start),
        }
    }
}

pub fn seconds(time: DateTime<Utc>) -> f64 {
    time.timestamp() as f64 + f64::from(time.timestamp_subsec_nanos()) / 1_000_000_000.0
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Alignment {
    Utc,
    Month,
    Year,
    Elapsed,
}

impl Alignment {
    pub fn description(self) -> &'static str {
        match self {
            Self::Utc => "Observation time (UTC)",
            Self::Month => "Day of month and time (UTC)",
            Self::Year => "Month and day (UTC; leap-day space retained)",
            Self::Elapsed => "Elapsed hours from each UTC start",
        }
    }
    pub fn tick(self, x: f64) -> String {
        match self {
            Self::Utc => DateTime::from_timestamp(x as i64, 0)
                .map_or_else(String::new, |t| t.format("%d %b %H:%M").to_string()),
            Self::Month => format!(
                "{} {:02}:{:02}",
                x as u32 / 86400 + 1,
                x as u32 % 86400 / 3600,
                x as u32 % 3600 / 60
            ),
            Self::Year => (NaiveDate::from_ymd_opt(2000, 1, 1)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap()
                + Duration::seconds(x as i64))
            .format("%d %b")
            .to_string(),
            Self::Elapsed => format!("{:.2} h", x / 3600.0),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Comparison {
    None,
    Entities(Vec<Scope>),
    Periods(Vec<Period>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Point {
    pub at: DateTime<Utc>,
    pub x: f64,
    pub value: Option<MetricValue>,
}

impl Point {
    pub(crate) fn has_gap_from(&self, previous: &Self) -> bool {
        self.at - previous.at > Duration::hours(2) || self.x - previous.x > 7200.0
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Series {
    pub identity: String,
    pub style: usize,
    pub label: String,
    pub points: Vec<Point>,
}

impl Series {
    pub fn segments(&self) -> Vec<Vec<(f64, f64)>> {
        let mut segments = Vec::new();
        let mut segment = Vec::new();
        let mut previous: Option<&Point> = None;
        for point in &self.points {
            let gap = previous.is_some_and(|p| point.has_gap_from(p));
            if (point.value.is_none() || gap) && !segment.is_empty() {
                segments.push(std::mem::take(&mut segment));
            }
            if let Some(value) = point.value {
                segment.push((point.x, value.number()));
            }
            previous = Some(point);
        }
        if !segment.is_empty() {
            segments.push(segment);
        }
        segments
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Plot {
    pub series: Vec<Series>,
    pub alignment: Alignment,
    pub x_bounds: (f64, f64),
    pub note: String,
}

pub fn plot(
    history: &History,
    metric: &Metric,
    scope: &Scope,
    range: TimeRange,
    now: DateTime<Utc>,
    comparison: &Comparison,
) -> Result<Plot, String> {
    let names = servers(history);
    let scope_label = |scope: &Scope| {
        if *metric == Metric::Subscriptions {
            "Global subscriptions".to_owned()
        } else {
            scope.label(&names)
        }
    };
    let (start, end) = range.bounds(history, now);
    let mut result = Plot {
        series: Vec::new(),
        alignment: Alignment::Utc,
        x_bounds: (seconds(start), seconds(end)),
        note: String::new(),
    };
    match comparison {
        Comparison::Periods(periods) => {
            if periods.is_empty() {
                result.note = "Add periods to compare.".into();
                return Ok(result);
            }
            result.alignment = periods[0].alignment();
            if periods.iter().any(|p| p.alignment() != result.alignment) {
                return Err("Compare periods of the same kind.".into());
            }
            let first_bounds = periods[0].bounds().ok_or("Invalid comparison period.")?;
            let duration = first_bounds.1 - first_bounds.0;
            result.x_bounds = (
                0.0,
                match result.alignment {
                    Alignment::Month => 31.0 * 86400.0,
                    Alignment::Year => 366.0 * 86400.0,
                    _ => duration.num_seconds() as f64,
                },
            );
            for period in periods {
                let (start, end) = period.bounds().ok_or("Invalid comparison period.")?;
                if result.alignment == Alignment::Elapsed && end - start != duration {
                    return Err("Intervals must have equal durations.".into());
                }
                result.series.push(Series {
                    identity: format!("period:{}", period.label()),
                    style: result.series.len(),
                    label: format!("{} · {}", scope_label(scope), period.label()),
                    points: history
                        .snapshots()
                        .iter()
                        .filter(|s| s.observed_at >= start && s.observed_at < end)
                        .map(|s| Point {
                            at: s.observed_at,
                            x: period.x(s.observed_at),
                            value: metric.value(s, scope),
                        })
                        .collect(),
                });
            }
            result.note = "Missing dates and observations remain gaps; incomplete periods are not extrapolated.".into();
        }
        other => {
            let scopes = match other {
                Comparison::Entities(scopes) => scopes.clone(),
                _ => vec![scope.clone()],
            };
            let scopes = if *metric == Metric::Subscriptions {
                if matches!(other, Comparison::Entities(_)) {
                    result.note =
                        "Subscriptions are global; server-specific comparison is unavailable."
                            .into();
                }
                vec![Scope::All]
            } else {
                scopes
            };
            for scope in scopes {
                result.series.push(Series {
                    identity: if *metric == Metric::Subscriptions {
                        "global".into()
                    } else {
                        format!("entity:{scope:?}")
                    },
                    style: result.series.len(),
                    label: scope_label(&scope),
                    points: history
                        .snapshots()
                        .iter()
                        .filter(|s| s.observed_at >= start && s.observed_at <= end)
                        .map(|s| Point {
                            at: s.observed_at,
                            x: seconds(s.observed_at),
                            value: metric.value(s, &scope),
                        })
                        .collect(),
                });
            }
            if result.series.is_empty() {
                result.note = "Select entities to compare.".into();
            }
        }
    }
    if result.x_bounds.1 <= result.x_bounds.0 {
        result.x_bounds.1 = result.x_bounds.0 + 3600.0;
    }
    Ok(result)
}

#[derive(Debug, PartialEq)]
pub struct Correlation {
    pub paired_days: usize,
    pub r: Option<f64>,
}

pub fn correlation(
    history: &History,
    a: &Metric,
    b: &Metric,
    scope: &Scope,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> Correlation {
    let mut days = BTreeMap::new();
    for snapshot in history
        .snapshots()
        .iter()
        .filter(|s| s.observed_at >= start && s.observed_at <= end)
    {
        if let Some((a, b)) = a.value(snapshot, scope).zip(b.value(snapshot, scope)) {
            days.insert(snapshot.observed_at.date_naive(), (a.number(), b.number()));
        }
    }
    let count = days.len();
    // Online covariance avoids cancellation from subtracting large squared sums.
    let (mut mx, mut my, mut xx, mut yy, mut xy) = (0.0, 0.0, 0.0, 0.0, 0.0);
    for (i, (x, y)) in days.into_values().enumerate() {
        let n = (i + 1) as f64;
        let (dx, dy) = (x - mx, y - my);
        mx += dx / n;
        my += dy / n;
        xx += dx * (x - mx);
        yy += dy * (y - my);
        xy += dx * (y - my);
    }
    let r =
        (count >= 3 && xx > 0.0 && yy > 0.0).then(|| (xy / xx.sqrt() / yy.sqrt()).clamp(-1.0, 1.0));
    Correlation {
        paired_days: count,
        r,
    }
}

/// All headline values must use this same snapshot, including unavailable servers.
pub fn latest_in_range(
    history: &History,
    range: TimeRange,
    now: DateTime<Utc>,
) -> Option<&Snapshot> {
    let (start, end) = range.bounds(history, now);
    history
        .snapshots()
        .iter()
        .rev()
        .find(|s| s.observed_at >= start && s.observed_at <= end)
}

pub fn grouped_count(value: u128) -> String {
    let digits = value.to_string();
    digits
        .chars()
        .enumerate()
        .fold(String::new(), |mut result, (index, digit)| {
            if index > 0 && (digits.len() - index).is_multiple_of(3) {
                result.push(',');
            }
            result.push(digit);
            result
        })
}
