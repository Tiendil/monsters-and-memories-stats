use chrono::{DateTime, Duration, Utc};
use mnm_stats_dashboard::analysis::*;
use mnm_stats_model::{History, Server, Snapshot, StartingZone};

fn time(s: &str) -> DateTime<Utc> {
    s.parse().unwrap()
}
fn snapshot(at: &str, daily: u64, monthly: u64, subscriptions: u64) -> Snapshot {
    Snapshot {
        observed_at: time(at),
        active_subscriptions: subscriptions,
        servers: vec![
            Server {
                id: "a".into(),
                name: "Alpha".into(),
                daily_active: daily,
                monthly_active: monthly,
                online: 7,
                starting_zones: vec![StartingZone {
                    id: "z".into(),
                    name: "Zone".into(),
                    online: 3,
                }],
            },
            Server {
                id: "b".into(),
                name: "Beta".into(),
                daily_active: 5,
                monthly_active: 10,
                online: 2,
                starting_zones: vec![StartingZone {
                    id: "z".into(),
                    name: "Zone".into(),
                    online: 1,
                }],
            },
        ],
    }
}
fn number(metric: Metric, s: &Snapshot, scope: &Scope) -> Option<f64> {
    metric.value(s, scope).map(|v| v.number())
}

#[test]
fn aggregation_ratios_and_missing_entities_preserve_their_semantics() {
    let s = snapshot("2024-02-29T12:00:00Z", 15, 10, 10);
    let a = Scope::Server("a".into());
    assert_eq!(number(Metric::Daily, &s, &Scope::All), Some(20.0));
    assert_eq!(number(Metric::Monthly, &s, &Scope::All), Some(20.0));
    assert_eq!(number(Metric::Online, &s, &Scope::All), Some(9.0));
    assert_eq!(number(Metric::StartingZones, &s, &Scope::All), Some(4.0));
    assert_eq!(
        number(Metric::Zone("z".into(), "Zone".into()), &s, &a),
        Some(3.0)
    );
    assert_eq!(number(Metric::DailyMonthly, &s, &a), Some(150.0));
    assert_eq!(
        number(Metric::DailySubscriptions, &s, &Scope::All),
        Some(200.0)
    );
    assert_eq!(number(Metric::MonthlySubscriptions, &s, &a), Some(100.0));
    assert_eq!(
        Metric::DailyMonthly.value(&s, &a).unwrap().display(),
        "150.00% (15 / 10)"
    );
    let mut zero = s.clone();
    zero.active_subscriptions = 0;
    zero.servers[0].monthly_active = 0;
    assert_eq!(number(Metric::DailyMonthly, &zero, &a), None);
    assert_eq!(number(Metric::DailySubscriptions, &zero, &a), None);
    assert_eq!(number(Metric::Monthly, &zero, &a), Some(0.0));
    let absent = Scope::Server("absent".into());
    assert_eq!(number(Metric::Daily, &s, &absent), None);
    assert_eq!(number(Metric::Subscriptions, &s, &absent), Some(10.0));
    assert_eq!(
        number(Metric::Zone("missing".into(), "?".into()), &s, &a),
        None
    );
    zero.servers[0].daily_active = u64::MAX;
    zero.servers[1].daily_active = u64::MAX;
    assert_eq!(
        Metric::Daily.value(&zero, &Scope::All).unwrap().display(),
        "36893488147419103230"
    );
}

#[test]
fn ranges_end_at_now_and_include_their_exact_utc_boundaries() {
    let now = time("2024-03-01T12:30:00Z");
    for (range, days) in TimeRange::ALL
        .into_iter()
        .take(5)
        .zip([7, 30, 90, 180, 365])
    {
        let start = now - Duration::days(days);
        let mut records = vec![];
        for at in [
            start - Duration::hours(1),
            start,
            now,
            now + Duration::hours(1),
        ] {
            records.push(snapshot(&at.to_rfc3339(), 1, 2, 3));
        }
        let history = History::new(records).unwrap();
        assert_eq!(range.bounds(&history, now), (start, now));
        let plot = plot(
            &history,
            &Metric::Daily,
            &Scope::All,
            range,
            now,
            &Comparison::None,
        )
        .unwrap();
        assert_eq!(
            plot.series[0]
                .points
                .iter()
                .map(|p| p.at)
                .collect::<Vec<_>>(),
            [start, now]
        );
    }
    let history = History::new(vec![snapshot("2020-01-01T00:00:00Z", 1, 2, 3)]).unwrap();
    assert_eq!(
        TimeRange::All.bounds(&history, now),
        (time("2020-01-01T00:00:00Z"), now)
    );
}

#[test]
fn correlation_uses_last_joint_observation_per_utc_day_and_reports_limits() {
    let start = time("2024-01-01T00:00:00Z");
    let end = time("2024-01-03T23:59:59Z");
    let mut records = vec![];
    for day in 1..=3 {
        records.push(snapshot(
            &format!("2024-01-{day:02}T01:00:00Z"),
            999,
            10,
            10,
        ));
        records.push(snapshot(
            &format!("2024-01-{day:02}T12:00:00Z"),
            day,
            4 - day,
            day * 2,
        ));
        let mut missing = snapshot(&format!("2024-01-{day:02}T23:00:00Z"), 888, 888, 888);
        missing.servers.remove(0);
        records.push(missing);
    }
    let history = History::new(records).unwrap();
    let a = Scope::Server("a".into());
    let negative = correlation(&history, &Metric::Daily, &Metric::Monthly, &a, start, end);
    assert_eq!(negative.paired_days, 3);
    assert!((negative.r.unwrap() + 1.0).abs() < 1e-12);
    let positive = correlation(
        &history,
        &Metric::Daily,
        &Metric::Subscriptions,
        &a,
        start,
        end,
    );
    assert_eq!(positive.paired_days, 3);
    assert!((positive.r.unwrap() - 1.0).abs() < 1e-12);
    let short = correlation(
        &history,
        &Metric::Daily,
        &Metric::Monthly,
        &a,
        start,
        time("2024-01-02T23:59:59Z"),
    );
    assert_eq!(
        short,
        Correlation {
            paired_days: 2,
            r: None
        }
    );
    let constant = correlation(
        &history,
        &Metric::Daily,
        &Metric::Monthly,
        &Scope::Server("b".into()),
        start,
        end,
    );
    assert_eq!(
        constant,
        Correlation {
            paired_days: 3,
            r: None
        }
    );
}

#[test]
fn calendar_alignment_preserves_leap_days_unequal_months_and_half_open_bounds() {
    let february = Period::month("2024-02").unwrap();
    let march = Period::month("2024-03").unwrap();
    assert_eq!(
        february.x(time("2024-02-29T12:30:00Z")),
        28.0 * 86400.0 + 45000.0
    );
    assert_eq!(
        march.x(time("2024-03-29T12:30:00Z")),
        february.x(time("2024-02-29T12:30:00Z"))
    );
    let leap = Period::year("2024").unwrap();
    let ordinary = Period::year("2023").unwrap();
    assert_eq!(
        leap.x(time("2024-03-01T00:00:00Z")),
        ordinary.x(time("2023-03-01T00:00:00Z"))
    );
    assert_eq!(
        ordinary.x(time("2023-03-01T00:00:00Z")) - ordinary.x(time("2023-02-28T23:00:00Z")),
        25.0 * 3600.0
    );
    let history = History::new(vec![
        snapshot("2024-02-01T00:00:00Z", 1, 2, 3),
        snapshot("2024-02-29T12:00:00Z", 1, 2, 3),
        snapshot("2024-03-01T00:00:00Z", 1, 2, 3),
        snapshot("2024-03-31T12:00:00Z", 1, 2, 3),
        snapshot("2024-04-30T12:00:00Z", 1, 2, 3),
    ])
    .unwrap();
    let plot = plot(
        &history,
        &Metric::Daily,
        &Scope::All,
        TimeRange::Days7,
        time("2026-01-01T00:00:00Z"),
        &Comparison::Periods(vec![february, march, Period::month("2024-04").unwrap()]),
    )
    .unwrap();
    assert_eq!(plot.series.len(), 3);
    assert_eq!(
        plot.series
            .iter()
            .map(|s| s.points.len())
            .collect::<Vec<_>>(),
        [2, 2, 1]
    );
    assert!(Period::month("2024-13").is_err());
    assert!(Period::year("999999").is_err());
    assert!(Period::interval("2024-01-01T00:00", 0).is_err());
}

#[test]
fn entity_and_interval_comparisons_allow_many_series_and_reject_mixed_intervals() {
    let mut missing = snapshot("2024-01-01T01:00:00Z", 1, 2, 3);
    missing.servers.remove(0);
    let history = History::new(vec![
        snapshot("2024-01-01T00:00:00Z", 1, 2, 3),
        missing,
        snapshot("2024-01-01T02:00:00Z", 1, 2, 3),
    ])
    .unwrap();
    let now = time("2024-01-01T03:00:00Z");
    let entities = Comparison::Entities(vec![
        Scope::All,
        Scope::Server("a".into()),
        Scope::Server("b".into()),
        Scope::Server("absent".into()),
    ]);
    let values = plot(
        &history,
        &Metric::Daily,
        &Scope::All,
        TimeRange::All,
        now,
        &entities,
    )
    .unwrap();
    assert_eq!(values.series.len(), 4);
    assert_eq!(values.series[1].segments().len(), 2);
    assert!(values.series[3].segments().is_empty());
    assert_eq!(
        plot(
            &history,
            &Metric::Subscriptions,
            &Scope::All,
            TimeRange::All,
            now,
            &entities
        )
        .unwrap()
        .series
        .len(),
        1
    );
    let periods: Vec<_> = (0..4)
        .map(|hour| Period::interval(&format!("2024-01-01T{hour:02}:00"), 1).unwrap())
        .collect();
    let values = plot(
        &history,
        &Metric::Daily,
        &Scope::All,
        TimeRange::All,
        now,
        &Comparison::Periods(periods),
    )
    .unwrap();
    assert_eq!(values.series.len(), 4);
    assert_eq!(values.series[2].points[0].x, 0.0);
    assert!(values.series[3].points.is_empty());
    let unequal = vec![
        Period::interval("2024-01-01T00:00", 1).unwrap(),
        Period::interval("2024-01-01T00:00", 2).unwrap(),
    ];
    assert!(
        plot(
            &history,
            &Metric::Daily,
            &Scope::All,
            TimeRange::All,
            now,
            &Comparison::Periods(unequal)
        )
        .is_err()
    );
    assert_eq!(servers(&history).len(), 2);
}

#[test]
fn lines_break_on_missing_values_long_intervals_and_absent_calendar_dates() {
    let points = [
        ("2023-02-28T21:00:00Z", Some(MetricValue::Count(0))),
        ("2023-02-28T23:00:00Z", Some(MetricValue::Count(1))),
        ("2023-03-01T00:00:00Z", Some(MetricValue::Count(2))),
        ("2023-03-01T01:00:00Z", None),
        ("2023-03-01T02:00:00Z", Some(MetricValue::Count(4))),
        ("2023-03-01T05:00:00Z", Some(MetricValue::Count(5))),
    ]
    .map(|(s, value)| {
        let at = time(s);
        Point {
            at,
            x: Period::Year(2023).x(at),
            value,
        }
    })
    .to_vec();
    let series = Series {
        label: "test".into(),
        points,
    };
    assert_eq!(
        series.segments().iter().map(Vec::len).collect::<Vec<_>>(),
        [2, 1, 1, 1]
    );
}

#[test]
fn svg_plots_zero_and_singleton_observations_without_inventing_lines() {
    let history = History::new(vec![snapshot("2024-01-01T00:00:00Z", 0, 0, 0)]).unwrap();
    let plot = plot(
        &history,
        &Metric::Daily,
        &Scope::Server("a".into()),
        TimeRange::All,
        time("2024-01-02T00:00:00Z"),
        &Comparison::None,
    )
    .unwrap();
    let svg = mnm_stats_dashboard::charts::svg(&plot, &Metric::Daily).unwrap();
    assert!(svg.contains("<svg"));
    assert!(svg.contains("<circle"));
    assert!(!svg.contains("NaN"));
}
