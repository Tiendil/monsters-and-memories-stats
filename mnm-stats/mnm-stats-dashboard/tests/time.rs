use chrono::{DateTime, Duration, Utc};
use mnm_stats_dashboard::{analysis::*, charts, time::TimeZone, view_state::ViewState};
use mnm_stats_model::{History, Server, Snapshot};

fn at(value: &str) -> DateTime<Utc> {
    value.parse().unwrap()
}
fn history(records: &[(&str, u64)]) -> History {
    History::new(
        records
            .iter()
            .map(|(time, online)| Snapshot {
                observed_at: at(time),
                active_subscriptions: 100,
                servers: vec![Server {
                    id: "a".into(),
                    name: "Alpha".into(),
                    daily_active: 40,
                    monthly_active: 80,
                    online: *online,
                    starting_zones: vec![],
                }],
            })
            .collect(),
    )
    .unwrap()
}
fn zone(name: &str) -> TimeZone {
    TimeZone::from_name(name).unwrap()
}

#[test]
fn timestamps_use_the_local_clock_and_zone_name_with_minute_precision() {
    let berlin = zone("Europe/Berlin");
    assert_eq!(
        berlin.timestamp(at("2026-01-01T23:30:42.123Z")),
        "02 Jan 2026, 00:30 Europe/Berlin"
    );
    assert_eq!(
        berlin.timestamp(at("2026-07-01T23:30:00Z")),
        "02 Jul 2026, 01:30 Europe/Berlin"
    );
    assert_eq!(
        zone("Asia/Kathmandu").timestamp(at("2026-01-01T18:30:00Z")),
        "02 Jan 2026, 00:15 Asia/Kathmandu"
    );
    assert_eq!(
        zone("America/Los_Angeles").timestamp(at("2026-07-01T02:30:00Z")),
        "30 Jun 2026, 19:30 America/Los_Angeles"
    );
    assert_eq!(
        TimeZone::UTC.timestamp(at("2026-01-01T23:30:42.123Z")),
        "01 Jan 2026, 23:30 UTC"
    );
    assert!(TimeZone::from_name("invalid/zone").is_none());
}

#[test]
fn local_days_have_calendar_boundaries_across_short_and_long_days() {
    let data = history(&[]);
    let la = zone("America/Los_Angeles");
    let now = at("2026-03-09T03:30:00Z");
    assert_eq!(
        TimeRange::Today.bounds(&data, now, la),
        (at("2026-03-08T08:00:00Z"), now)
    );
    assert_eq!(
        TimeRange::Today.bounds(&data, now, TimeZone::UTC),
        (at("2026-03-09T00:00:00Z"), now)
    );
    for (date, start, end, hours) in [
        (
            "2026-03-08",
            "2026-03-08T08:00:00Z",
            "2026-03-09T07:00:00Z",
            23,
        ),
        (
            "2026-11-01",
            "2026-11-01T07:00:00Z",
            "2026-11-02T08:00:00Z",
            25,
        ),
    ] {
        let expected = (at(start), at(end) - Duration::nanoseconds(1));
        assert_eq!(
            TimeRange::custom(date, date)
                .unwrap()
                .bounds(&data, now, la),
            expected
        );
        assert_eq!(
            TimeRange::Yesterday.bounds(&data, at(end) + Duration::hours(1), la),
            expected
        );
        assert_eq!(
            (expected.1 + Duration::nanoseconds(1) - expected.0).num_hours(),
            hours
        );
    }
    assert_eq!(
        TimeRange::Days7.bounds(&data, now, la),
        (now - Duration::days(7), now)
    );
}

#[test]
fn calendar_boundaries_resolve_midnight_gaps_and_repeated_hours() {
    let havana = zone("America/Havana");
    let midnight = |date: &str| date.parse().unwrap();
    assert_eq!(
        havana.midnight(midnight("2026-03-08")),
        at("2026-03-08T05:00:00Z")
    );
    assert_eq!(
        havana.midnight(midnight("2026-11-01")),
        at("2026-11-01T04:00:00Z")
    );
    let apia = zone("Pacific/Apia");
    assert_eq!(
        apia.midnight(midnight("2011-12-30")),
        apia.midnight(midnight("2011-12-31"))
    );
}

#[test]
fn heatmaps_rebucket_original_instants_and_preserve_repeated_hour_samples() {
    let la = zone("America/Los_Angeles");
    let data = history(&[("2026-03-08T09:30:00Z", 2), ("2026-03-08T10:30:00Z", 6)]);
    let maps = activity_heatmaps(
        &data,
        &[Scope::All],
        TimeRange::All,
        at("2026-03-09T12:00:00Z"),
        &Comparison::None,
        la,
    )
    .unwrap();
    assert_eq!(maps[0].cells[6][1].mean(), Some(2.0));
    assert_eq!(maps[0].cells[6][2].mean(), None);
    assert_eq!(maps[0].cells[6][3].mean(), Some(6.0));
    let data = history(&[("2026-11-01T08:30:00Z", 2), ("2026-11-01T09:30:00Z", 6)]);
    let maps = activity_heatmaps(
        &data,
        &[Scope::All],
        TimeRange::All,
        at("2026-11-02T12:00:00Z"),
        &Comparison::None,
        la,
    )
    .unwrap();
    assert_eq!(
        maps[0].cells[6][1],
        ActivityCell {
            total: 8,
            samples: 2
        }
    );
    assert_eq!(maps[0].cells[6][1].mean(), Some(4.0));
    let data = history(&[("2026-03-08T18:30:00Z", 9)]);
    let maps = activity_heatmaps(
        &data,
        &[Scope::All],
        TimeRange::custom("2026-03-09", "2026-03-09").unwrap(),
        at("2026-03-10T12:00:00Z"),
        &Comparison::None,
        zone("Asia/Kathmandu"),
    )
    .unwrap();
    assert_eq!(maps[0].cells[0][0].mean(), Some(9.0));
    assert_eq!(
        maps[0]
            .cells
            .iter()
            .flatten()
            .map(|c| c.samples)
            .sum::<usize>(),
        1
    );
}

#[test]
fn chronological_charts_preserve_instants_while_tooltips_use_zone_names() {
    let data = history(&[("2026-11-01T08:30:00Z", 2), ("2026-11-01T09:30:00Z", 6)]);
    let make = |tz| {
        plot(
            &data,
            &Metric::Online,
            &[Scope::All],
            TimeRange::All,
            at("2026-11-02T12:00:00Z"),
            &Comparison::None,
            tz,
        )
        .unwrap()
    };
    let utc = make(TimeZone::UTC);
    let local = make(zone("America/Los_Angeles"));
    assert_eq!(utc.series[0].points, local.series[0].points);
    let figure: serde_json::Value =
        serde_json::from_str(&charts::render(&local, &Metric::Online).to_json()).unwrap();
    let text = figure["data"][0]["text"].to_string();
    assert_eq!(
        text.matches("01 Nov 2026, 01:30 America/Los_Angeles")
            .count(),
        2
    );
    let original = data
        .snapshots()
        .iter()
        .map(|s| s.observed_at)
        .collect::<Vec<_>>();
    assert_eq!(
        local.series[0]
            .points
            .iter()
            .map(|p| p.at)
            .collect::<Vec<_>>(),
        original
    );
}

#[test]
fn comparisons_follow_local_calendar_dates_and_actual_elapsed_lengths() {
    let la = zone("America/Los_Angeles");
    let data = history(&[]);
    let resolve = |range, now, mode, matching| {
        let Comparison::Periods(periods) =
            comparison_for(&data, range, now, mode, matching, &[], la).unwrap()
        else {
            panic!("comparison periods")
        };
        periods
    };
    let periods = resolve(
        TimeRange::Today,
        at("2026-03-08T10:30:00Z"),
        ComparisonMode::Previous,
        DateMatching::ExactDate,
    );
    assert_eq!(
        periods[0].period.bounds(la).unwrap(),
        (
            at("2026-03-08T08:00:00Z"),
            at("2026-03-08T10:30:00Z") + Duration::nanoseconds(1)
        )
    );
    assert_eq!(
        periods[1].period.bounds(la).unwrap(),
        (
            at("2026-03-07T08:00:00Z"),
            at("2026-03-07T11:30:00Z") + Duration::nanoseconds(1)
        )
    );
    let periods = resolve(
        TimeRange::Yesterday,
        at("2026-03-09T18:00:00Z"),
        ComparisonMode::Previous,
        DateMatching::ExactDate,
    );
    let first = periods[0].period.bounds(la).unwrap();
    let previous = periods[1].period.bounds(la).unwrap();
    assert_eq!(first.1 - first.0, Duration::hours(23));
    assert_eq!(previous.1 - previous.0, Duration::hours(24));
    assert_eq!(previous.1, first.0);
    let range = TimeRange::custom("2026-03-08", "2026-03-08").unwrap();
    let yearly = resolve(
        range,
        at("2026-03-10T00:00:00Z"),
        ComparisonMode::YearOverYear,
        DateMatching::ExactDate,
    );
    assert_eq!(
        yearly[1].period.bounds(la).unwrap(),
        (at("2025-03-08T08:00:00Z"), at("2025-03-09T08:00:00Z"))
    );
    let weekday = resolve(
        range,
        at("2026-03-10T00:00:00Z"),
        ComparisonMode::YearOverYear,
        DateMatching::Weekday,
    );
    assert_eq!(
        weekday[1].period.bounds(la).unwrap(),
        (at("2025-03-09T08:00:00Z"), at("2025-03-10T07:00:00Z"))
    );
}

#[test]
fn calendar_alignment_uses_local_dates_without_drawing_backwards_across_a_clock_change() {
    let la = zone("America/Los_Angeles");
    let november = Period::custom("2026-11-01", "2026-11-30").unwrap();
    assert_eq!(november.x(at("2026-11-01T08:30:00Z"), la), 90.0 * 60.0);
    assert_eq!(november.x(at("2026-11-01T09:30:00Z"), la), 90.0 * 60.0);
    let data = history(&[("2026-11-01T08:30:00Z", 2), ("2026-11-01T09:30:00Z", 6)]);
    let plotted = plot(
        &data,
        &Metric::Online,
        &[Scope::All],
        TimeRange::All,
        at("2026-12-01T12:00:00Z"),
        &Comparison::periods(vec![november]),
        la,
    )
    .unwrap();
    assert_eq!(plotted.series[0].points.len(), 2);
    assert_eq!(plotted.series[0].segments().len(), 2);
    assert!(plotted.series[0].label.contains("America/Los_Angeles"));
}

#[test]
fn local_url_choice_keeps_fixed_calendar_dates_instead_of_host_dependent_instants() {
    let fragment = "#chart-activity-heatmap?tz=local&range=custom&from=2026-03-08&to=2026-03-08&compare=custom&period=2025-03-09%2F2025-03-09";
    let state = ViewState::from_fragment(fragment);
    assert_eq!(state.time_mode, mnm_stats_dashboard::time::TimeMode::Local);
    assert_eq!(
        state.fragment(),
        "#chart-activity-heatmap?range=custom&from=2026-03-08&to=2026-03-08&compare=custom&period=2025-03-09%2F2025-03-09"
    );
    assert_eq!(ViewState::from_fragment(&state.fragment()), state);
    assert_eq!(
        ViewState::from_fragment("#overview?tz=invalid"),
        ViewState::default()
    );
}

#[test]
fn switching_time_zone_keeps_custom_comparison_series_colors() {
    let data = history(&[("2026-03-08T09:30:00Z", 2)]);
    let mut styles = charts::SeriesStyles::default();
    let make = |tz| {
        let range = TimeRange::custom("2026-03-08", "2026-03-08").unwrap();
        let now = at("2026-03-10T12:00:00Z");
        let comparison = comparison_for(
            &data,
            range,
            now,
            ComparisonMode::Custom,
            DateMatching::ExactDate,
            &[Period::custom("2026-03-07", "2026-03-07").unwrap()],
            tz,
        )
        .unwrap();
        plot(
            &data,
            &Metric::Online,
            &[Scope::All],
            range,
            now,
            &comparison,
            tz,
        )
        .unwrap()
    };
    let mut utc = make(TimeZone::UTC);
    let mut local = make(zone("America/Los_Angeles"));
    styles.assign(&mut utc);
    styles.assign(&mut local);
    for (utc, local) in utc.series.iter().zip(&local.series) {
        assert_eq!(utc.style, local.style);
        assert_ne!(utc.label, local.label);
    }
}
