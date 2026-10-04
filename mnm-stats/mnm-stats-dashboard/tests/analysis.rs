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
fn changing_zone_membership_preserves_gaps_totals_and_exported_rows() {
    let first = snapshot("2024-01-01T00:00:00Z", 1, 2, 3);
    let mut missing = snapshot("2024-01-01T01:00:00Z", 1, 2, 3);
    missing.servers[0].starting_zones.clear();
    let mut latest = snapshot("2024-01-01T02:00:00Z", 1, 2, 3);
    latest.servers[0].starting_zones.push(StartingZone {
        id: "new-zone".into(),
        name: "New zone".into(),
        online: 7,
    });
    let zone = Metric::Zone("z".into(), "Zone".into());
    let a = Scope::Server("a".into());
    assert_eq!(zone.value(&missing, &a), None);
    assert_eq!(zone.value(&missing, &Scope::All), None);
    assert_eq!(
        zone.value(&missing, &Scope::Server("b".into())),
        Some(MetricValue::Count(1))
    );
    assert_eq!(
        Metric::StartingZones.value(&missing, &a),
        Some(MetricValue::Count(0))
    );
    assert_eq!(
        Metric::StartingZones.value(&missing, &Scope::All),
        Some(MetricValue::Count(1))
    );
    assert_eq!(
        Metric::StartingZones.value(&latest, &Scope::All),
        Some(MetricValue::Count(11))
    );
    let history = History::new(vec![first, missing, latest]).unwrap();
    assert_eq!(
        zones(&history)
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["new-zone", "z"]
    );
    let chart = plot(
        &history,
        &zone,
        &a,
        TimeRange::All,
        time("2024-01-01T03:00:00Z"),
        &Comparison::Entities(vec![Scope::All, a.clone(), Scope::Server("b".into())]),
    )
    .unwrap();
    for series in &chart.series[..2] {
        assert_eq!(series.points[1].value, None);
        assert_eq!(series.segments().len(), 2);
    }
    assert_eq!(chart.series[2].segments().len(), 1);
    let exported: serde_json::Value = serde_json::from_str(&history.to_json().unwrap()).unwrap();
    assert_eq!(
        exported["snapshots"][1]["servers"][0]["starting_zones"],
        serde_json::json!([])
    );
    assert_eq!(
        exported["snapshots"][2]["servers"][0]["starting_zones"][1]["id"],
        "new-zone"
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
        identity: "test".into(),
        style: 0,
        label: "test".into(),
        points,
    };
    assert_eq!(
        series.segments().iter().map(Vec::len).collect::<Vec<_>>(),
        [2, 1, 1, 1]
    );
}

#[test]
fn plotly_keeps_zero_and_singleton_observations() {
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
    let figure: serde_json::Value =
        serde_json::from_str(&mnm_stats_dashboard::charts::render(&plot, &Metric::Daily).to_json())
            .unwrap();
    assert_eq!(figure["data"][0]["y"], serde_json::json!([0.0]));
    assert_eq!(figure["data"][0]["mode"], "lines+markers");
    assert!(
        figure["data"][0]["text"][0]
            .as_str()
            .unwrap()
            .contains("2024-01-01T00:00:00Z UTC")
    );
}

#[test]
fn plotly_preserves_gaps_original_dates_exact_values_and_literal_names() {
    let start = time("2024-02-01T00:00:00.123Z");
    let plot = Plot {
        series: vec![Series {
            identity: "test".into(),
            style: 0,
            label: "Alpha <island> & West".into(),
            points: vec![
                Point {
                    at: start,
                    x: 0.0,
                    value: Some(MetricValue::Count(9007199254740993)),
                },
                Point {
                    at: start + Duration::hours(1),
                    x: 3600.0,
                    value: None,
                },
                Point {
                    at: start + Duration::hours(5),
                    x: 18000.0,
                    value: Some(MetricValue::Count(7)),
                },
            ],
        }],
        alignment: Alignment::Month,
        x_bounds: (0.0, 86400.0),
        note: String::new(),
    };
    let figure: serde_json::Value =
        serde_json::from_str(&mnm_stats_dashboard::charts::render(&plot, &Metric::Daily).to_json())
            .unwrap();
    let series = &figure["data"][0];
    assert_eq!(series["connectgaps"], false);
    assert_eq!(series["x"], serde_json::json!([0.0, 3600.0, null, 18000.0]));
    assert!(series["y"][1].is_null() && series["y"][2].is_null());
    let text = series["text"][0].as_str().unwrap();
    assert!(text.contains("Alpha &lt;island&gt; &amp; West"));
    assert!(text.contains("2024-02-01T00:00:00.123Z UTC"));
    assert!(
        text.contains("9007199254740993"),
        "exact hover text must survive JS numeric rounding"
    );
    assert_eq!(series["text"][1], "");
    assert_eq!(series["text"][2], "");
}

#[test]
fn dense_plots_keep_every_observation_for_hover() {
    let start = time("2024-01-01T00:00:00Z");
    let plot = Plot {
        series: vec![Series {
            identity: "test".into(),
            style: 0,
            label: "Dense series".into(),
            points: (0..201)
                .map(|i| Point {
                    at: start + Duration::hours(i),
                    x: (i * 3600) as f64,
                    value: (i != 100).then_some(MetricValue::Count(i as u128)),
                })
                .collect(),
        }],
        alignment: Alignment::Elapsed,
        x_bounds: (0.0, 201.0 * 3600.0),
        note: String::new(),
    };
    let figure: serde_json::Value =
        serde_json::from_str(&mnm_stats_dashboard::charts::render(&plot, &Metric::Daily).to_json())
            .unwrap();
    let series = &figure["data"][0];
    assert_eq!(series["mode"], "lines");
    assert_eq!(series["x"].as_array().unwrap().len(), 201);
    assert_eq!(
        series["y"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|v| !v.is_null())
            .count(),
        200
    );
    assert!(series["y"][100].is_null());
    assert!(
        series["text"][200]
            .as_str()
            .unwrap()
            .contains("2024-01-09T08:00:00Z UTC<br>200")
    );
}

#[test]
fn dense_history_keeps_isolated_observations_visible() {
    let start = time("2024-01-01T00:00:00Z");
    let plot = Plot {
        series: vec![Series {
            identity: "test".into(),
            style: 0,
            label: "Sparse history".into(),
            points: (0..201)
                .map(|i| Point {
                    at: start + Duration::hours(i * 3),
                    x: (i * 10800) as f64,
                    value: Some(MetricValue::Count(i as u128)),
                })
                .collect(),
        }],
        alignment: Alignment::Elapsed,
        x_bounds: (0.0, 201.0 * 10800.0),
        note: String::new(),
    };
    let figure: serde_json::Value =
        serde_json::from_str(&mnm_stats_dashboard::charts::render(&plot, &Metric::Daily).to_json())
            .unwrap();
    let series = &figure["data"][0];
    assert_eq!(series["mode"], "lines+markers");
    let sizes = series["marker"]["size"].as_array().unwrap();
    assert_eq!(
        sizes.iter().filter(|n| n.as_u64().unwrap() > 0).count(),
        201
    );
    assert_eq!(
        series["x"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|x| x.is_null())
            .count(),
        200
    );
}

#[test]
fn headlines_use_one_in_range_snapshot_without_filling_absent_servers() {
    let earlier = snapshot("2026-05-01T00:00:00Z", 100, 200, 300);
    let mut latest = snapshot("2026-05-31T12:00:00Z", 1, 2, 0);
    latest.servers.remove(0);
    let future = snapshot("2026-06-02T00:00:00Z", 999, 999, 999);
    let history = History::new(vec![earlier, latest.clone(), future]).unwrap();
    let now = time("2026-06-01T12:00:00Z");
    let selected = latest_in_range(&history, TimeRange::All, now).unwrap();
    assert_eq!(selected, &latest);
    for metric in [Metric::Online, Metric::Daily, Metric::Monthly] {
        assert_eq!(metric.value(selected, &Scope::Server("a".into())), None);
    }
    assert_eq!(
        Metric::Subscriptions.value(selected, &Scope::Server("a".into())),
        Some(MetricValue::Count(0))
    );
    assert!(latest_in_range(&history, TimeRange::Days7, time("2026-07-01T00:00:00Z")).is_none());
    assert_eq!(
        grouped_count(u128::MAX),
        "340,282,366,920,938,463,463,374,607,431,768,211,455"
    );
    assert_eq!(grouped_count(0), "0");
}

#[test]
fn comparison_encodings_survive_other_selections_and_metric_changes() {
    use mnm_stats_dashboard::charts::{SeriesStyles, render};
    let history = History::new(vec![snapshot("2026-05-01T00:00:00Z", 1, 2, 3)]).unwrap();
    let now = time("2026-06-01T00:00:00Z");
    let periods: Vec<_> = (1..=7)
        .map(|month| Period::month(&format!("2026-{month:02}")).unwrap())
        .collect();
    let mut styles = SeriesStyles::default();
    let mut original = plot(
        &history,
        &Metric::Daily,
        &Scope::All,
        TimeRange::All,
        now,
        &Comparison::Periods(periods.clone()),
    )
    .unwrap();
    styles.assign(&mut original);
    let mut subset = plot(
        &history,
        &Metric::Monthly,
        &Scope::Server("a".into()),
        TimeRange::All,
        now,
        &Comparison::Periods(periods[2..].to_vec()),
    )
    .unwrap();
    styles.assign(&mut subset);
    for (expected, actual) in original.series[2..].iter().zip(&subset.series) {
        assert_eq!(expected.identity, actual.identity);
        assert_eq!(expected.style, actual.style);
    }
    let figure: serde_json::Value =
        serde_json::from_str(&render(&original, &Metric::Daily).to_json()).unwrap();
    let lines: Vec<_> = figure["data"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["line"].clone())
        .collect();
    assert_eq!(lines.len(), 7);
    for (i, line) in lines.iter().enumerate() {
        assert!(!lines[..i].contains(line));
    }
    assert_ne!(lines[0]["dash"], lines[1]["dash"]);
}
