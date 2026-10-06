//! Dashboard calendar and presentation time; stored observations remain UTC instants.
use chrono::{DateTime, Duration, NaiveDate, NaiveDateTime, TimeZone as _, Utc};
use chrono_tz::{GapInfo, Tz};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TimeMode {
    Utc,
    #[default]
    Local,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TimeZone(Tz);

impl TimeZone {
    pub const UTC: Self = Self(chrono_tz::UTC);

    pub fn from_name(name: &str) -> Option<Self> {
        name.parse().ok().map(Self)
    }

    pub fn name(self) -> &'static str {
        self.0.name()
    }

    pub fn short_label(self) -> &'static str {
        if self == Self::UTC { "UTC" } else { "local" }
    }

    pub fn at(self, time: DateTime<Utc>) -> DateTime<Tz> {
        time.with_timezone(&self.0)
    }

    pub fn format(self, time: DateTime<Utc>, pattern: &str) -> String {
        self.at(time).format(pattern).to_string()
    }

    pub fn timestamp(self, time: DateTime<Utc>) -> String {
        format!("{} {}", self.format(time, "%d %b %Y, %H:%M"), self.name())
    }

    /// Calendar boundaries use the first occurrence of an ambiguous clock time.
    /// A skipped clock time advances to the first valid instant after the gap.
    pub fn resolve(self, local: NaiveDateTime) -> Option<DateTime<Utc>> {
        self.0
            .from_local_datetime(&local)
            .earliest()
            .or_else(|| GapInfo::new(&local, &self.0).and_then(|gap| gap.end))
            .map(|time| time.with_timezone(&Utc))
    }

    pub fn midnight(self, date: NaiveDate) -> DateTime<Utc> {
        self.resolve(date.and_hms_opt(0, 0, 0).unwrap())
            .expect("supported calendar date has a time-zone boundary")
    }

    pub fn shift_days(self, time: DateTime<Utc>, days: i64) -> Option<DateTime<Utc>> {
        self.resolve(
            self.at(time)
                .naive_local()
                .checked_add_signed(Duration::days(days))?,
        )
    }

    pub fn previous_year(self, time: DateTime<Utc>) -> Option<DateTime<Utc>> {
        self.resolve(
            self.at(time)
                .naive_local()
                .checked_sub_months(chrono::Months::new(12))?,
        )
    }
}

#[cfg(target_arch = "wasm32")]
pub fn browser_time_zone() -> Option<TimeZone> {
    let options = js_sys::Intl::DateTimeFormat::new(&js_sys::Array::new(), &js_sys::Object::new())
        .resolved_options();
    let name = js_sys::Reflect::get(&options, &"timeZone".into())
        .ok()?
        .as_string()?;
    TimeZone::from_name(&name)
}
