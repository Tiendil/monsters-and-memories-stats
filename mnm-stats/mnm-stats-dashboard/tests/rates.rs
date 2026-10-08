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
    ChartView {
        mode,
        window,
        ..Default::default()
    }
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

fn average(mode: RateMode) -> ChartView {
    ChartView {
        aggregation: Aggregation::Average24,
        ..view(mode, TrendWindow::Hours6)
    }
}

fn hourly(hours: i64, count: u64) -> History {
    history(&(0..=hours).map(|h| (h * 60, count)).collect::<Vec<_>>())
}

fn coverage(value: Option<MetricValue>) -> Duration {
    let Some(MetricValue::Average { covered, .. }) = value else {
        panic!("missing average")
    };
    covered
}

#[test]
fn weighted_average_uses_durations_and_clips_the_window_without_future_samples() {
    // A triangle lasting three hours has area 300 player-hours, regardless
    // of the uneven spacing of its observations.
    let mut points = vec![(0, 0), (60, 200), (180, 0)];
    points.extend((4..=24).map(|h| (h * 60, 0)));
    points.extend([(1530, 0), (1620, 100_000)]);
    let h = history(&points);
    let v = values(&h, &Metric::Online, &Scope::All, average(RateMode::Value));
    assert_eq!(v[0], None);
    let at_day = v.len() - 3;
    assert_number(v[at_day], 300.0 / 24.0);
    assert_eq!(coverage(v[at_day]), Duration::hours(24));
    // At 25:30, clipping the triangle at 01:30 leaves 112.5 player-hours.
    assert_number(v[at_day + 1], 112.5 / 24.0);
    let Some(MetricValue::Average {
        start, percentage, ..
    }) = v[at_day + 1]
    else {
        panic!()
    };
    assert!(!percentage);
    assert_eq!(start, h.snapshots()[0].observed_at + Duration::minutes(90));
    let mut future = h.snapshots().to_vec();
    future.last_mut().unwrap().servers[0].online = 1_000_000;
    let changed = values(
        &History::new(future).unwrap(),
        &Metric::Online,
        &Scope::All,
        average(RateMode::Value),
    );
    assert_eq!(v[..v.len() - 1], changed[..changed.len() - 1]);
}

#[test]
fn coverage_requires_eighteen_hours_and_rejects_long_intervals_before_clipping() {
    let h = hourly(18, 100);
    let v = values(&h, &Metric::Online, &Scope::All, average(RateMode::Value));
    assert!(v[..18].iter().all(Option::is_none));
    assert_number(v[18], 100.0);
    assert_eq!(coverage(v[18]), Duration::hours(18));
    let mut snapshots = h.snapshots().to_vec();
    snapshots[0].observed_at += Duration::seconds(1);
    let v = values(
        &History::new(snapshots).unwrap(),
        &Metric::Online,
        &Scope::All,
        average(RateMode::Value),
    );
    assert_eq!(v[18], None);
    // Exactly three hours is accepted.
    let h = history(&(0..=6).map(|i| (i * 180, 100)).collect::<Vec<_>>());
    assert_number(
        values(&h, &Metric::Online, &Scope::All, average(RateMode::Value))[6],
        100.0,
    );
    let mut too_long = h.snapshots().to_vec();
    too_long[1].observed_at += Duration::seconds(1);
    assert_eq!(
        values(
            &History::new(too_long).unwrap(),
            &Metric::Online,
            &Scope::All,
            average(RateMode::Value)
        )[6],
        None
    );
    assert_number(
        values(
            &hourly(18, 0),
            &Metric::Online,
            &Scope::All,
            average(RateMode::Value),
        )[18],
        0.0,
    );
    // Two five-hour gaps leave fourteen covered hours, despite many snapshots.
    let h = history(
        &(0..=24)
            .filter(|h| !(5..9).contains(h) && !(15..19).contains(h))
            .map(|h| (h * 60, 100))
            .collect::<Vec<_>>(),
    );
    assert_eq!(
        values(&h, &Metric::Online, &Scope::All, average(RateMode::Value)).last(),
        Some(&None)
    );
    // The long interval 0..5 remains excluded even when the cutoff clips it to 2..5.
    let h = history(
        &std::iter::once((0, 100))
            .chain((5..=26).map(|h| (h * 60, 100)))
            .collect::<Vec<_>>(),
    );
    let v = values(&h, &Metric::Online, &Scope::All, average(RateMode::Value));
    assert_eq!(coverage(*v.last().unwrap()), Duration::hours(21));
}

#[test]
fn gaps_are_excluded_from_both_area_and_duration() {
    let h = history(
        &(0..=24)
            .filter(|h| !(10..13).contains(h))
            .map(|h| (h * 60, if h < 10 { 100 } else { 300 }))
            .collect::<Vec<_>>(),
    );
    let v = values(&h, &Metric::Online, &Scope::All, average(RateMode::Value));
    let last = *v.last().unwrap();
    assert_eq!(coverage(last), Duration::hours(20));
    assert_number(last, (9.0 * 100.0 + 11.0 * 300.0) / 20.0);
    // No carrying a value across a long outage, or averaging an isolated point.
    let h = history(&[(0, 100), (1440, 100), (3000, 100)]);
    assert!(
        values(&h, &Metric::Online, &Scope::All, average(RateMode::Value))
            .iter()
            .all(Option::is_none)
    );
}

#[test]
fn rolling_ratios_average_snapshot_ratios_and_leave_other_metrics_unchanged() {
    let original = hourly(18, 20);
    let mut snapshots = original.snapshots().to_vec();
    snapshots[0].servers[0].online = 10;
    snapshots[0].servers[0].daily_active = 10;
    snapshots[0].servers[0].monthly_active = 10;
    snapshots[0].active_subscriptions = 10;
    for snapshot in &mut snapshots[1..] {
        snapshot.servers[0].daily_active = 100;
    }
    let h = History::new(snapshots).unwrap();
    for metric in [
        Metric::OnlineDaily,
        Metric::OnlineMonthly,
        Metric::OnlineSubscriptions,
        Metric::OnlineShare,
    ] {
        let v = values(
            &h,
            &metric,
            &Scope::Server("a".into()),
            average(RateMode::Value),
        );
        assert_number(
            v[18],
            if metric == Metric::OnlineDaily {
                (60.0 + 17.0 * 20.0) / 18.0
            } else {
                100.0
            },
        );
        assert!(matches!(
            v[18],
            Some(MetricValue::Average {
                percentage: true,
                ..
            })
        ));
    }
    for metric in [
        Metric::Daily,
        Metric::Monthly,
        Metric::Subscriptions,
        Metric::DailyMonthly,
        Metric::DailySubscriptions,
        Metric::MonthlySubscriptions,
    ] {
        for mode in RateMode::ALL {
            assert_eq!(
                values(&h, &metric, &Scope::All, average(mode)),
                values(&h, &metric, &Scope::All, view(mode, TrendWindow::Hours6))
            );
        }
    }
}

#[test]
fn rates_are_calculated_after_rolling_averages() {
    let h = history(
        &(0..=30)
            .map(|h| (h * 60, h as u64 * 10))
            .collect::<Vec<_>>(),
    );
    let changes = values(&h, &Metric::Online, &Scope::All, average(RateMode::Change));
    assert!(changes[..=18].iter().all(Option::is_none));
    assert_number(changes[19], 5.0); // Mean of the growing covered interval.
    assert_number(changes[25], 10.0); // Full 24-hour window moves one hour.
    let trends = values(&h, &Metric::Online, &Scope::All, average(RateMode::Trend));
    assert_eq!(trends[20], None);
    assert_number(trends[21], 5.0);
    assert_number(trends[30], 10.0);
}

#[test]
fn missing_values_exclude_adjacent_intervals_without_discarding_earlier_coverage() {
    let mut snapshots = hourly(24, 100).snapshots().to_vec();
    snapshots[20].servers[0].starting_zones.clear();
    snapshots[20].servers[0].daily_active = 0;
    let h = History::new(snapshots).unwrap();
    for metric in [Metric::Zone("z".into(), "Z".into()), Metric::OnlineDaily] {
        let v = values(&h, &metric, &Scope::All, average(RateMode::Value));
        assert_eq!(v[20], None);
        assert_number(v[21], 100.0);
        assert_eq!(coverage(v[21]), Duration::hours(19));
        assert_eq!(coverage(v[24]), Duration::hours(22));
    }
    // All Zones remains valid: a published empty list means zero.
    let v = values(
        &h,
        &Metric::StartingZones,
        &Scope::All,
        average(RateMode::Value),
    );
    assert_eq!(coverage(v[24]), Duration::hours(24));
    assert_number(v[24], 2300.0 / 24.0);
}

#[test]
fn contributor_changes_restart_totals_and_shares_but_not_unchanged_servers() {
    let mut snapshots = hourly(40, 100).snapshots().to_vec();
    for snapshot in &mut snapshots[20..] {
        let mut server = snapshot.servers[0].clone();
        server.id = "b".into();
        snapshot.servers.push(server);
    }
    let h = History::new(snapshots).unwrap();
    for (metric, scope, expected) in [
        (Metric::Online, Scope::All, 200.0),
        (Metric::OnlineShare, Scope::Server("a".into()), 50.0),
    ] {
        let v = values(&h, &metric, &scope, average(RateMode::Value));
        assert!(v[20..38].iter().all(Option::is_none));
        assert_number(v[38], expected);
    }
    let v = values(
        &h,
        &Metric::Online,
        &Scope::Server("a".into()),
        average(RateMode::Value),
    );
    assert_number(v[20], 100.0);
    assert_eq!(coverage(v[20]), Duration::hours(20));
}

#[test]
fn averaging_preserves_observation_positions_and_lookback_in_comparisons_and_timezones() {
    let h = history(
        &(0..72)
            .map(|hour| (hour * 60, hour as u64 * 10))
            .collect::<Vec<_>>(),
    );
    let now = h.snapshots().last().unwrap().observed_at;
    let range = TimeRange::custom("2024-03-31", "2024-03-31").unwrap();
    let computed = values(&h, &Metric::Online, &Scope::All, average(RateMode::Value));
    for zone in [TimeZone::UTC, TimeZone::from_name("Europe/Berlin").unwrap()] {
        for compare in [
            Comparison::None,
            comparison_for(
                &h,
                range,
                now,
                ComparisonMode::Previous,
                DateMatching::ExactDate,
                &[],
                zone,
            )
            .unwrap(),
        ] {
            let mut plotted = plot(
                &h,
                &Metric::Online,
                &[Scope::All],
                range,
                now,
                &compare,
                zone,
            )
            .unwrap();
            let before = plotted.clone();
            apply(&mut plotted, &h, average(RateMode::Value));
            assert_eq!(plotted.x_bounds, before.x_bounds);
            for (s, old) in plotted.series.iter().zip(before.series) {
                assert_eq!(s.identity, old.identity);
                assert_eq!(s.points.len(), old.points.len());
                for (point, old) in s.points.iter().zip(old.points) {
                    assert_eq!((point.at, point.x), (old.at, old.x));
                    let i = h
                        .snapshots()
                        .binary_search_by_key(&point.at, |s| s.observed_at)
                        .unwrap();
                    assert_eq!(point.value, computed[i]);
                }
            }
        }
    }
    assert_number(computed[24], 120.0); // Integral over hours 0..24, including lookback before the selected day.
}

#[test]
fn average_labels_only_distinguish_mixed_averaging_behavior() {
    let h = hourly(24, 100);
    let now = h.snapshots().last().unwrap().observed_at;
    let metrics = [
        Metric::OnlineDaily,
        Metric::OnlineMonthly,
        Metric::OnlineSubscriptions,
        Metric::DailySubscriptions,
        Metric::MonthlySubscriptions,
    ];
    let mut plotted = engagement_plot(
        &h,
        &metrics,
        &[Scope::All],
        TimeRange::All,
        now,
        &Comparison::None,
        TimeZone::UTC,
    )
    .unwrap();
    let original = plotted.clone();
    apply(&mut plotted, &h, average(RateMode::Value));
    for (series, old) in plotted.series.iter().zip(&original.series) {
        let expected = if series.metric.supports_average() {
            format!("{} (24-hour average) · All Servers", series.metric.title())
        } else {
            old.label.clone()
        };
        assert_eq!(plotted.series_label(series), expected);
        assert_eq!(original.series_label(old), old.label);
        assert_eq!(series.identity, old.identity);
        assert_eq!(series.style, old.style);
    }
    let figure: serde_json::Value = serde_json::from_str(
        &mnm_stats_dashboard::charts::render(&plotted, &Metric::OnlineDaily).to_json(),
    )
    .unwrap();
    assert_eq!(
        figure["data"][0]["name"],
        "Online / daily active (24-hour average) · All Servers"
    );
    assert!(
        figure["data"][0]["text"]
            .as_array()
            .unwrap()
            .iter()
            .any(|text| text
                .as_str()
                .unwrap_or_default()
                .contains("Online / daily active (24-hour average) · All Servers"))
    );

    // Removing unaveraged metrics restores the original labels, including when
    // several online ratios or comparison series remain selected.
    plotted.series.retain(|s| s.metric.supports_average());
    for series in &plotted.series {
        assert_eq!(plotted.series_label(series), series.label);
    }
    plotted
        .series
        .retain(|s| s.metric == Metric::OnlineSubscriptions);
    assert_eq!(
        plotted.series_label(&plotted.series[0]),
        plotted.series[0].label
    );
    for metric in [Metric::Online, Metric::StartingZones, Metric::OnlineShare] {
        let mut single = plot(
            &h,
            &metric,
            &[Scope::All],
            TimeRange::All,
            now,
            &Comparison::None,
            TimeZone::UTC,
        )
        .unwrap();
        apply(&mut single, &h, average(RateMode::Value));
        for series in &single.series {
            assert_eq!(single.series_label(series), series.label);
        }
    }
}
