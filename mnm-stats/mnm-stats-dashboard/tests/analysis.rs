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
        "150.00%"
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
fn changing_zone_membership_preserves_gaps_totals_and_original_rows() {
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
        &[Scope::All, a.clone(), Scope::Server("b".into())],
        TimeRange::All,
        time("2024-01-01T03:00:00Z"),
        &Comparison::None,
    )
    .unwrap();
    for series in &chart.series[..2] {
        assert_eq!(series.points[1].value, None);
        assert_eq!(series.segments().len(), 2);
    }
    assert_eq!(chart.series[2].segments().len(), 1);
    let snapshots = serde_json::to_value(history.snapshots()).unwrap();
    assert_eq!(
        snapshots[1]["servers"][0]["starting_zones"],
        serde_json::json!([])
    );
    assert_eq!(
        snapshots[2]["servers"][0]["starting_zones"][1]["id"],
        "new-zone"
    );
}

#[test]
fn population_combines_zones_servers_and_periods_without_partial_totals() {
    let records = (2..=4)
        .map(|month| {
            let mut record = snapshot(&format!("2024-{month:02}-01T00:00:00Z"), 15, 25, 10);
            record.servers[0].starting_zones.push(StartingZone {
                id: "w".into(),
                name: "West".into(),
                online: 2,
            });
            record
        })
        .collect();
    let history = History::new(records).unwrap();
    let selected = [
        ZoneScope::All,
        ZoneScope::Zone("z".into()),
        ZoneScope::Zone("w".into()),
    ];
    let scopes = [Scope::All, Scope::Server("a".into())];
    let comparison = Comparison::periods(
        (2..=4)
            .map(|month| Period::month(&format!("2024-{month:02}")).unwrap())
            .collect(),
    );
    let now = time("2024-05-01T00:00:00Z");
    let chart = population_plot(
        &history,
        &selected,
        &scopes,
        TimeRange::All,
        now,
        &comparison,
    )
    .unwrap();
    assert_eq!(chart.series.len(), 18);
    assert_eq!(
        chart
            .series
            .iter()
            .map(|s| &s.identity)
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        18
    );
    assert_eq!(
        chart
            .series
            .iter()
            .map(|s| s.style)
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        18
    );
    for (zone, expected) in chart.series.chunks(6).zip([
        [Some(6.0), Some(5.0)],
        [Some(4.0), Some(3.0)],
        [None, Some(2.0)],
    ]) {
        for period in zone.chunks(2) {
            assert_eq!(
                period
                    .iter()
                    .map(|s| s.points[0].value.map(|v| v.number()))
                    .collect::<Vec<_>>(),
                expected
            );
            assert_eq!(period[0].points[0].x, period[1].points[0].x);
        }
    }
    assert_eq!(
        chart.series[0].label,
        "All Zones · All Servers · February 2024 (UTC)"
    );
    assert_eq!(chart.series[7].label, "Zone · Alpha · February 2024 (UTC)");
    assert_eq!(chart.series[13].label, "West · Alpha · February 2024 (UTC)");
    let single = population_plot(
        &history,
        &selected[1..],
        &scopes,
        TimeRange::All,
        now,
        &comparison,
    )
    .unwrap();
    assert_eq!(single.x_bounds, chart.x_bounds);
    assert_eq!(single.alignment, chart.alignment);
    assert!(
        population_plot(&history, &[], &scopes, TimeRange::All, now, &comparison)
            .unwrap()
            .series
            .is_empty()
    );
    assert!(
        population_plot(&history, &selected, &[], TimeRange::All, now, &comparison)
            .unwrap()
            .series
            .is_empty()
    );
}

#[test]
fn population_colors_survive_zone_reordering_and_renaming() {
    let now = time("2024-01-01T03:00:00Z");
    let mut records = vec![snapshot("2024-01-01T00:00:00Z", 1, 2, 3)];
    let selected = [ZoneScope::All, ZoneScope::Zone("z".into())];
    let scopes = [Scope::All, Scope::Server("a".into())];
    let history = History::new(records.clone()).unwrap();
    let mut original = population_plot(
        &history,
        &selected,
        &scopes,
        TimeRange::All,
        now,
        &Comparison::None,
    )
    .unwrap();
    let mut styles = mnm_stats_dashboard::charts::SeriesStyles::default();
    styles.assign(&mut original);
    for server in &mut records[0].servers {
        server.starting_zones[0].name = "New name".into();
    }
    let renamed = History::new(records).unwrap();
    for selection in [
        vec![selected[1].clone()],
        vec![selected[1].clone(), ZoneScope::All],
    ] {
        let mut changed = population_plot(
            &renamed,
            &selection,
            &scopes,
            TimeRange::All,
            now,
            &Comparison::None,
        )
        .unwrap();
        styles.assign(&mut changed);
        assert!(changed.series[0].label.starts_with("New name ·"));
        for series in changed.series {
            let before = original
                .series
                .iter()
                .find(|s| s.identity == series.identity)
                .unwrap();
            assert_eq!(series.style, before.style);
            assert_eq!(series.points, before.points);
        }
    }
    assert_eq!(
        ZoneScope::Zone("missing".into()).label(&zones(&history)),
        "missing"
    );
}

#[test]
fn ranges_end_at_now_and_include_their_exact_utc_boundaries() {
    let now = time("2024-03-01T12:30:00Z");
    for (range, days) in [
        TimeRange::Days7,
        TimeRange::Days30,
        TimeRange::Days90,
        TimeRange::Days180,
        TimeRange::Year,
    ]
    .into_iter()
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
            &[Scope::All],
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
        &[Scope::All],
        TimeRange::Days7,
        time("2026-01-01T00:00:00Z"),
        &Comparison::periods(vec![february, march, Period::month("2024-04").unwrap()]),
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
fn entity_and_interval_comparisons_allow_many_series_and_different_lengths() {
    let mut missing = snapshot("2024-01-01T01:00:00Z", 1, 2, 3);
    missing.servers.remove(0);
    let history = History::new(vec![
        snapshot("2024-01-01T00:00:00Z", 1, 2, 3),
        missing,
        snapshot("2024-01-01T02:00:00Z", 1, 2, 3),
    ])
    .unwrap();
    let now = time("2024-01-01T03:00:00Z");
    let entities = vec![
        Scope::All,
        Scope::Server("a".into()),
        Scope::Server("b".into()),
        Scope::Server("absent".into()),
    ];
    let values = plot(
        &history,
        &Metric::Daily,
        &entities,
        TimeRange::All,
        now,
        &Comparison::None,
    )
    .unwrap();
    assert_eq!(values.series.len(), 4);
    assert_eq!(
        values
            .series
            .iter()
            .map(|series| series.label.as_str())
            .collect::<Vec<_>>(),
        ["All Servers", "Alpha", "Beta", "absent"]
    );
    let mut names = servers(&history);
    names.insert("a".into(), "  ".into());
    assert_eq!(Scope::Server("a".into()).label(&names), "a");
    assert_eq!(Metric::Zone("z".into(), "".into()).title(), "z");
    assert_eq!(values.series[1].segments().len(), 2);
    assert!(values.series[3].segments().is_empty());
    assert_eq!(
        plot(
            &history,
            &Metric::Subscriptions,
            &entities,
            TimeRange::All,
            now,
            &Comparison::None
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
        &[Scope::All],
        TimeRange::All,
        now,
        &Comparison::periods(periods),
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
            &[Scope::All],
            TimeRange::All,
            now,
            &Comparison::periods(unequal)
        )
        .is_ok()
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
        ("2023-03-02T05:00:00Z", Some(MetricValue::Count(6))),
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
        [2, 1, 2, 1]
    );
}

#[test]
fn plotly_keeps_zero_and_singleton_observations() {
    let history = History::new(vec![snapshot("2024-01-01T00:00:00Z", 0, 0, 0)]).unwrap();
    let plot = plot(
        &history,
        &Metric::Daily,
        &[Scope::Server("a".into())],
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
    assert_eq!(
        figure["data"][0]["text"][0],
        "<b>0 Alpha</b><br>01 Jan 2024, 00:00 UTC"
    );
}

#[test]
fn plotly_preserves_gaps_original_dates_exact_values_and_literal_names() {
    let start = time("2024-02-01T12:34:56.123Z");
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
    assert_eq!(series["x"], serde_json::json!([0.0, 3600.0, 18000.0]));
    assert!(series["y"][1].is_null());
    assert_eq!(series["y"][2], 7.0);
    let text = series["text"][0].as_str().unwrap();
    assert_eq!(
        text, "<b>9007199254740993 Alpha &lt;island&gt; &amp; West</b><br>01 Feb 2024, 12:34 UTC",
        "hover keeps the exact value and literal name, with minute precision on the second line"
    );
    assert_eq!(
        text.matches("<br>").count(),
        1,
        "long names stay on one line"
    );
    assert_eq!(series["text"][1], "");
    assert_eq!(figure["data"].as_array().unwrap().len(), 1);
}

#[test]
fn plotly_styles_interval_boundaries_without_extra_hover_observations() {
    let start = time("2024-01-01T00:00:00Z");
    let intervals = [
        0, 10_799_999, 10_800_000, 86_399_999, 86_400_000, 86_400_001, 3_600_000, 3_600_000,
        3_600_000,
    ];
    let mut elapsed = 0_i64;
    let points: Vec<_> = intervals
        .into_iter()
        .enumerate()
        .map(|(i, interval)| {
            elapsed += interval;
            Point {
                at: start + Duration::milliseconds(elapsed),
                x: elapsed as f64 / 1000.0,
                value: (i != 7).then_some(MetricValue::Count(i as u128)),
            }
        })
        .collect();
    let x: Vec<_> = points.iter().map(|p| p.x).collect();
    let plot = Plot {
        series: vec![Series {
            identity: "intervals".into(),
            style: 0,
            label: "Intervals".into(),
            points,
        }],
        alignment: Alignment::Elapsed,
        x_bounds: (0.0, *x.last().unwrap()),
        note: String::new(),
    };
    let figure: serde_json::Value = serde_json::from_str(
        &mnm_stats_dashboard::charts::render(&plot, &Metric::Online).to_json(),
    )
    .unwrap();
    let traces = figure["data"].as_array().unwrap();
    let edges = |trace: &serde_json::Value| {
        trace["x"]
            .as_array()
            .unwrap()
            .windows(2)
            .zip(trace["y"].as_array().unwrap().windows(2))
            .filter_map(|(xs, ys)| {
                (ys.iter().all(serde_json::Value::is_number))
                    .then(|| Some((xs[0].as_f64()?, xs[1].as_f64()?)))
                    .flatten()
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(traces.len(), 2);
    assert_eq!(traces[0]["line"]["dash"], "solid");
    assert_eq!(traces[1]["line"]["dash"], "solid");
    assert_eq!(edges(&traces[0]), [(x[0], x[1]), (x[5], x[6])]);
    assert_eq!(edges(&traces[1]), [(x[1], x[2]), (x[2], x[3])]);
    assert_eq!(
        traces[1]["line"]["color"],
        mnm_stats_dashboard::tokens::T_CHART_SERIES_GAP_COLOR
    );
    assert_eq!(
        traces[1]["opacity"],
        mnm_stats_dashboard::tokens::T_CHART_SERIES_GAP_OPACITY
    );
    assert_ne!(traces[0]["line"]["color"], traces[1]["line"]["color"]);
    assert_eq!(traces[0]["line"]["width"], traces[1]["line"]["width"]);
    assert_eq!(traces[1]["hoverinfo"], "skip");
    assert_eq!(traces[1]["showlegend"], false);
    let values: Vec<_> = traces[0]["y"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|v| v.as_f64())
        .collect();
    assert_eq!(values, [0., 1., 2., 3., 4., 5., 6., 8.]);
    assert_eq!(
        plot.series[0]
            .segments()
            .iter()
            .map(Vec::len)
            .collect::<Vec<_>>(),
        [4, 1, 2, 1]
    );
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
    assert_eq!(
        series["text"][200],
        "<b>200 Dense series</b><br>09 Jan 2024, 08:00 UTC"
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
                    at: start + Duration::hours(i * 24),
                    x: (i * 86400) as f64,
                    value: Some(MetricValue::Count(i as u128)),
                })
                .collect(),
        }],
        alignment: Alignment::Elapsed,
        x_bounds: (0.0, 201.0 * 86400.0),
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
        &[Scope::All],
        TimeRange::All,
        now,
        &Comparison::periods(periods.clone()),
    )
    .unwrap();
    styles.assign(&mut original);
    let mut subset = plot(
        &history,
        &Metric::Monthly,
        &[Scope::All],
        TimeRange::All,
        now,
        &Comparison::periods(periods[2..].to_vec()),
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
        assert_eq!(line["dash"], "solid");
    }
}

#[test]
fn server_selection_applies_to_every_period_without_duplicating_global_counts() {
    let history = History::new(
        (2..=4)
            .map(|month| snapshot(&format!("2024-{month:02}-01T00:00:00Z"), 15, 25, 10))
            .collect(),
    )
    .unwrap();
    let scopes = [
        Scope::All,
        Scope::Server("a".into()),
        Scope::Server("b".into()),
    ];
    let comparison = Comparison::periods(
        (2..=4)
            .map(|month| Period::month(&format!("2024-{month:02}")).unwrap())
            .collect(),
    );
    let now = time("2024-05-01T00:00:00Z");
    let daily = plot(
        &history,
        &Metric::Daily,
        &scopes,
        TimeRange::All,
        now,
        &comparison,
    )
    .unwrap();
    assert_eq!(daily.series.len(), 9);
    let identities: std::collections::BTreeSet<_> =
        daily.series.iter().map(|s| &s.identity).collect();
    assert_eq!(identities.len(), 9);
    for series in daily.series.chunks(3) {
        let values: Vec<_> = series
            .iter()
            .map(|s| s.points[0].value.unwrap().number())
            .collect();
        assert_eq!(values, [20.0, 15.0, 5.0]);
        assert!(series[0].label.starts_with("All Servers ·"));
        assert!(series[1].label.starts_with("Alpha ·"));
    }
    let mut styles = mnm_stats_dashboard::charts::SeriesStyles::default();
    let mut original = daily;
    styles.assign(&mut original);
    let mut subset = plot(
        &history,
        &Metric::Daily,
        &scopes[1..],
        TimeRange::All,
        now,
        &comparison,
    )
    .unwrap();
    styles.assign(&mut subset);
    for series in &subset.series {
        assert_eq!(
            series.style,
            original
                .series
                .iter()
                .find(|s| s.identity == series.identity)
                .unwrap()
                .style
        );
    }
    for comparison in [Comparison::None, comparison] {
        let global = plot(
            &history,
            &Metric::Subscriptions,
            &scopes,
            TimeRange::All,
            now,
            &comparison,
        )
        .unwrap();
        assert_eq!(
            global.series.len(),
            if comparison == Comparison::None { 1 } else { 3 }
        );
        assert!(global.series.iter().all(|s| {
            s.points
                .iter()
                .all(|p| p.value == Some(MetricValue::Count(10)))
        }));
        for metric in [Metric::Daily, Metric::Subscriptions] {
            let empty = plot(&history, &metric, &[], TimeRange::All, now, &comparison).unwrap();
            assert!(empty.series.is_empty());
            assert!(empty.note.contains("Select servers"));
        }
    }
}

fn compared(
    range: TimeRange,
    mode: ComparisonMode,
    matching: DateMatching,
    custom: &[Period],
) -> Vec<ComparedPeriod> {
    let history = History::new(Vec::new()).unwrap();
    match comparison_for(
        &history,
        range,
        time("2026-06-01T12:00:00Z"),
        mode,
        matching,
        custom,
    )
    .unwrap()
    {
        Comparison::Periods(periods) => periods,
        Comparison::None => panic!("expected comparison periods"),
    }
}

#[test]
fn custom_ranges_include_whole_utc_dates_and_validate_before_selection() {
    let history = History::new(vec![
        snapshot("2024-02-29T00:00:00Z", 1, 10, 20),
        snapshot("2024-02-29T23:59:59.999999999Z", 2, 10, 20),
        snapshot("2024-03-01T00:00:00Z", 3, 10, 20),
    ])
    .unwrap();
    let range = TimeRange::custom("2024-02-29", "2024-02-29").unwrap();
    let plot = plot(
        &history,
        &Metric::Daily,
        &[Scope::All],
        range,
        time("2026-01-01T00:00:00Z"),
        &Comparison::None,
    )
    .unwrap();
    assert_eq!(plot.series[0].points.len(), 2);
    for (start, end) in [
        ("2023-02-29", "2023-03-01"),
        ("2024-03-01", "2024-02-29"),
        ("", "2024-03-01"),
        ("0000-01-01", "0001-01-01"),
    ] {
        assert!(TimeRange::custom(start, end).is_err());
        assert!(Period::custom(start, end).is_err());
    }
}

#[test]
fn previous_period_tracks_primary_range_and_weekdays_without_overlap() {
    use chrono::Datelike;
    let range = TimeRange::custom("2026-05-01", "2026-05-30").unwrap();
    let exact = compared(
        range,
        ComparisonMode::Previous,
        DateMatching::ExactDate,
        &[],
    );
    assert_eq!(
        exact[1].period.bounds().unwrap(),
        (time("2026-04-01T00:00:00Z"), time("2026-05-01T00:00:00Z"))
    );
    let weekdays = compared(range, ComparisonMode::Previous, DateMatching::Weekday, &[]);
    let (start, end) = weekdays[1].period.bounds().unwrap();
    assert_eq!(start, time("2026-03-27T00:00:00Z"));
    assert_eq!(start.weekday(), time("2026-05-01T00:00:00Z").weekday());
    assert_eq!(end - start, Duration::days(30));
    assert!(end <= weekdays[0].period.bounds().unwrap().0);
    let rolling = compared(
        TimeRange::Days7,
        ComparisonMode::Previous,
        DateMatching::Weekday,
        &[],
    );
    assert_eq!(
        rolling[1].period.bounds().unwrap(),
        (time("2026-05-18T12:00:00Z"), time("2026-05-25T12:00:00Z"))
    );
    let changed = compared(
        TimeRange::Days30,
        ComparisonMode::Previous,
        DateMatching::ExactDate,
        &[],
    );
    assert_eq!(
        changed[1].period.bounds().unwrap(),
        (time("2026-04-02T12:00:00Z"), time("2026-05-02T12:00:00Z"))
    );
}

#[test]
fn year_over_year_preserves_calendar_alignment_and_handles_leap_day_boundaries() {
    let range = TimeRange::custom("2024-02-28", "2024-03-01").unwrap();
    let periods = compared(
        range,
        ComparisonMode::YearOverYear,
        DateMatching::ExactDate,
        &[],
    );
    assert_eq!(
        periods[1].period.bounds().unwrap(),
        (time("2023-02-28T00:00:00Z"), time("2023-03-02T00:00:00Z"))
    );
    assert_eq!(
        periods[0].period.x(time("2024-03-01T12:00:00Z")),
        periods[1].period.x(time("2023-03-01T12:00:00Z"))
    );
    assert_eq!(
        periods[1].period.x(time("2023-03-01T00:00:00Z"))
            - periods[1].period.x(time("2023-02-28T00:00:00Z")),
        2.0 * 86400.0
    );
    for day in ["2024-02-28", "2024-02-29"] {
        let single = compared(
            TimeRange::custom(day, day).unwrap(),
            ComparisonMode::YearOverYear,
            DateMatching::ExactDate,
            &[],
        );
        assert_eq!(
            single[1].period.bounds().unwrap(),
            (time("2023-02-28T00:00:00Z"), time("2023-03-01T00:00:00Z"))
        );
    }
    let weekdays = compared(
        TimeRange::custom("2023-01-01", "2023-01-07").unwrap(),
        ComparisonMode::YearOverYear,
        DateMatching::Weekday,
        &[],
    );
    assert_eq!(
        weekdays[1].period.bounds().unwrap(),
        (time("2022-01-02T00:00:00Z"), time("2022-01-09T00:00:00Z"))
    );
    assert_eq!(weekdays[0].period.alignment(), Alignment::Elapsed);
}

#[test]
fn custom_comparisons_allow_different_lengths_and_keep_every_original_timestamp() {
    let range = TimeRange::custom("2024-02-01", "2024-02-29").unwrap();
    let custom = [
        Period::custom("2024-03-01", "2024-03-31").unwrap(),
        Period::custom("2024-04-01", "2024-04-30").unwrap(),
    ];
    let comparison = Comparison::Periods(compared(
        range,
        ComparisonMode::Custom,
        DateMatching::ExactDate,
        &custom,
    ));
    let history = History::new(vec![
        snapshot("2024-02-29T23:00:00Z", 1, 2, 3),
        snapshot("2024-03-31T23:00:00Z", 1, 2, 3),
        snapshot("2024-04-30T23:00:00Z", 1, 2, 3),
    ])
    .unwrap();
    let charts = plot(
        &history,
        &Metric::Online,
        &[Scope::All, Scope::Server("a".into())],
        range,
        time("2026-01-01T00:00:00Z"),
        &comparison,
    )
    .unwrap();
    assert_eq!(charts.alignment, Alignment::Month);
    assert_eq!(charts.series.len(), 6);
    assert_eq!(charts.series[2].points[0].at, time("2024-03-31T23:00:00Z"));
    let custom = [
        Period::custom("2024-03-01", "2024-03-02").unwrap(),
        Period::custom("2024-04-01", "2024-04-30").unwrap(),
    ];
    let comparison = Comparison::Periods(compared(
        range,
        ComparisonMode::Custom,
        DateMatching::ExactDate,
        &custom,
    ));
    let charts = plot(
        &history,
        &Metric::Subscriptions,
        &[Scope::All, Scope::Server("a".into())],
        range,
        time("2026-01-01T00:00:00Z"),
        &comparison,
    )
    .unwrap();
    assert_eq!(charts.alignment, Alignment::Elapsed);
    assert_eq!(charts.x_bounds, (0.0, 30.0 * 86400.0));
    assert_eq!(charts.series.len(), 3);
    assert!(charts.series[1].points.is_empty());
}

#[test]
fn comparison_disable_empty_custom_and_clock_updates_preserve_primary_selection_and_identity() {
    let history = History::new(vec![]).unwrap();
    let now = time("2026-06-01T00:00:00Z");
    for mode in [ComparisonMode::Disabled, ComparisonMode::Custom] {
        assert_eq!(
            comparison_for(
                &history,
                TimeRange::Days30,
                now,
                mode,
                DateMatching::ExactDate,
                &[]
            )
            .unwrap(),
            Comparison::None
        );
    }
    let mut styles = mnm_stats_dashboard::charts::SeriesStyles::default();
    let mut identities = None;
    for at in [now, now + Duration::minutes(1)] {
        let comparison = comparison_for(
            &history,
            TimeRange::Days30,
            at,
            ComparisonMode::Previous,
            DateMatching::ExactDate,
            &[],
        )
        .unwrap();
        let mut chart = plot(
            &history,
            &Metric::Online,
            &[Scope::All],
            TimeRange::Days30,
            at,
            &comparison,
        )
        .unwrap();
        styles.assign(&mut chart);
        let actual: Vec<_> = chart
            .series
            .into_iter()
            .map(|s| (s.identity, s.style))
            .collect();
        if let Some(expected) = &identities {
            assert_eq!(&actual, expected);
        } else {
            identities = Some(actual);
        }
    }
}

#[test]
fn cross_year_alignment_custom_weekdays_and_duplicate_ranges_remain_consistent() {
    use chrono::Datelike;
    let range = TimeRange::custom("2024-12-20", "2025-01-10").unwrap();
    let yearly = compared(
        range,
        ComparisonMode::YearOverYear,
        DateMatching::ExactDate,
        &[],
    );
    assert_eq!(
        yearly[0].period.x(time("2025-01-01T10:00:00Z")),
        yearly[1].period.x(time("2024-01-01T10:00:00Z"))
    );
    assert_eq!(
        Alignment::Year.tick(yearly[0].period.x(time("2025-01-01T00:00:00Z"))),
        "01 Jan"
    );
    let original = Period::custom("2024-11-01", "2024-11-10").unwrap();
    let custom = [
        original.clone(),
        original.clone(),
        Period::custom("2024-12-20", "2025-01-10").unwrap(),
    ];
    let exact = compared(
        range,
        ComparisonMode::Custom,
        DateMatching::ExactDate,
        &custom,
    );
    assert_eq!(
        exact.len(),
        2,
        "secondary duplicates and the primary range occur once"
    );
    let weekdays = compared(
        TimeRange::custom("2024-12-21", "2025-01-10").unwrap(),
        ComparisonMode::Custom,
        DateMatching::Weekday,
        std::slice::from_ref(&original),
    );
    let adjusted = weekdays[1].period.bounds().unwrap();
    assert_eq!(adjusted.0, time("2024-11-02T00:00:00Z"));
    assert_eq!(
        adjusted.0.weekday(),
        weekdays[0].period.bounds().unwrap().0.weekday()
    );
    assert_eq!(adjusted.1 - adjusted.0, Duration::days(10));
    assert_eq!(
        custom[0], original,
        "matching does not mutate custom input dates"
    );
}

#[test]
fn day_presets_use_utc_midnights_including_leap_days_and_year_rollover() {
    for (now, midnight, yesterday) in [
        (
            "2024-03-01T12:30:00Z",
            "2024-03-01T00:00:00Z",
            "2024-02-29T00:00:00Z",
        ),
        (
            "2025-01-01T00:00:00Z",
            "2025-01-01T00:00:00Z",
            "2024-12-31T00:00:00Z",
        ),
    ] {
        let (now, midnight, yesterday) = (time(now), time(midnight), time(yesterday));
        let last_yesterday = midnight - Duration::nanoseconds(1);
        let times: std::collections::BTreeSet<_> = [
            yesterday - Duration::nanoseconds(1),
            yesterday,
            last_yesterday,
            midnight,
            now,
            now + Duration::hours(1),
        ]
        .into_iter()
        .collect();
        let history = History::new(
            times
                .into_iter()
                .map(|at| snapshot(&at.to_rfc3339(), 1, 2, 3))
                .collect(),
        )
        .unwrap();
        assert_eq!(TimeRange::Today.bounds(&history, now), (midnight, now));
        assert_eq!(
            TimeRange::Yesterday.bounds(&history, now),
            (yesterday, last_yesterday)
        );
        for (range, expected) in [
            (TimeRange::Today, [midnight, now]),
            (TimeRange::Yesterday, [yesterday, last_yesterday]),
        ] {
            let actual = plot(
                &history,
                &Metric::Daily,
                &[Scope::All],
                range,
                now,
                &Comparison::None,
            )
            .unwrap();
            let expected: std::collections::BTreeSet<_> = expected.into_iter().collect();
            assert_eq!(
                actual.series[0]
                    .points
                    .iter()
                    .map(|p| p.at)
                    .collect::<std::collections::BTreeSet<_>>(),
                expected
            );
            assert_eq!(
                latest_in_range(&history, range, now).unwrap().observed_at,
                *expected.last().unwrap()
            );
        }
        let next_midnight = midnight + Duration::days(1);
        assert_eq!(
            TimeRange::Today.bounds(&history, next_midnight),
            (next_midnight, next_midnight)
        );
        assert_eq!(
            TimeRange::Yesterday.bounds(&history, next_midnight),
            (midnight, next_midnight - Duration::nanoseconds(1))
        );
    }
}

#[test]
fn day_presets_compare_equivalent_parts_of_prior_days_and_years() {
    for (range, matching, start, last) in [
        (
            TimeRange::Today,
            DateMatching::ExactDate,
            "2026-05-31T00:00:00Z",
            "2026-05-31T12:00:00Z",
        ),
        (
            TimeRange::Today,
            DateMatching::Weekday,
            "2026-05-25T00:00:00Z",
            "2026-05-25T12:00:00Z",
        ),
        (
            TimeRange::Yesterday,
            DateMatching::ExactDate,
            "2026-05-30T00:00:00Z",
            "2026-05-30T23:59:59.999999999Z",
        ),
        (
            TimeRange::Yesterday,
            DateMatching::Weekday,
            "2026-05-24T00:00:00Z",
            "2026-05-24T23:59:59.999999999Z",
        ),
    ] {
        let periods = compared(range, ComparisonMode::Previous, matching, &[]);
        assert_eq!(
            periods[1].period.bounds().unwrap(),
            (time(start), time(last) + Duration::nanoseconds(1))
        );
    }
    let yearly = compared(
        TimeRange::Today,
        ComparisonMode::YearOverYear,
        DateMatching::ExactDate,
        &[],
    );
    assert_eq!(
        yearly[1].period.bounds().unwrap(),
        (
            time("2025-06-01T00:00:00Z"),
            time("2025-06-01T12:00:00Z") + Duration::nanoseconds(1)
        )
    );
}

#[test]
fn population_share_uses_all_observed_servers_and_preserves_unavailability() {
    let first = snapshot("2024-02-05T10:00:00Z", 1, 2, 3);
    let mut zero = snapshot("2024-02-05T11:00:00Z", 1, 2, 3);
    for server in &mut zero.servers {
        server.online = 0;
    }
    let mut missing = snapshot("2024-02-05T12:00:00Z", 1, 2, 3);
    missing.servers.remove(0);
    let h = History::new(vec![first, zero, missing]).unwrap();
    let now = time("2024-04-01T00:00:00Z");
    let a = Scope::Server("a".into());
    let shares = plot(
        &h,
        &Metric::OnlineShare,
        std::slice::from_ref(&a),
        TimeRange::All,
        now,
        &Comparison::None,
    )
    .unwrap();
    assert_eq!(
        shares.series[0]
            .points
            .iter()
            .map(|p| p.value)
            .collect::<Vec<_>>(),
        [
            Some(MetricValue::Ratio {
                numerator: 7,
                denominator: 9
            }),
            None,
            None
        ]
    );
    let expanded = plot(
        &h,
        &Metric::OnlineShare,
        &[Scope::All, a],
        TimeRange::All,
        now,
        &Comparison::None,
    )
    .unwrap();
    assert_eq!(expanded.series.len(), 2);
    assert_eq!(expanded.series[0], shares.series[0]);
    assert_eq!(expanded.series[1].points[2].value.unwrap().number(), 100.0);
    let figure: serde_json::Value = serde_json::from_str(
        &mnm_stats_dashboard::charts::render(&expanded, &Metric::OnlineShare).to_json(),
    )
    .unwrap();
    assert_eq!(
        figure["layout"]["yaxis"]["range"],
        serde_json::json!([0.0, 100.0])
    );
    let comparison = Comparison::periods(
        ["2024-02", "2024-03", "2024-04"]
            .map(|m| Period::month(m).unwrap())
            .to_vec(),
    );
    let compared = plot(
        &h,
        &Metric::OnlineShare,
        &[Scope::All],
        TimeRange::All,
        now,
        &comparison,
    )
    .unwrap();
    assert_eq!(compared.series.len(), 6);
    assert!(
        plot(
            &h,
            &Metric::OnlineShare,
            &[],
            TimeRange::All,
            now,
            &comparison
        )
        .unwrap()
        .series
        .is_empty()
    );
    let mut one_zero = h.snapshots()[0].clone();
    one_zero.servers[0].online = 0;
    assert_eq!(
        Metric::OnlineShare.value(&one_zero, &Scope::Server("a".into())),
        Some(MetricValue::Ratio {
            numerator: 0,
            denominator: 2
        })
    );
}

#[test]
fn heatmap_means_count_only_available_samples_in_original_utc_buckets() {
    let mut records = vec![
        snapshot("2024-02-04T23:59:59.999Z", 1, 2, 3),
        snapshot("2024-02-05T00:00:00Z", 1, 2, 3),
        snapshot("2024-02-05T10:59:59Z", 1, 2, 3),
        snapshot("2024-02-05T11:00:00Z", 1, 2, 3),
        snapshot("2024-02-12T10:02:00Z", 1, 2, 3),
        snapshot("2024-02-19T10:00:00Z", 1, 2, 3),
        snapshot("2024-03-04T10:00:00Z", 1, 2, 3),
    ];
    records[3].servers[0].online = 0;
    records[4].servers[0].online = 13;
    records[5].servers.remove(0);
    records[6].servers[0].online = 30;
    let h = History::new(records).unwrap();
    let scopes = [
        Scope::All,
        Scope::Server("a".into()),
        Scope::Server("b".into()),
    ];
    let now = time("2024-05-01T00:00:00Z");
    let comparison = Comparison::periods(
        ["2024-02", "2024-03", "2024-04"]
            .map(|m| Period::month(m).unwrap())
            .to_vec(),
    );
    let maps = activity_heatmaps(&h, &scopes, TimeRange::Days7, now, &comparison).unwrap();
    assert_eq!(maps.len(), 9);
    let feb_a = &maps[1];
    assert_eq!(
        feb_a.cells[0][10],
        ActivityCell {
            total: 20,
            samples: 2
        }
    );
    assert_eq!(feb_a.cells[0][10].mean(), Some(10.0));
    assert_eq!(feb_a.cells[0][11].mean(), Some(0.0));
    assert_eq!(feb_a.cells[0][12].mean(), None);
    assert_eq!(feb_a.cells[6][23].mean(), Some(7.0));
    assert_eq!(feb_a.cells[0][0].mean(), Some(7.0));
    assert_eq!(
        maps[0].cells[0][10],
        ActivityCell {
            total: 26,
            samples: 3
        }
    );
    assert_eq!(maps[4].cells[0][10].mean(), Some(30.0));
    assert!(
        maps[6..]
            .iter()
            .flat_map(|m| m.cells.iter().flatten())
            .all(|cell| cell.mean().is_none())
    );
    assert_eq!(heatmap_bounds(&maps), Some((0.0, 32.0)));
    let fig: serde_json::Value = serde_json::from_str(
        &mnm_stats_dashboard::charts::render_heatmap(feb_a, heatmap_bounds(&maps).unwrap())
            .to_json(),
    )
    .unwrap();
    assert_eq!(fig["data"][0]["z"][0][10], 10.0);
    assert_eq!(fig["data"][0]["z"][0][11], 0.0);
    assert!(fig["data"][0]["z"][0][12].is_null());
    assert_eq!(fig["data"][0]["zmax"], 32.0);
    let colors = fig["data"][0]["colorscale"].as_array().unwrap();
    assert_eq!(colors.len(), 10);
    assert_eq!(colors[0], serde_json::json!([0.0, "rgba(0, 0, 4, 1)"]));
    assert_eq!(
        colors[9],
        serde_json::json!([1.0, "rgba(252, 255, 164, 1)"])
    );
    assert!(
        fig["data"][0]["text"][0][10]
            .as_str()
            .unwrap()
            .contains("2 records · sum 20")
    );
    assert_eq!(fig["data"][0]["hoverongaps"], false);
    let filtered = activity_heatmaps(
        &h,
        &[Scope::Server("a".into())],
        TimeRange::custom("2024-02-05", "2024-02-05").unwrap(),
        now,
        &Comparison::None,
    )
    .unwrap();
    assert_eq!(
        filtered[0].cells[0][10],
        ActivityCell {
            total: 7,
            samples: 1
        }
    );
    assert_eq!(filtered[0].cells[6][23].mean(), None);
    assert!(
        activity_heatmaps(&h, &[], TimeRange::All, now, &comparison)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn heatmap_bounds_follow_available_means_across_selected_panels() {
    let empty = ActivityHeatmap {
        label: "No data".into(),
        cells: [[ActivityCell::default(); 24]; 7],
    };
    assert_eq!(heatmap_bounds(&[]), None);
    assert_eq!(heatmap_bounds(std::slice::from_ref(&empty)), None);
    let mut first = empty.clone();
    first.cells[0][0] = ActivityCell {
        total: 840,
        samples: 2,
    };
    first.cells[0][1] = ActivityCell {
        total: 1056,
        samples: 2,
    };
    assert_eq!(
        heatmap_bounds(std::slice::from_ref(&first)),
        Some((420.0, 528.0))
    );
    let mut second = empty.clone();
    second.cells[6][23] = ActivityCell {
        total: 100,
        samples: 1,
    };
    let mut maps = [empty, first, second];
    assert_eq!(heatmap_bounds(&maps), Some((100.0, 528.0)));
    maps[2].cells[0][0] = ActivityCell {
        total: 0,
        samples: 1,
    };
    assert_eq!(heatmap_bounds(&maps), Some((0.0, 528.0)));
    assert_eq!(heatmap_bounds(&maps[1..2]), Some((420.0, 528.0)));
}

#[test]
fn heatmap_zero_cells_and_fractional_means_keep_distinct_color_bounds() {
    use mnm_stats_dashboard::charts::render_heatmap;
    let mut map = ActivityHeatmap {
        label: "<West> & friends — Eastern North American realm".into(),
        cells: [[ActivityCell::default(); 24]; 7],
    };
    map.cells[0][0] = ActivityCell {
        total: 0,
        samples: 1,
    };
    let zero: serde_json::Value =
        serde_json::from_str(&render_heatmap(&map, (0.0, 0.0)).to_json()).unwrap();
    assert_eq!(zero["data"][0]["z"][0][0], 0.0);
    assert!(zero["data"][0]["z"][0][1].is_null());
    assert!(zero["data"][0]["zmax"].as_f64().unwrap() > 0.0);
    assert_eq!(
        zero["data"][0]["text"][0][0],
        "<b>0.00 mean online</b><br>&lt;West&gt; &amp; friends — Eastern North American realm<br>Mon 00:00–01:00 UTC<br>1 record · sum 0",
        "heatmap names stay on one line while escaping source markup"
    );
    map.cells[0][1] = ActivityCell {
        total: 1,
        samples: 2,
    };
    let fractional: serde_json::Value = serde_json::from_str(
        &render_heatmap(&map, heatmap_bounds(std::slice::from_ref(&map)).unwrap()).to_json(),
    )
    .unwrap();
    assert_eq!(fractional["data"][0]["zmax"], 0.5);
}

#[test]
fn heatmap_scales_label_common_endpoints_with_fixed_point_numbers() {
    let map = ActivityHeatmap {
        label: "Small server with a larger comparison partner".into(),
        cells: [[ActivityCell {
            total: 1,
            samples: 2,
        }; 24]; 7],
    };
    for (bounds, ticks, format) in [
        ((0.0, 0.0), [0.0, 0.5, 1.0], ",.2~f"),
        ((0.0, 0.005), [0.0, 0.0025, 0.005], ",.4~f"),
        ((0.0, 0.5), [0.0, 0.25, 0.5], ",.2~f"),
        ((420.0, 528.0), [420.0, 474.0, 528.0], ",.2~f"),
        ((420.0, 420.0), [399.0, 420.0, 441.0], ",.2~f"),
        ((7_000.5, 7_500.5), [7_000.5, 7_250.5, 7_500.5], ",.2~f"),
        (
            (7_500.0, 7_500.001),
            [7_500.0, 7_500.000_5, 7_500.001],
            ",.5~f",
        ),
    ] {
        let figure: serde_json::Value = serde_json::from_str(
            &mnm_stats_dashboard::charts::render_heatmap(&map, bounds).to_json(),
        )
        .unwrap();
        let trace = &figure["data"][0];
        assert_eq!(trace["zmin"], ticks[0]);
        assert_eq!(trace["zmax"], ticks[2]);
        assert_eq!(trace["colorbar"]["tickvals"], serde_json::json!(ticks));
        assert_eq!(trace["colorbar"]["tickformat"], format);
    }
}

#[test]
fn online_ratios_use_each_snapshots_counts_and_preserve_gaps() {
    let mut samples = vec![
        snapshot("2024-02-28T23:00:00Z", 999, 999, 999),
        snapshot("2024-02-29T08:00:00Z", 40, 200, 0),
        snapshot("2024-02-29T09:00:00Z", 50, 0, 100),
        snapshot("2024-02-29T20:00:00Z", 100, 400, 200),
        snapshot("2024-03-01T00:00:00Z", 0, 500, 200),
        snapshot("2024-03-03T12:00:00Z", 100, 500, 200),
    ];
    for (sample, online) in samples.iter_mut().zip([900, 10, 20, 31, 0, 50]) {
        sample.servers[0].online = online;
    }
    samples[5].servers.remove(0);
    let history = History::new(samples).unwrap();
    let scope = Scope::Server("a".into());
    let range = TimeRange::custom("2024-02-29", "2024-03-03").unwrap();
    let now = time("2024-03-04T00:00:00Z");
    let result = |metric: Metric, scope: Scope, range| {
        plot(&history, &metric, &[scope], range, now, &Comparison::None).unwrap()
    };
    for (metric, expected) in [
        (
            Metric::OnlineDaily,
            [Some(25.0), Some(40.0), Some(31.0), None, None],
        ),
        (
            Metric::OnlineMonthly,
            [Some(5.0), None, Some(7.75), Some(0.0), None],
        ),
        (
            Metric::OnlineSubscriptions,
            [None, Some(20.0), Some(15.5), Some(0.0), None],
        ),
    ] {
        let plotted = result(metric, scope.clone(), range);
        let points = &plotted.series[0].points;
        assert_eq!(points.len(), 5, "each snapshot remains a separate point");
        for ((point, expected), snapshot) in
            points.iter().zip(expected).zip(&history.snapshots()[1..])
        {
            assert_eq!(point.at, snapshot.observed_at);
            assert_eq!(point.value.map(MetricValue::number), expected);
        }
    }
    let daily = result(Metric::OnlineDaily, scope.clone(), range);
    assert_eq!(
        daily.series[0].points[0].value,
        Some(MetricValue::Ratio {
            numerator: 10,
            denominator: 40
        })
    );
    let partial = result(
        Metric::OnlineDaily,
        scope,
        TimeRange::Custom {
            start: time("2024-02-29T12:00:00Z"),
            end: time("2024-02-29T21:00:00Z"),
        },
    );
    assert_eq!(partial.series[0].points.len(), 1);
    assert_eq!(partial.series[0].points[0].value.unwrap().number(), 31.0);
    let all = result(Metric::OnlineDaily, Scope::All, range);
    assert!((all.series[0].points[0].value.unwrap().number() - 100.0 * 12.0 / 45.0).abs() < 1e-10);
    let global = result(Metric::OnlineSubscriptions, Scope::All, range);
    assert_eq!(global.series[0].points[2].value.unwrap().number(), 16.5);
    let absent = result(Metric::OnlineMonthly, Scope::Server("gone".into()), range);
    assert!(
        absent.series[0]
            .points
            .iter()
            .all(|point| point.value.is_none())
    );
    let figure: serde_json::Value = serde_json::from_str(
        &mnm_stats_dashboard::charts::render(&daily, &Metric::OnlineDaily).to_json(),
    )
    .unwrap();
    assert_eq!(
        figure["data"][0]["text"][0], "<b>25.00% Alpha</b><br>29 Feb 2024, 08:00 UTC",
        "tooltips retain each snapshot's percentage and timestamp"
    );
}

#[test]
fn engagement_metrics_combine_scopes_and_periods_preserving_each_snapshot() {
    let mut records = Vec::new();
    for month in [1, 2, 3] {
        for (hour, online) in [(1, 10), (22, 30)] {
            let mut sample = snapshot(
                &format!("2024-{month:02}-01T{hour:02}:00:00Z"),
                100,
                400,
                200,
            );
            sample.servers[0].online = online;
            records.push(sample);
        }
    }
    let history = History::new(records).unwrap();
    let periods = Comparison::periods(
        ["2024-01", "2024-02", "2024-03"]
            .map(|m| Period::month(m).unwrap())
            .into(),
    );
    let scopes = [
        Scope::All,
        Scope::Server("a".into()),
        Scope::Server("b".into()),
    ];
    let metrics = [
        Metric::DailySubscriptions,
        Metric::MonthlySubscriptions,
        Metric::OnlineSubscriptions,
    ];
    let now = time("2024-04-01T00:00:00Z");
    let all = engagement_plot(&history, &metrics, &scopes, TimeRange::All, now, &periods).unwrap();
    assert_eq!(all.series.len(), 27);
    let mut identities = std::collections::BTreeSet::new();
    for series in &all.series {
        assert!(identities.insert(series.identity.clone()));
    }
    for series in &all.series[18..] {
        assert_eq!(series.points.len(), 2);
        assert_eq!(series.points[0].x, 3600.0);
        assert_eq!(series.points[1].x, 22.0 * 3600.0);
        assert!(series.label.contains("Online / global subscribers"));
    }
    let mut styles = mnm_stats_dashboard::charts::SeriesStyles::default();
    let mut all = all;
    styles.assign(&mut all);
    let mut subset = engagement_plot(
        &history,
        &metrics[2..],
        &scopes[1..2],
        TimeRange::All,
        now,
        &periods,
    )
    .unwrap();
    styles.assign(&mut subset);
    for series in &subset.series {
        let original = all
            .series
            .iter()
            .find(|s| s.identity == series.identity)
            .unwrap();
        assert_eq!(original.style, series.style);
        assert_eq!(series.points[0].value.unwrap().number(), 5.0);
        assert_eq!(series.points[1].value.unwrap().number(), 15.0);
    }
    for (metrics, scopes) in [(&[][..], &scopes[..]), (&metrics[..], &[][..])] {
        let empty =
            engagement_plot(&history, metrics, scopes, TimeRange::All, now, &periods).unwrap();
        assert!(empty.series.is_empty());
    }
}
