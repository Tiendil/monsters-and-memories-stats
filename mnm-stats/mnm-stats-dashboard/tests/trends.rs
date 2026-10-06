use chrono::{DateTime, Duration, NaiveDate, Timelike, Utc};
use mnm_stats_dashboard::{analysis::Scope, time::TimeZone, trends::*};
use mnm_stats_model::{History, Server, Snapshot, StartingZone};

fn time(value: &str) -> DateTime<Utc> {
    value.parse().unwrap()
}

fn server(id: &str, population: u64) -> Server {
    Server {
        id: id.into(),
        name: format!("Server {id}"),
        online: population,
        daily_active: population * 3,
        monthly_active: population * 12,
        starting_zones: vec![StartingZone {
            id: "harbor".into(),
            name: "Harbor".into(),
            online: population / 10,
        }],
    }
}

fn history(
    days: i64,
    end: DateTime<Utc>,
    mut population: impl FnMut(i64, u32) -> Vec<Server>,
) -> History {
    History::new(
        (0..days * 24)
            .map(|hour| {
                let observed_at = end - Duration::hours(days * 24 - hour);
                Snapshot {
                    observed_at,
                    active_subscriptions: 1000,
                    servers: population(hour / 24, observed_at.hour()),
                }
            })
            .collect(),
    )
    .unwrap()
}

fn rank(
    history: &History,
    period: TrendPeriod,
    grouping: BusyGrouping,
    end: DateTime<Utc>,
) -> Trends {
    rankings(history, &[Scope::All], period, grouping, end, TimeZone::UTC)
}

#[test]
fn complete_periods_rank_absolute_growth_and_starting_area_population() {
    let end = time("2026-10-15T00:00:00Z");
    let mut samples = history(14, end, |day, _| {
        vec![
            server("a", if day < 7 { 100 } else { 150 }),
            server("b", if day < 7 { 200 } else { 100 }),
        ]
    })
    .snapshots()
    .to_vec();
    samples.push(Snapshot {
        observed_at: end,
        active_subscriptions: 1000,
        servers: vec![server("a", 99999)],
    });
    let history = History::new(samples).unwrap();
    let result = rank(
        &history,
        TrendPeriod::Week,
        BusyGrouping::AllDays,
        end + Duration::hours(12),
    );
    assert_eq!(
        result.current.label(TimeZone::UTC),
        "08 Oct 2026 – 14 Oct 2026"
    );
    assert_eq!(
        result.previous.label(TimeZone::UTC),
        "01 Oct 2026 – 07 Oct 2026"
    );
    assert_eq!(
        result
            .servers
            .iter()
            .map(|row| row.id.as_str())
            .collect::<Vec<_>>(),
        ["a", "b"]
    );
    assert_eq!(
        change(&result.servers[0].current, &result.servers[0].previous),
        Some(PopulationChange {
            count: 50.0,
            percent: Some(50.0)
        })
    );
    assert_eq!(
        change(&result.servers[1].current, &result.servers[1].previous),
        Some(PopulationChange {
            count: -100.0,
            percent: Some(-50.0)
        })
    );
    assert_eq!(result.areas[0].id, "a");
    assert_eq!(result.areas[0].current.value, Some(15.0));
    assert_eq!(result.servers[0].current.days, 7);
    assert_eq!(result.servers[0].current.observations, 168);
    let repeated = rankings(
        &history,
        &[Scope::All, Scope::Server("a".into()), Scope::All],
        TrendPeriod::Week,
        BusyGrouping::AllDays,
        end,
        TimeZone::UTC,
    );
    assert_eq!(result, repeated);
    let selected = rankings(
        &history,
        &[Scope::Server("b".into())],
        TrendPeriod::Week,
        BusyGrouping::AllDays,
        end,
        TimeZone::UTC,
    );
    assert_eq!(selected.servers.len(), 1);
    assert!(selected.areas.iter().all(|row| row.id == "b"));
    assert!(selected.hours.iter().all(|row| row.server_id == "b"));
    for scopes in [vec![], vec![Scope::Server("missing".into())]] {
        let empty = rankings(
            &history,
            &scopes,
            TrendPeriod::Week,
            BusyGrouping::AllDays,
            end,
            TimeZone::UTC,
        );
        assert!(empty.servers.is_empty() && empty.areas.is_empty() && empty.hours.is_empty());
    }
}

#[test]
fn starting_areas_sum_each_observation_before_medians_and_rank_servers_by_total() {
    let end = time("2026-10-15T00:00:00Z");
    let data = history(14, end, |day, hour| {
        let mut a = server("a", 1000);
        let value = if day < 7 { 100 } else { 150 };
        a.starting_zones[0].online = if hour % 3 == 0 { 0 } else { value };
        a.starting_zones.push(StartingZone {
            id: "hills".into(),
            name: "Hills".into(),
            online: if hour % 3 == 1 { 0 } else { value },
        });
        // Server online counts are equal; starting-area totals determine the order.
        let mut b = server("b", 1000);
        b.starting_zones[0].online = 120;
        vec![a, b]
    });
    let result = rank(&data, TrendPeriod::Week, BusyGrouping::AllDays, end);
    assert_eq!(result.areas.len(), 2);
    assert_eq!(result.areas[0].id, "a");
    assert_eq!(result.areas[1].id, "b");
    // Daily totals repeat 150,150,300: median 150, not 150+150.
    assert_eq!(result.areas[0].current.value, Some(150.0));
    assert_eq!(result.areas[0].previous.value, Some(100.0));
    assert_eq!(result.areas[0].current.observations, 168);
    assert_eq!(
        change(&result.areas[0].current, &result.areas[0].previous),
        Some(PopulationChange {
            count: 50.0,
            percent: Some(50.0)
        })
    );

    let wide = history(7, end, |_, _| {
        let mut a = server("a", 0);
        a.starting_zones[0].online = u64::MAX;
        a.starting_zones.push(StartingZone {
            id: "hills".into(),
            name: "Hills".into(),
            online: u64::MAX,
        });
        vec![a]
    });
    assert_eq!(
        rank(&wide, TrendPeriod::Week, BusyGrouping::AllDays, end).areas[0]
            .current
            .value,
        Some((u128::from(u64::MAX) * 2) as f64)
    );
}

#[test]
fn missing_entities_are_unavailable_and_zero_baselines_do_not_produce_infinity() {
    let end = time("2026-10-15T00:00:00Z");
    let data = history(14, end, |day, _| {
        let mut a = server("a", if day < 7 { 0 } else { 50 });
        if day >= 7 {
            a.starting_zones.clear();
        }
        if day < 7 {
            vec![a, server("b", 100)]
        } else {
            vec![a]
        }
    });
    let result = rank(&data, TrendPeriod::Week, BusyGrouping::AllDays, end);
    assert_eq!(
        change(&result.servers[0].current, &result.servers[0].previous),
        Some(PopulationChange {
            count: 50.0,
            percent: None
        })
    );
    assert_eq!(result.servers[1].current.value, None);
    assert_eq!(
        change(&result.servers[1].current, &result.servers[1].previous),
        None
    );
    assert_eq!(result.areas[0].current.value, Some(0.0)); // Present server, empty areas: zero total.
    assert_eq!(result.areas[1].current.value, None); // Absent server stays unavailable.
    assert_eq!(result.areas[0].previous.value, Some(0.0));
    let zero = history(14, end, |_, _| vec![server("a", 0)]);
    let result = rank(&zero, TrendPeriod::Week, BusyGrouping::AllDays, end);
    assert_eq!(result.servers[0].current.value, Some(0.0));
    assert_eq!(
        change(&result.servers[0].current, &result.servers[0].previous),
        Some(PopulationChange {
            count: 0.0,
            percent: None
        })
    );
    assert_eq!(result.hours.len(), 3);
    assert_eq!(result.hours[0].start_hour, 0);
}

#[test]
fn daily_medians_have_equal_weight_and_incomplete_coverage_cannot_win_a_ranking() {
    let end = time("2026-10-15T00:00:00Z");
    let data = history(7, end, |day, hour| {
        if day >= 4 || (day < 2 && hour >= 12) {
            return Vec::new();
        }
        vec![server("a", (day as u64 + 1) * 10)]
    });
    let result = rank(&data, TrendPeriod::Week, BusyGrouping::AllDays, end);
    assert_eq!(result.servers[0].current.value, Some(25.0)); // median of 10,20,30,40
    assert_eq!(result.servers[0].current.days, 4);
    assert_eq!(result.servers[0].current.observations, 72);
    assert_eq!(result.servers[0].previous.value, None);
    for (days, hours) in [(3, 24), (7, 11)] {
        let sparse = history(7, end, |day, hour| {
            if day < days && hour < hours {
                vec![server("a", 9000)]
            } else {
                Vec::new()
            }
        });
        assert_eq!(
            rank(&sparse, TrendPeriod::Week, BusyGrouping::AllDays, end).servers[0]
                .current
                .value,
            None
        );
    }
}

#[test]
fn repeated_samples_do_not_substitute_for_distinct_hours() {
    // Two different UTC hours can fall within one local hour in fractional zones.
    let zone = TimeZone::from_name("Asia/Kathmandu").unwrap();
    let end = zone.midnight(NaiveDate::from_ymd_opt(2026, 10, 15).unwrap());
    let samples = (0..7).rev().flat_map(|day| {
        let date = zone.at(end).date_naive() - Duration::days(day + 1);
        [1, 3, 5, 7, 9, 11].into_iter().flat_map(move |hour| {
            [40, 50].into_iter().map(move |minute| Snapshot {
                observed_at: zone
                    .resolve(date.and_hms_opt(hour, minute, 0).unwrap())
                    .unwrap(),
                active_subscriptions: 1000,
                servers: vec![server("a", 9000)],
            })
        })
    });
    let data = History::new(samples.collect()).unwrap();
    let result = rankings(
        &data,
        &[Scope::All],
        TrendPeriod::Week,
        BusyGrouping::AllDays,
        end,
        zone,
    );
    assert_eq!(result.servers[0].current.value, None);
    assert_eq!(result.areas[0].current.value, None);

    // The repeated autumn clock hour covers two elapsed hours for daily totals,
    // so a 25-hour date needs 13 covered hours.
    let zone = TimeZone::from_name("Europe/Berlin").unwrap();
    let end = time("2026-10-25T23:00:00Z");
    let samples = (0..7).rev().flat_map(|day| {
        let date = NaiveDate::from_ymd_opt(2026, 10, 25).unwrap() - Duration::days(day);
        let start = zone.midnight(date);
        let hours = if day == 0 { 13 } else { 12 };
        (0..hours).map(move |hour| Snapshot {
            observed_at: start + Duration::hours(hour),
            active_subscriptions: 1000,
            servers: vec![server("a", 100)],
        })
    });
    let data = History::new(samples.collect()).unwrap();
    let result = rankings(
        &data,
        &[Scope::All],
        TrendPeriod::Week,
        BusyGrouping::AllDays,
        end,
        zone,
    );
    assert_eq!(result.servers[0].current.days, 7);
    assert_eq!(result.servers[0].current.observations, 85);
}

#[test]
fn recurring_windows_rank_typical_activity_and_ignore_single_evening_spikes() {
    let end = time("2026-10-15T00:00:00Z");
    let data = history(14, end, |day, hour| {
        let base = match hour {
            6..=8 => 300,
            21..=23 => 200,
            12..=14 => 100,
            0..=2 if day == 13 => 9999,
            _ => 10,
        };
        vec![server("a", base + if day >= 7 { 20 } else { 0 })]
    });
    let result = rank(&data, TrendPeriod::Week, BusyGrouping::AllDays, end);
    assert_eq!(
        result
            .hours
            .iter()
            .map(|row| row.start_hour)
            .collect::<Vec<_>>(),
        [6, 21, 12]
    );
    assert_eq!(result.hours[0].current.value, Some(320.0));
    assert_eq!(
        change(&result.hours[0].current, &result.hours[0].previous)
            .unwrap()
            .count,
        20.0
    );
    assert!(
        rank(&data, TrendPeriod::Week, BusyGrouping::Weekday, end)
            .hours
            .is_empty()
    );
    let month = history(60, end, |_, hour| {
        vec![server(
            "a",
            if hour == 6 || hour == 7 || hour == 8 {
                500
            } else {
                100
            },
        )]
    });
    let ranked = rank(&month, TrendPeriod::Month, BusyGrouping::Weekday, end);
    assert_eq!(ranked.hours.len(), 3);
    assert_eq!(ranked.hours[0].weekday, Some(0));
    assert_eq!(ranked.hours[0].start_hour, 6);
    assert_eq!(ranked.hours[0].current.value, Some(500.0));
    assert_eq!(ranked.hours[0].current.days, 4);
}

#[test]
fn local_calendar_periods_follow_dst_and_windows_follow_local_clock_time() {
    let zone = TimeZone::from_name("Europe/Berlin").unwrap();
    let date = NaiveDate::from_ymd_opt(2026, 3, 30).unwrap();
    let end = zone.midnight(date);
    let data = history(15, end, |_, hour| {
        vec![server(
            "a",
            if (17..20).contains(&hour) { 500 } else { 100 },
        )]
    });
    let local = rankings(
        &data,
        &[Scope::All],
        TrendPeriod::Week,
        BusyGrouping::AllDays,
        end,
        zone,
    );
    assert_eq!(local.current.start, time("2026-03-22T23:00:00Z"));
    assert_eq!((local.current.end - local.current.start).num_hours(), 167);
    assert_eq!(local.servers[0].current.days, 7);
    assert_eq!(local.hours[0].start_hour, 18);
    let autumn_end = time("2026-10-26T00:00:00Z");
    let autumn = rankings(
        &History::default(),
        &[Scope::All],
        TrendPeriod::Week,
        BusyGrouping::AllDays,
        autumn_end,
        zone,
    );
    assert_eq!((autumn.current.end - autumn.current.start).num_hours(), 169);
    let fractional = TimeZone::from_name("Asia/Kathmandu").unwrap();
    let local = rankings(
        &data,
        &[Scope::All],
        TrendPeriod::Week,
        BusyGrouping::AllDays,
        end,
        fractional,
    );
    assert_eq!(fractional.at(local.current.end).hour(), 0);
    assert_eq!(local.current.end.minute(), 15);
}

#[test]
fn month_and_year_use_explicit_equal_day_counts() {
    let now = time("2024-03-01T11:00:00Z");
    for period in TrendPeriod::ALL {
        let result = rank(&History::default(), period, BusyGrouping::AllDays, now);
        assert_eq!(
            (result.current.end - result.current.start).num_days(),
            period.days() as i64
        );
        assert_eq!(
            (result.previous.end - result.previous.start).num_days(),
            period.days() as i64
        );
        assert_eq!(result.previous.end, result.current.start);
        assert_eq!(result.current.end, time("2024-03-01T00:00:00Z"));
    }
}
