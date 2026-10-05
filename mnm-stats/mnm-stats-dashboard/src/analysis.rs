//! Dashboard calculations over collected observations, with no resampling or backfill.
use chrono::{DateTime, Datelike, Duration, NaiveDate, Timelike, Utc};
use mnm_stats_model::{History, Snapshot};
use std::collections::BTreeMap;

pub fn display_name(id: &str, name: &str) -> String {
    if name.trim().is_empty() { id } else { name }.to_owned()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Scope {
    All,
    Server(String),
}

impl Scope {
    pub fn label(&self, names: &BTreeMap<String, String>) -> String {
        match self {
            Self::All => "All Servers".into(),
            Self::Server(id) => display_name(id, names.get(id).map_or("", String::as_str)),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ZoneScope {
    All,
    Zone(String),
}

impl ZoneScope {
    pub fn label(&self, names: &BTreeMap<String, String>) -> String {
        match self {
            Self::All => "All Zones".into(),
            Self::Zone(id) => display_name(id, names.get(id).map_or("", String::as_str)),
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
    OnlineShare,
    StartingZones,
    Zone(String, String),
    DailyMonthly,
    DailySubscriptions,
    MonthlySubscriptions,
    OnlineDaily,
    OnlineMonthly,
    OnlineSubscriptions,
}

impl Metric {
    pub fn key(&self) -> String {
        match self {
            Self::Daily => "daily",
            Self::Monthly => "monthly",
            Self::Subscriptions => "subscriptions",
            Self::Online => "online",
            Self::OnlineShare => "online-share",
            Self::StartingZones => "starting-zones",
            Self::Zone(id, _) => return format!("zone-{id}"),
            Self::DailyMonthly => "daily-monthly",
            Self::DailySubscriptions => "daily-subscriptions",
            Self::MonthlySubscriptions => "monthly-subscriptions",
            Self::OnlineDaily => "online-daily",
            Self::OnlineMonthly => "online-monthly",
            Self::OnlineSubscriptions => "online-subscriptions",
        }
        .into()
    }

    pub fn title(&self) -> String {
        match self {
            Self::Daily => "Daily active (DAU)",
            Self::Monthly => "Monthly active (MAU)",
            Self::Subscriptions => "Subscribers",
            Self::Online => "Online",
            Self::OnlineShare => "Server population share",
            Self::StartingZones => "Starting-zone population",
            Self::Zone(id, name) => return display_name(id, name),
            Self::DailyMonthly => "Daily / monthly activity",
            Self::DailySubscriptions => "Daily activity / global subscribers",
            Self::MonthlySubscriptions => "Monthly activity / global subscribers",
            Self::OnlineDaily => "Online / daily active",
            Self::OnlineMonthly => "Online / monthly active",
            Self::OnlineSubscriptions => "Online / global subscribers",
        }
        .into()
    }

    pub fn is_ratio(&self) -> bool {
        matches!(
            self,
            Self::OnlineShare
                | Self::DailyMonthly
                | Self::DailySubscriptions
                | Self::MonthlySubscriptions
                | Self::OnlineDaily
                | Self::OnlineMonthly
                | Self::OnlineSubscriptions
        )
    }

    pub fn unit(&self) -> &'static str {
        if self.is_ratio() {
            "Percent (%)"
        } else {
            "Count"
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::Daily | Self::Monthly => {
                "Activity counts. All Servers adds each server's count, so a player active on several servers may be counted more than once."
            }
            Self::Subscriptions => {
                "Active subscriptions across the game. Multiple subscriptions may belong to the same player; counts for individual servers are not available."
            }
            Self::Online => {
                "Players online. All Servers adds the counts from all servers in that record."
            }
            Self::OnlineShare => {
                "Each server's share of all online players in the same record. No share is shown when the server is missing or the total is zero."
            }
            Self::StartingZones | Self::Zone(..) => {
                "Players in starting areas, rather than a count of new players or characters. All Zones combines all starting areas. All Servers adds their counts across servers."
            }
            Self::DailyMonthly => {
                "Daily activity as a percentage of monthly activity. No ratio is shown when monthly activity is zero. Values can exceed 100%."
            }
            Self::OnlineDaily | Self::OnlineMonthly => {
                "Players online, shown as a percentage of the daily or monthly active audience."
            }
            _ => {
                "Activity compared with total subscriptions across the game, even when viewing one server. These ratios can exceed 100% and do not show what share of subscribers are playing."
            }
        }
    }

    pub fn value(&self, snapshot: &Snapshot, scope: &Scope) -> Option<MetricValue> {
        if *self == Self::Subscriptions {
            return Some(MetricValue::Count(snapshot.active_subscriptions.into()));
        }
        let ratio = match self {
            Self::OnlineShare => Some((Self::Online, Self::Online)),
            Self::DailyMonthly => Some((Self::Daily, Self::Monthly)),
            Self::DailySubscriptions => Some((Self::Daily, Self::Subscriptions)),
            Self::MonthlySubscriptions => Some((Self::Monthly, Self::Subscriptions)),
            Self::OnlineDaily => Some((Self::Online, Self::Daily)),
            Self::OnlineMonthly => Some((Self::Online, Self::Monthly)),
            Self::OnlineSubscriptions => Some((Self::Online, Self::Subscriptions)),
            _ => None,
        };
        if let Some((numerator, denominator)) = ratio {
            let MetricValue::Count(numerator) = numerator.value(snapshot, scope)? else {
                return None;
            };
            let MetricValue::Count(denominator) = denominator.value(
                snapshot,
                if *self == Self::OnlineShare {
                    &Scope::All
                } else {
                    scope
                },
            )?
            else {
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
            Self::Ratio { .. } => format!("{:.2}%", self.number()),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TimeRange {
    Today,
    Yesterday,
    #[default]
    Days7,
    Days30,
    Days90,
    Days180,
    Year,
    All,
    Custom {
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    },
}

impl TimeRange {
    pub const ALL: [Self; 8] = [
        Self::Today,
        Self::Yesterday,
        Self::Days7,
        Self::Days30,
        Self::Days90,
        Self::Days180,
        Self::Year,
        Self::All,
    ];
    pub fn key(self) -> &'static str {
        match self {
            Self::Today => "today",
            Self::Yesterday => "yesterday",
            Self::Days7 => "7",
            Self::Days30 => "30",
            Self::Days90 => "90",
            Self::Days180 => "180",
            Self::Year => "365",
            Self::All => "all",
            Self::Custom { .. } => "custom",
        }
    }
    pub fn label(self) -> String {
        match self {
            Self::Today => "Today",
            Self::Yesterday => "Yesterday",
            Self::Days7 => "Last 7 days",
            Self::Days30 => "Last 30 days",
            Self::Days90 => "Last 90 days",
            Self::Days180 => "Last 180 days",
            Self::Year => "Last year (365 days)",
            Self::All => "All time",
            Self::Custom { start, end } => return date_label(start, end),
        }
        .into()
    }
    pub fn custom(start: &str, end: &str) -> Result<Self, String> {
        let (start, end) = date_bounds(start, end)?;
        Ok(Self::Custom {
            start,
            end: end - Duration::nanoseconds(1),
        })
    }
    pub fn bounds(self, history: &History, now: DateTime<Utc>) -> (DateTime<Utc>, DateTime<Utc>) {
        let days = match self {
            Self::Today | Self::Yesterday => {
                let midnight = now.date_naive().and_hms_opt(0, 0, 0).unwrap().and_utc();
                return if self == Self::Today {
                    (midnight, now)
                } else {
                    (
                        midnight - Duration::days(1),
                        midnight - Duration::nanoseconds(1),
                    )
                };
            }
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
            Self::Custom { start, end } => return (start, end),
        };
        (now - Duration::days(days), now)
    }
}

fn date_bounds(start: &str, end: &str) -> Result<(DateTime<Utc>, DateTime<Utc>), String> {
    let parse = |value: &str| {
        NaiveDate::parse_from_str(value, "%Y-%m-%d")
            .ok()
            .filter(|d| (1..=9998).contains(&d.year()))
            .ok_or_else(|| "Choose valid dates between years 1 and 9998.".to_string())
    };
    let start = parse(start)?;
    let end = parse(end)?;
    if start > end {
        return Err("The end date must be on or after the start date.".into());
    }
    Ok((
        start.and_hms_opt(0, 0, 0).unwrap().and_utc(),
        end.succ_opt()
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc(),
    ))
}

fn date_label(start: DateTime<Utc>, end: DateTime<Utc>) -> String {
    format!("{} – {}", start.format("%d %b %Y"), end.format("%d %b %Y"))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Period {
    Month(NaiveDate),
    Year(i32),
    Interval {
        start: DateTime<Utc>,
        hours: u32,
    },
    Window {
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    },
    CalendarRange {
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    },
}

impl Period {
    pub fn custom(start: &str, end: &str) -> Result<Self, String> {
        let (start, end) = date_bounds(start, end)?;
        Ok(Self::window(start, end))
    }
    pub fn window(start: DateTime<Utc>, end: DateTime<Utc>) -> Self {
        if start.time() == chrono::NaiveTime::MIN && start.day() == 1 {
            if start.month() == 1 && start.checked_add_months(chrono::Months::new(12)) == Some(end)
            {
                return Self::Year(start.year());
            }
            if start.checked_add_months(chrono::Months::new(1)) == Some(end) {
                return Self::Month(start.date_naive());
            }
        }
        Self::Window { start, end }
    }
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
            Self::Window { start, end } | Self::CalendarRange { start, end } if end > start => {
                Some((*start, *end))
            }
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
            Self::Window { start, end } | Self::CalendarRange { start, end } => {
                format!(
                    "{} (UTC)",
                    date_label(*start, *end - Duration::nanoseconds(1))
                )
            }
        }
    }
    pub fn alignment(&self) -> Alignment {
        match self {
            Self::Month(_) => Alignment::Month,
            Self::Year(_) => Alignment::Year,
            Self::CalendarRange { .. } => Alignment::Year,
            Self::Interval { .. } | Self::Window { .. } => Alignment::Elapsed,
        }
    }
    pub fn x(&self, time: DateTime<Utc>) -> f64 {
        let fraction = f64::from(time.nanosecond()) / 1_000_000_000.0;
        match self {
            Self::Month(_) => {
                f64::from((time.day() - 1) * 86400 + time.num_seconds_from_midnight()) + fraction
            }
            Self::Year(_) | Self::CalendarRange { .. } => {
                // A leap-year axis reserves February 29 even for non-leap years.
                let day = NaiveDate::from_ymd_opt(2000, time.month(), time.day())
                    .unwrap()
                    .ordinal0();
                let years = match self {
                    Self::CalendarRange { start, .. } => time.year() - start.year(),
                    _ => 0,
                };
                f64::from(years) * 366.0 * 86400.0
                    + f64::from(day * 86400 + time.num_seconds_from_midnight())
                    + fraction
            }
            Self::Interval { start, .. } | Self::Window { start, .. } => {
                seconds(time) - seconds(*start)
            }
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
                + Duration::seconds((x as i64).rem_euclid(366 * 86400)))
            .format("%d %b")
            .to_string(),
            Self::Elapsed => format!("{:.2} h", x / 3600.0),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Comparison {
    None,
    Periods(Vec<ComparedPeriod>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComparedPeriod {
    pub identity: String,
    pub period: Period,
}

impl Comparison {
    pub fn periods(periods: Vec<Period>) -> Self {
        Self::Periods(
            periods
                .into_iter()
                .map(|period| ComparedPeriod {
                    identity: period.label(),
                    period,
                })
                .collect(),
        )
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ComparisonMode {
    #[default]
    Disabled,
    Previous,
    YearOverYear,
    Custom,
}

impl ComparisonMode {
    pub const ALL: [Self; 4] = [
        Self::Disabled,
        Self::Previous,
        Self::YearOverYear,
        Self::Custom,
    ];
    pub fn key(self) -> &'static str {
        match self {
            Self::Disabled => "disabled",
            Self::Previous => "previous",
            Self::YearOverYear => "year-over-year",
            Self::Custom => "custom",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Disabled => "Disable comparison",
            Self::Previous => "Previous period",
            Self::YearOverYear => "Year over year",
            Self::Custom => "Custom period",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DateMatching {
    #[default]
    ExactDate,
    Weekday,
}

/// Resolve the primary range and its comparisons without changing original timestamps.
pub fn comparison_for(
    history: &History,
    range: TimeRange,
    now: DateTime<Utc>,
    mode: ComparisonMode,
    matching: DateMatching,
    custom: &[Period],
) -> Result<Comparison, String> {
    if mode == ComparisonMode::Disabled || (mode == ComparisonMode::Custom && custom.is_empty()) {
        return Ok(Comparison::None);
    }
    let (start, last) = range.bounds(history, now);
    let end = last
        .checked_add_signed(Duration::nanoseconds(1))
        .ok_or("Range exceeds the supported calendar.")?;
    if end <= start {
        return Err("Choose a range ending after its start.".into());
    }
    let duration = if matches!(
        range,
        TimeRange::Today | TimeRange::Yesterday | TimeRange::Custom { .. }
    ) {
        end - start
    } else {
        (last - start).max(Duration::nanoseconds(1))
    };
    let mut primary = Period::window(start, end);
    let mut secondary = match mode {
        ComparisonMode::Previous => {
            let mut previous = start
                .checked_sub_signed(if range == TimeRange::Today {
                    Duration::days(1)
                } else {
                    duration
                })
                .ok_or("Previous period exceeds the supported calendar.")?;
            if matching == DateMatching::Weekday {
                let days = (previous.weekday().num_days_from_monday() as i64
                    - start.weekday().num_days_from_monday() as i64)
                    .rem_euclid(7);
                previous = previous
                    .checked_sub_signed(Duration::days(days))
                    .ok_or("Previous period exceeds the supported calendar.")?;
            }
            vec![ComparedPeriod {
                identity: "previous".into(),
                period: Period::window(previous, previous + duration),
            }]
        }
        ComparisonMode::YearOverYear => {
            let previous = start
                .checked_sub_months(chrono::Months::new(12))
                .ok_or("Previous year exceeds the supported calendar.")?;
            let previous_end = last
                .checked_sub_months(chrono::Months::new(12))
                .and_then(|t| t.checked_add_signed(Duration::nanoseconds(1)))
                .ok_or("Previous year exceeds the supported calendar.")?;
            primary = Period::CalendarRange { start, end };
            vec![ComparedPeriod {
                identity: "year-over-year".into(),
                period: Period::CalendarRange {
                    start: previous,
                    end: previous_end,
                },
            }]
        }
        ComparisonMode::Custom => custom
            .iter()
            .cloned()
            .map(|period| ComparedPeriod {
                identity: period.label(),
                period,
            })
            .collect(),
        ComparisonMode::Disabled => unreachable!(),
    };
    if matching == DateMatching::Weekday {
        primary = Period::Window { start, end };
        for selected in &mut secondary {
            let (other, other_end) = selected
                .period
                .bounds()
                .ok_or("Invalid comparison period.")?;
            let days = (start.weekday().num_days_from_monday() as i64
                - other.weekday().num_days_from_monday() as i64
                + 3)
            .rem_euclid(7)
                - 3;
            let adjusted = other
                .checked_add_signed(Duration::days(days))
                .ok_or("Comparison exceeds the supported calendar.")?;
            let length = if mode == ComparisonMode::YearOverYear {
                end - start
            } else {
                other_end - other
            };
            selected.period = Period::Window {
                start: adjusted,
                end: adjusted
                    .checked_add_signed(length)
                    .ok_or("Comparison exceeds the supported calendar.")?,
            };
        }
    }
    let mut periods = vec![ComparedPeriod {
        identity: "primary".into(),
        period: primary,
    }];
    for selected in secondary {
        if !periods
            .iter()
            .any(|p| p.period.bounds() == selected.period.bounds())
        {
            periods.push(selected);
        }
    }
    Ok(Comparison::Periods(periods))
}

#[derive(Clone, Debug, PartialEq)]
pub struct Point {
    pub at: DateTime<Utc>,
    pub x: f64,
    pub value: Option<MetricValue>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Connection {
    Regular,
    Sparse,
}

impl Point {
    pub(crate) fn connection_from(&self, previous: &Self) -> Option<Connection> {
        let elapsed = self.at - previous.at;
        // Calendar alignment can introduce an absent leap day between otherwise
        // adjacent observations. Keep that space empty as well as actual outages.
        if self.value.is_none()
            || previous.value.is_none()
            || elapsed >= Duration::hours(24)
            || self.x - previous.x >= 86400.0
        {
            None
        } else if elapsed >= Duration::hours(3) {
            Some(Connection::Sparse)
        } else {
            Some(Connection::Regular)
        }
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
            let gap = previous.is_some_and(|p| point.connection_from(p).is_none());
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
    scopes: &[Scope],
    range: TimeRange,
    now: DateTime<Utc>,
    comparison: &Comparison,
) -> Result<Plot, String> {
    let names = servers(history);
    let scope_label = |scope: &Scope| {
        if *metric == Metric::Subscriptions {
            "Global subscribers".to_owned()
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
    if scopes.is_empty() {
        result.note = "Select servers to show their data.".into();
        return Ok(result);
    }
    let share_scopes;
    let scopes = if *metric == Metric::OnlineShare && scopes.contains(&Scope::All) {
        share_scopes = names.keys().cloned().map(Scope::Server).collect::<Vec<_>>();
        &share_scopes[..]
    } else if *metric == Metric::Subscriptions {
        &[Scope::All][..]
    } else {
        scopes
    };
    match comparison {
        Comparison::Periods(periods) => {
            if periods.is_empty() {
                result.note = "Add periods to compare.".into();
                return Ok(result);
            }
            result.alignment = periods[0].period.alignment();
            if periods
                .iter()
                .any(|p| p.period.alignment() != result.alignment)
            {
                result.alignment = Alignment::Elapsed;
            }
            result.x_bounds = (
                if result.alignment == Alignment::Year {
                    f64::INFINITY
                } else {
                    0.0
                },
                match result.alignment {
                    Alignment::Month => 31.0 * 86400.0,
                    _ => 0.0,
                },
            );
            for selected in periods {
                let period = &selected.period;
                let (start, end) = period.bounds().ok_or("Invalid comparison period.")?;
                if result.alignment == Alignment::Year {
                    result.x_bounds.0 = result.x_bounds.0.min(period.x(start));
                    result.x_bounds.1 = result
                        .x_bounds
                        .1
                        .max(period.x(end - Duration::nanoseconds(1)));
                } else if result.alignment == Alignment::Elapsed {
                    result.x_bounds.1 = result.x_bounds.1.max(seconds(end) - seconds(start));
                }
                for scope in scopes {
                    result.series.push(Series {
                        identity: format!("period:{}:{scope:?}", selected.identity),
                        style: result.series.len(),
                        label: format!("{} · {}", scope_label(scope), period.label()),
                        points: history
                            .snapshots()
                            .iter()
                            .filter(|s| s.observed_at >= start && s.observed_at < end)
                            .map(|s| Point {
                                at: s.observed_at,
                                x: if result.alignment == Alignment::Elapsed {
                                    seconds(s.observed_at) - seconds(start)
                                } else {
                                    period.x(s.observed_at)
                                },
                                value: metric.value(s, scope),
                            })
                            .collect(),
                    });
                }
            }
            result.note = "Incomplete periods show only the available data.".into();
        }
        Comparison::None => {
            for scope in scopes {
                result.series.push(Series {
                    identity: if *metric == Metric::Subscriptions {
                        "global".into()
                    } else {
                        format!("entity:{scope:?}")
                    },
                    style: result.series.len(),
                    label: scope_label(scope),
                    points: history
                        .snapshots()
                        .iter()
                        .filter(|s| s.observed_at >= start && s.observed_at <= end)
                        .map(|s| Point {
                            at: s.observed_at,
                            x: seconds(s.observed_at),
                            value: metric.value(s, scope),
                        })
                        .collect(),
                });
            }
        }
    }
    if result.x_bounds.1 <= result.x_bounds.0 {
        result.x_bounds.1 = result.x_bounds.0 + 3600.0;
    }
    Ok(result)
}

/// Combine compatible percentage series, retaining a distinct identity per metric.
pub fn engagement_plot(
    history: &History,
    metrics: &[Metric],
    scopes: &[Scope],
    range: TimeRange,
    now: DateTime<Utc>,
    comparison: &Comparison,
) -> Result<Plot, String> {
    let mut result = plot(history, &Metric::DailyMonthly, &[], range, now, comparison)?;
    for metric in metrics {
        let mut part = plot(history, metric, scopes, range, now, comparison)?;
        for series in &mut part.series {
            series.identity = format!("metric:{}:{}", metric.key(), series.identity);
            series.label = format!("{} · {}", metric.title(), series.label);
        }
        result.alignment = part.alignment;
        result.x_bounds = part.x_bounds;
        result.note = part.note;
        result.series.extend(part.series);
    }
    if metrics.is_empty() {
        result.note = "Select metrics to show their data.".into();
    }
    for (index, series) in result.series.iter_mut().enumerate() {
        series.style = index;
    }
    Ok(result)
}

/// Combine zone totals and individual zones on the same server/period axes.
pub fn population_plot(
    history: &History,
    selected_zones: &[ZoneScope],
    scopes: &[Scope],
    range: TimeRange,
    now: DateTime<Utc>,
    comparison: &Comparison,
) -> Result<Plot, String> {
    let names = zones(history);
    let mut combined: Option<Plot> = None;
    for zone in selected_zones {
        let label = zone.label(&names);
        let metric = match zone {
            ZoneScope::All => Metric::StartingZones,
            ZoneScope::Zone(id) => Metric::Zone(id.clone(), label.clone()),
        };
        let mut part = plot(history, &metric, scopes, range, now, comparison)?;
        for series in &mut part.series {
            series.label = format!("{label} · {}", series.label);
            if let ZoneScope::Zone(id) = zone {
                series.identity = format!("zone:{id:?}:{}", series.identity);
            }
        }
        if let Some(result) = &mut combined {
            result.series.extend(part.series);
        } else {
            combined = Some(part);
        }
    }
    if let Some(mut result) = combined {
        for (style, series) in result.series.iter_mut().enumerate() {
            series.style = style;
        }
        Ok(result)
    } else {
        let mut result = plot(history, &Metric::StartingZones, &[], range, now, comparison)?;
        result.note = "Select starting zones to show their data.".into();
        Ok(result)
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

/// An unweighted bucket of observed online counts; zero samples means unavailable.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ActivityCell {
    pub total: u128,
    pub samples: usize,
}

impl ActivityCell {
    pub fn mean(self) -> Option<f64> {
        (self.samples > 0).then(|| self.total as f64 / self.samples as f64)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ActivityHeatmap {
    pub label: String,
    /// Monday first, then UTC hour 00 through 23.
    pub cells: [[ActivityCell; 24]; 7],
}

pub fn activity_heatmaps(
    history: &History,
    scopes: &[Scope],
    range: TimeRange,
    now: DateTime<Utc>,
    comparison: &Comparison,
) -> Result<Vec<ActivityHeatmap>, String> {
    plot(history, &Metric::Online, scopes, range, now, comparison)?
        .series
        .into_iter()
        .map(|series| {
            let mut cells = [[ActivityCell::default(); 24]; 7];
            for point in series.points {
                if let Some(MetricValue::Count(value)) = point.value {
                    // Comparison x coordinates may be shifted. Bucket by original UTC time.
                    let cell = &mut cells[point.at.weekday().num_days_from_monday() as usize]
                        [point.at.hour() as usize];
                    cell.total = cell
                        .total
                        .checked_add(value)
                        .ok_or("Online sample total is too large.")?;
                    cell.samples += 1;
                }
            }
            Ok(ActivityHeatmap {
                label: series.label,
                cells,
            })
        })
        .collect()
}

pub fn heatmap_maximum(maps: &[ActivityHeatmap]) -> f64 {
    maps.iter()
        .flat_map(|map| map.cells.iter().flatten())
        .filter_map(|cell| cell.mean())
        .fold(0.0, f64::max)
}
