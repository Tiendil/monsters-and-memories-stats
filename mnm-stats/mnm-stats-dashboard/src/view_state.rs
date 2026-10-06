//! Shareable applied dashboard settings; transient menu and hover state stays in the UI.
use crate::analysis::{ComparisonMode, DateMatching, Metric, Period, Scope, TimeRange, ZoneScope};
use crate::time::{TimeMode, TimeZone};
use chrono::Duration;

pub const ONLINE_METRICS: [Metric; 2] = [Metric::OnlineDaily, Metric::OnlineMonthly];
pub const SUBSCRIBER_METRICS: [Metric; 3] = [
    Metric::DailySubscriptions,
    Metric::MonthlySubscriptions,
    Metric::OnlineSubscriptions,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Section {
    Overview,
    Population,
    Relationships,
}
impl Section {
    pub const ALL: [Self; 3] = [Self::Overview, Self::Population, Self::Relationships];
    pub fn label(self) -> &'static str {
        match self {
            Self::Overview => "Overview",
            Self::Population => "Player activity",
            Self::Relationships => "Engagement",
        }
    }
    pub fn key(self) -> &'static str {
        match self {
            Self::Overview => "overview",
            Self::Population => "population",
            Self::Relationships => "relationships",
        }
    }

    pub fn fragment(self) -> &'static str {
        match self {
            Self::Overview => "#overview",
            Self::Population => "#player-activity",
            Self::Relationships => "#engagement",
        }
    }

    pub fn from_fragment(fragment: &str) -> Option<Self> {
        match fragment {
            ""
            | "#overview"
            | "#chart-online"
            | "#chart-daily"
            | "#chart-monthly"
            | "#chart-subscriptions" => Some(Self::Overview),
            "#player-activity"
            | "#chart-starting-zones"
            | "#chart-online-share"
            | "#chart-activity-heatmap" => Some(Self::Population),
            "#engagement"
            | "#chart-daily-monthly"
            | "#chart-online-presence"
            | "#chart-subscriber-activity" => Some(Self::Relationships),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ViewState {
    pub target: String,
    pub scopes: Vec<Scope>,
    pub range: TimeRange,
    pub time_mode: TimeMode,
    pub comparison: ComparisonMode,
    pub matching: DateMatching,
    pub periods: Vec<Period>,
    pub zones: Vec<ZoneScope>,
    pub online_metrics: Vec<Metric>,
    pub subscriber_metrics: Vec<Metric>,
}

impl Default for ViewState {
    fn default() -> Self {
        Self {
            target: "overview".into(),
            scopes: vec![Scope::All],
            range: TimeRange::default(),
            time_mode: TimeMode::default(),
            comparison: ComparisonMode::default(),
            matching: DateMatching::default(),
            periods: Vec::new(),
            zones: vec![ZoneScope::All],
            online_metrics: ONLINE_METRICS.to_vec(),
            subscriber_metrics: SUBSCRIBER_METRICS.to_vec(),
        }
    }
}

fn selection<T: Clone + PartialEq>(
    values: Vec<&str>,
    defaults: &[T],
    parse: impl Fn(&str) -> Option<T>,
) -> Vec<T> {
    if values.is_empty() {
        return defaults.to_vec();
    }
    let explicitly_empty = values.contains(&"");
    let mut selected = Vec::new();
    for value in values.into_iter().filter_map(parse) {
        if !selected.contains(&value) {
            selected.push(value);
        }
    }
    if selected.is_empty() && !explicitly_empty {
        defaults.to_vec()
    } else {
        selected
    }
}

impl ViewState {
    pub fn section(&self) -> Section {
        Section::from_fragment(&format!("#{}", self.target)).unwrap_or(Section::Overview)
    }

    pub fn from_fragment(fragment: &str) -> Self {
        let (target, query) = fragment
            .trim_start_matches('#')
            .split_once('?')
            .unwrap_or((fragment.trim_start_matches('#'), ""));
        let mut state = Self::default();
        if !target.is_empty() && Section::from_fragment(&format!("#{target}")).is_some() {
            state.target = target.into();
        }
        let pairs = form_urlencoded::parse(query.as_bytes())
            .into_owned()
            .collect::<Vec<_>>();
        let one = |key: &str| {
            pairs
                .iter()
                .rev()
                .find(|(k, _)| k == key)
                .map(|(_, v)| v.as_str())
        };
        let values = |key: &str| {
            pairs
                .iter()
                .filter(|(k, _)| k == key)
                .map(|(_, v)| v.as_str())
                .collect()
        };
        state.time_mode = if one("tz") == Some("utc") {
            TimeMode::Utc
        } else {
            TimeMode::Local
        };
        state.range = match one("range") {
            Some("custom") => one("from")
                .zip(one("to"))
                .and_then(|(start, end)| TimeRange::custom(start, end).ok())
                .unwrap_or_default(),
            Some(key) => TimeRange::ALL
                .into_iter()
                .find(|range| range.key() == key)
                .unwrap_or_default(),
            None => TimeRange::default(),
        };
        state.scopes = selection(values("scope"), &state.scopes, |value| {
            if value == "all" {
                Some(Scope::All)
            } else {
                value
                    .strip_prefix("server:")
                    .filter(|id| !id.is_empty())
                    .map(|id| Scope::Server(id.into()))
            }
        });
        state.zones = selection(values("zone"), &state.zones, |value| {
            if value == "all" {
                Some(ZoneScope::All)
            } else {
                value
                    .strip_prefix("zone:")
                    .filter(|id| !id.is_empty())
                    .map(|id| ZoneScope::Zone(id.into()))
            }
        });
        state.comparison = ComparisonMode::ALL
            .into_iter()
            .find(|mode| Some(mode.key()) == one("compare"))
            .unwrap_or_default();
        state.matching = if one("match") == Some("weekday") {
            DateMatching::Weekday
        } else {
            DateMatching::ExactDate
        };
        for value in values("period") {
            if let Some((from, to)) = value.split_once('/')
                && let Ok(period) = Period::custom(from, to)
                && !state.periods.contains(&period)
            {
                state.periods.push(period);
            }
        }
        state.online_metrics = selection(values("online-metric"), &ONLINE_METRICS, |value| {
            ONLINE_METRICS
                .into_iter()
                .find(|metric| metric.key() == value)
        });
        state.subscriber_metrics =
            selection(values("subscriber-metric"), &SUBSCRIBER_METRICS, |value| {
                SUBSCRIBER_METRICS
                    .into_iter()
                    .find(|metric| metric.key() == value)
            });
        state
    }

    pub fn fragment(&self) -> String {
        self.link(&self.target)
    }

    /// Native links retain the view settings while changing only the destination.
    pub fn link(&self, target: &str) -> String {
        let mut query = form_urlencoded::Serializer::new(String::new());
        if self.time_mode == TimeMode::Utc {
            query.append_pair("tz", "utc");
        }
        if self.range != TimeRange::default() {
            query.append_pair("range", self.range.key());
            if let TimeRange::Custom { start, end } = self.range {
                query.append_pair("from", &start.format("%Y-%m-%d").to_string());
                query.append_pair("to", &end.format("%Y-%m-%d").to_string());
            }
        }
        if self.scopes != [Scope::All] {
            if self.scopes.is_empty() {
                query.append_pair("scope", "");
            }
            for scope in &self.scopes {
                query.append_pair(
                    "scope",
                    &match scope {
                        Scope::All => "all".into(),
                        Scope::Server(id) => format!("server:{id}"),
                    },
                );
            }
        }
        if self.comparison != ComparisonMode::Disabled {
            query.append_pair("compare", self.comparison.key());
        }
        if self.matching == DateMatching::Weekday {
            query.append_pair("match", "weekday");
        }
        for period in &self.periods {
            if let Some((start, end)) = period.bounds(TimeZone::UTC) {
                query.append_pair(
                    "period",
                    &format!(
                        "{}/{}",
                        start.format("%Y-%m-%d"),
                        (end - Duration::nanoseconds(1)).format("%Y-%m-%d")
                    ),
                );
            }
        }
        if self.zones != [ZoneScope::All] {
            if self.zones.is_empty() {
                query.append_pair("zone", "");
            }
            for zone in &self.zones {
                query.append_pair(
                    "zone",
                    &match zone {
                        ZoneScope::All => "all".into(),
                        ZoneScope::Zone(id) => format!("zone:{id}"),
                    },
                );
            }
        }
        for (key, selected, defaults) in [
            (
                "online-metric",
                self.online_metrics.as_slice(),
                ONLINE_METRICS.as_slice(),
            ),
            (
                "subscriber-metric",
                self.subscriber_metrics.as_slice(),
                SUBSCRIBER_METRICS.as_slice(),
            ),
        ] {
            if selected != defaults {
                if selected.is_empty() {
                    query.append_pair(key, "");
                }
                for metric in selected {
                    query.append_pair(key, &metric.key());
                }
            }
        }
        let query = query.finish();
        if query.is_empty() {
            format!("#{target}")
        } else {
            format!("#{target}?{query}")
        }
    }
}
