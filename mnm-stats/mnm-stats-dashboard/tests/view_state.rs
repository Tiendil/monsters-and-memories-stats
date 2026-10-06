use mnm_stats_dashboard::{analysis::*, view_state::*};

#[test]
fn simple_links_keep_defaults_and_chart_ownership() {
    for (target, section) in [
        ("overview", Section::Overview),
        ("chart-online", Section::Overview),
        ("chart-daily", Section::Overview),
        ("chart-monthly", Section::Overview),
        ("chart-subscriptions", Section::Overview),
        ("player-activity", Section::Population),
        ("chart-starting-zones", Section::Population),
        ("chart-online-share", Section::Population),
        ("chart-activity-heatmap", Section::Population),
        ("engagement", Section::Relationships),
        ("chart-daily-monthly", Section::Relationships),
        ("chart-online-presence", Section::Relationships),
        ("chart-subscriber-activity", Section::Relationships),
    ] {
        let state = ViewState::from_fragment(&format!("#{target}"));
        assert_eq!(state.section(), section);
        assert_eq!(
            state,
            ViewState {
                target: target.into(),
                ..ViewState::default()
            }
        );
        assert_eq!(state.fragment(), format!("#{target}"));
    }
    for fragment in ["", "#", "#unknown", "#content"] {
        assert_eq!(ViewState::from_fragment(fragment), ViewState::default());
    }
}

#[test]
fn complete_view_round_trips_without_losing_ids_or_inactive_settings() {
    let id = "unavailable & + # ? : / = , % 雪";
    let state = ViewState {
        target: "chart-online-presence".into(),
        scopes: vec![
            Scope::Server(id.into()),
            Scope::All,
            Scope::Server("all".into()),
        ],
        range: TimeRange::custom("2024-02-29", "2024-03-02").unwrap(),
        comparison: ComparisonMode::YearOverYear,
        matching: DateMatching::Weekday,
        periods: vec![
            Period::custom("2023-01-01", "2023-12-31").unwrap(),
            Period::custom("2024-02-01", "2024-02-29").unwrap(),
            Period::custom("2024-01-15", "2024-01-20").unwrap(),
        ],
        zones: vec![ZoneScope::Zone(id.into()), ZoneScope::All],
        online_metrics: vec![Metric::OnlineMonthly],
        subscriber_metrics: vec![Metric::OnlineSubscriptions, Metric::DailySubscriptions],
    };
    let fragment = state.fragment();
    assert!(fragment.contains("range=custom&from=2024-02-29&to=2024-03-02"));
    assert!(!fragment[1..].contains('#'));
    assert_eq!(ViewState::from_fragment(&fragment), state);
    assert_eq!(ViewState::from_fragment(&fragment).fragment(), fragment);
    assert_eq!(
        ViewState::from_fragment(&state.link("chart-starting-zones")),
        ViewState {
            target: "chart-starting-zones".into(),
            ..state
        }
    );
}

#[test]
fn empty_selections_are_distinct_from_omitted_defaults() {
    let state =
        ViewState::from_fragment("#overview?scope=&zone=&online-metric=&subscriber-metric=");
    assert!(state.scopes.is_empty());
    assert!(state.zones.is_empty());
    assert!(state.online_metrics.is_empty());
    assert!(state.subscriber_metrics.is_empty());
    assert_eq!(ViewState::from_fragment(&state.fragment()), state);
    assert!(!ViewState::from_fragment("#overview").scopes.is_empty());
}

#[test]
fn invalid_input_recovers_each_setting_independently() {
    let state = ViewState::from_fragment(concat!(
        "#missing?range=custom&from=2024-03-01&to=2024-02-29",
        "&scope=server%3Amissing&scope=server%3Amissing&scope=garbage",
        "&compare=previous&match=unknown&zone=unknown&online-metric=unknown",
        "&subscriber-metric=online-subscriptions&subscriber-metric=unknown",
        "&period=2024-02-01%2F2024-02-29&period=invalid",
        "&period=2024-02-01%2F2024-02-29&future-parameter=ignored"
    ));
    assert_eq!(state.target, "overview");
    assert_eq!(state.range, TimeRange::default());
    assert_eq!(state.scopes, [Scope::Server("missing".into())]);
    assert_eq!(state.comparison, ComparisonMode::Previous);
    assert_eq!(state.matching, DateMatching::ExactDate);
    assert_eq!(state.zones, [ZoneScope::All]);
    assert_eq!(state.online_metrics, ONLINE_METRICS);
    assert_eq!(state.subscriber_metrics, [Metric::OnlineSubscriptions]);
    assert_eq!(
        state.periods,
        [Period::custom("2024-02-01", "2024-02-29").unwrap()]
    );
    for query in [
        "range=custom&from=bad&to=2024-01-01",
        "range=custom",
        "range=missing",
    ] {
        assert_eq!(
            ViewState::from_fragment(&format!("#engagement?{query}&compare=custom")).range,
            TimeRange::default()
        );
    }
}

#[test]
fn presets_remain_relative_and_defaults_stay_short() {
    for range in TimeRange::ALL {
        let state = ViewState {
            range,
            ..ViewState::default()
        };
        let fragment = state.fragment();
        assert!(!fragment.contains("from="));
        assert!(!fragment.contains("to="));
        assert_eq!(ViewState::from_fragment(&fragment), state);
    }
    assert_eq!(ViewState::default().fragment(), "#overview");
    for comparison in ComparisonMode::ALL {
        let state = ViewState {
            comparison,
            ..ViewState::default()
        };
        assert_eq!(ViewState::from_fragment(&state.fragment()), state);
    }
}
