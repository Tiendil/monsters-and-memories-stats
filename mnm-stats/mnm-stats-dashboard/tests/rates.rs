use chrono::{Duration, TimeZone as _, Utc};
use mnm_stats_dashboard::{analysis::*, rates::*, time::TimeZone};
use mnm_stats_model::{History, Server, Snapshot, StartingZone};

fn history(points: &[(i64, u64)]) -> History {
    History::new(
        points
            .iter()
            .map(|&(minute, count)| Snapshot {
                observed_at: Utc.with_ymd_and_hms(2024, 3, 30, 0, 0, 0).unwrap()
                    + Duration::minutes(minute),
                active_subscriptions: count,
                servers: vec![Server {
                    id: "a".into(),
                    name: "A".into(),
                    daily_active: count,
                    monthly_active: count,
                    online: count,
                    starting_zones: vec![StartingZone {
                        id: "z".into(),
                        name: "Z".into(),
                        online: count,
                    }],
                }],
            })
            .collect(),
    )
    .unwrap()
}
fn view(mode: RateMode, window: TrendWindow) -> ChartView {
    ChartView { mode, window }
}
fn assert_number(value: Option<MetricValue>, expected: f64) {
    assert!(
        (value.unwrap().number() - expected).abs() < 1e-8,
        "{value:?}, expected {expected}"
    );
}

#[test]
fn consecutive_changes_use_elapsed_time_and_keep_negative_zero_and_gaps() {
    let h = history(&[
        (0, 1000),
        (90, 1060),
        (240, 1010),
        (1680, 1200),
        (1740, 1200),
    ]);
    let v = values(
        &h,
        &Metric::Online,
        &Scope::All,
        view(RateMode::Change, TrendWindow::Hours6),
    );
    assert_eq!(v[0], None);
    assert_number(v[1], 40.0);
    assert_number(v[2], -20.0);
    assert_eq!(v[3], None); // Exactly 24 hours is unavailable.
    assert_number(v[4], 0.0);
    let MetricValue::Rate { start, unit, .. } = v[1].unwrap() else {
        panic!()
    };
    assert_eq!(start, h.snapshots()[0].observed_at);
    assert_eq!(unit, RateUnit::PlayersHour);
    let v = values(
        &h,
        &Metric::Monthly,
        &Scope::All,
        view(RateMode::Change, TrendWindow::Hours6),
    );
    assert_number(v[1], 960.0); // Same actual slope, expressed per day.
}

#[test]
fn linear_trends_are_exact_on_irregular_samples_for_all_windows() {
    for window in TrendWindow::ALL {
        let points: Vec<_> = (0..=window.hours())
            .map(|hour| {
                let minute = hour * 60 + if hour % 2 == 0 { 0 } else { 15 };
                (minute, 1_000_000 + minute as u64 * 2)
            })
            .collect();
        let h = history(&points);
        let v = values(
            &h,
            &Metric::Monthly,
            &Scope::All,
            view(RateMode::Trend, window),
        );
        assert_number(*v.last().unwrap(), 2880.0);
        assert_eq!(v[0], None);
    }
}

#[test]
fn regression_balances_rise_and_fall_and_requires_spread_and_coverage() {
    let h = history(&[(0, 1000), (60, 1500), (120, 2000), (180, 1500), (240, 1000)]);
    let v = values(
        &h,
        &Metric::Online,
        &Scope::All,
        view(RateMode::Trend, TrendWindow::Hours6),
    );
    assert_number(v[4], 0.0);
    assert!(v[..3].iter().all(Option::is_none));
    assert!(
        values(
            &h,
            &Metric::Online,
            &Scope::All,
            view(RateMode::Trend, TrendWindow::Hours24)
        )
        .iter()
        .all(Option::is_none)
    );
    let gaps = history(&[
        (0, 100),
        (60, 110),
        (120, 120),
        (180, 130),
        (480, 180),
        (540, 190),
    ]);
    assert!(
        values(
            &gaps,
            &Metric::Online,
            &Scope::All,
            view(RateMode::Trend, TrendWindow::Hours6)
        )[4..]
            .iter()
            .all(Option::is_none)
    );
    let too_sparse = history(&[(0, 1), (120, 2), (360, 3)]);
    assert_eq!(
        *values(
            &too_sparse,
            &Metric::Online,
            &Scope::All,
            view(RateMode::Trend, TrendWindow::Hours6)
        )
        .last()
        .unwrap(),
        None
    );
}

#[test]
fn missing_entities_and_changing_contributors_never_create_growth() {
    let original = history(&[(0, 100), (60, 100), (120, 100), (180, 100)]);
    let mut snapshots = original.snapshots().to_vec();
    let mut added = snapshots[0].servers[0].clone();
    added.id = "b".into();
    snapshots[1].servers.push(added);
    let h = History::new(snapshots).unwrap();
    let v = values(
        &h,
        &Metric::Online,
        &Scope::All,
        view(RateMode::Change, TrendWindow::Hours6),
    );
    assert_eq!(v[1], None);
    assert_eq!(v[2], None);
    assert_number(v[3], 0.0);
    let v = values(
        &h,
        &Metric::Online,
        &Scope::Server("a".into()),
        view(RateMode::Change, TrendWindow::Hours6),
    );
    assert_number(v[1], 0.0);
    let v = values(
        &h,
        &Metric::OnlineShare,
        &Scope::Server("a".into()),
        view(RateMode::Change, TrendWindow::Hours6),
    );
    assert_eq!(v[1], None);
    assert_eq!(v[2], None);
    let mut snapshots = original.snapshots().to_vec();
    snapshots[1].servers[0].id = "b".into();
    let h = History::new(snapshots).unwrap();
    let v = values(
        &h,
        &Metric::Online,
        &Scope::Server("a".into()),
        view(RateMode::Change, TrendWindow::Hours6),
    );
    assert_eq!(v[1], None);
    assert_eq!(v[2], None);
    assert_number(v[3], 0.0);
    let mut snapshots = original.snapshots().to_vec();
    snapshots[1].servers[0].starting_zones.clear();
    let h = History::new(snapshots).unwrap();
    let v = values(
        &h,
        &Metric::Zone("z".into(), "Z".into()),
        &Scope::All,
        view(RateMode::Change, TrendWindow::Hours6),
    );
    assert_eq!(v[1], None);
    assert_eq!(v[2], None);
}

#[test]
fn starting_zone_totals_keep_rates_when_zone_membership_changes() {
    let original = history(
        &(0..=8)
            .map(|h| (h * 60, 100 + h as u64 * 10))
            .collect::<Vec<_>>(),
    );
    let mut snapshots = original.snapshots().to_vec();
    for (i, snapshot) in snapshots.iter_mut().enumerate() {
        let server = &mut snapshot.servers[0];
        let total = server.starting_zones[0].online;
        // A zone disappears and reappears while the combined total keeps growing.
        server.starting_zones.clear();
        let extra = if i == 4 { 0 } else { 10 };
        if extra != 0 {
            server.starting_zones.push(StartingZone {
                id: "z".into(),
                name: "Z".into(),
                online: extra,
            });
        }
        server.starting_zones.push(StartingZone {
            id: "other".into(),
            name: "Other".into(),
            online: total - extra,
        });
    }
    let h = History::new(snapshots).unwrap();
    for scope in [Scope::All, Scope::Server("a".into())] {
        for mode in [RateMode::Change, RateMode::Trend] {
            let v = values(
                &h,
                &Metric::StartingZones,
                &scope,
                view(mode, TrendWindow::Hours6),
            );
            for value in &v[3..] {
                assert_number(*value, 10.0);
            }
            let individual = values(
                &h,
                &Metric::Zone("z".into(), "Z".into()),
                &scope,
                view(mode, TrendWindow::Hours6),
            );
            assert_eq!(individual[4], None);
            assert_eq!(individual[5], None);
            assert_number(individual[8], 0.0);
        }
    }
}

#[test]
fn empty_zone_lists_are_zero_totals_but_missing_servers_break_rates() {
    let original = history(&[(0, 100), (60, 100), (120, 100), (180, 100)]);
    let mut snapshots = original.snapshots().to_vec();
    snapshots[1].servers[0].starting_zones.clear();
    let h = History::new(snapshots.clone()).unwrap();
    let settings = view(RateMode::Change, TrendWindow::Hours6);
    let v = values(&h, &Metric::StartingZones, &Scope::All, settings);
    assert_number(v[1], -100.0);
    assert_number(v[2], 100.0);
    snapshots[1].servers[0].id = "b".into();
    let h = History::new(snapshots).unwrap();
    for scope in [Scope::All, Scope::Server("a".into())] {
        let v = values(&h, &Metric::StartingZones, &scope, settings);
        assert_eq!(v[1], None);
        assert_eq!(v[2], None);
        assert_number(v[3], 0.0);
    }
}

#[test]
fn ratio_rates_are_percentage_points_and_zero_denominators_break_history() {
    let original = history(&[(0, 200), (60, 230), (120, 240), (180, 250), (240, 260)]);
    let mut snapshots = original.snapshots().to_vec();
    for s in &mut snapshots {
        s.servers[0].monthly_active = 1000;
    }
    snapshots[2].servers[0].monthly_active = 0;
    let h = History::new(snapshots).unwrap();
    let v = values(
        &h,
        &Metric::DailyMonthly,
        &Scope::All,
        view(RateMode::Change, TrendWindow::Hours24),
    );
    assert_number(v[1], 72.0);
    assert_eq!(v[2], None);
    assert_eq!(v[3], None);
    assert_number(v[4], 24.0);
    let MetricValue::Rate { unit, .. } = v[1].unwrap() else {
        panic!()
    };
    assert_eq!(unit, RateUnit::PointsDay);
}

#[test]
fn huge_count_baselines_retain_small_changes() {
    let base = u64::MAX - 100;
    let h = history(&[(0, base), (60, base + 1), (120, base + 2), (180, base + 3)]);
    for mode in [RateMode::Change, RateMode::Trend] {
        assert_number(
            *values(
                &h,
                &Metric::Online,
                &Scope::All,
                view(mode, TrendWindow::Hours6),
            )
            .last()
            .unwrap(),
            1.0,
        );
    }
}

#[test]
fn plotting_keeps_lookback_comparisons_timezones_and_series_identity() {
    let points: Vec<_> = (0..48)
        .map(|hour| (hour * 60, 2000 - hour as u64 * 10))
        .collect();
    let h = history(&points);
    let now = h.snapshots().last().unwrap().observed_at;
    let range = TimeRange::custom("2024-03-31", "2024-03-31").unwrap();
    let settings = view(RateMode::Trend, TrendWindow::Hours6);
    let mut normal = plot(
        &h,
        &Metric::Online,
        &[Scope::All],
        range,
        now,
        &Comparison::None,
        TimeZone::UTC,
    )
    .unwrap();
    let before = normal.series[0].clone();
    apply(&mut normal, &h, settings);
    assert_eq!(normal.series[0].identity, before.identity);
    assert_number(normal.series[0].points[0].value, -10.0); // Uses observations before the displayed range.
    let comparison = comparison_for(
        &h,
        range,
        now,
        ComparisonMode::Previous,
        DateMatching::ExactDate,
        &[],
        TimeZone::UTC,
    )
    .unwrap();
    let mut compared = plot(
        &h,
        &Metric::Online,
        &[Scope::All],
        range,
        now,
        &comparison,
        TimeZone::UTC,
    )
    .unwrap();
    apply(&mut compared, &h, settings);
    for series in &compared.series {
        for point in &series.points {
            if point.value.is_some() {
                assert_number(point.value, -10.0);
            }
        }
    }
    assert_eq!(
        normal.series[0].points[0].value,
        compared.series[0].points[0].value
    );
    // The local date crosses the spring DST transition: use elapsed time,
    // not the apparent distance between local clock labels.
    let mut local = plot(
        &h,
        &Metric::Online,
        &[Scope::All],
        range,
        now,
        &Comparison::None,
        TimeZone::from_name("Europe/Berlin").unwrap(),
    )
    .unwrap();
    apply(&mut local, &h, settings);
    for point in &local.series[0].points {
        assert_number(point.value, -10.0);
        if let Some(utc) = normal.series[0]
            .points
            .iter()
            .find(|utc| utc.at == point.at)
        {
            assert_eq!(point.value, utc.value);
        }
    }
    let fig: serde_json::Value = serde_json::from_str(
        &mnm_stats_dashboard::charts::render(&normal, &Metric::Online).to_json(),
    )
    .unwrap();
    assert!(fig["layout"]["yaxis"]["range"][0].as_f64().unwrap() < 0.0);
    assert_eq!(fig["layout"]["yaxis"]["zeroline"], true);
    assert_eq!(fig["layout"]["yaxis"]["title"]["text"], "Players/hour");
    assert!(fig["data"][0]["text"][0].as_str().unwrap().contains(" – "));
    // A future spike cannot change an earlier trailing estimate.
    let old = normal.series[0].points[0].value;
    let mut snapshots = h.snapshots().to_vec();
    snapshots.last_mut().unwrap().servers[0].online = 1_000_000;
    apply(&mut normal, &History::new(snapshots).unwrap(), settings);
    assert_eq!(normal.series[0].points[0].value, old);
}
