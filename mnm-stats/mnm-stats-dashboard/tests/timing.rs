#[test]
fn stale_status_uses_elapsed_time_with_a_strict_three_hour_boundary() {
    let observed = chrono::DateTime::parse_from_rfc3339("2026-02-28T23:10:00Z")
        .unwrap()
        .to_utc();
    for seconds in [-1, 0, 10_799, 10_800] {
        assert!(!mnm_stats_dashboard::is_stale(
            observed,
            observed + chrono::Duration::seconds(seconds)
        ));
    }
    assert!(mnm_stats_dashboard::is_stale(
        observed,
        observed + chrono::Duration::seconds(10_801)
    ));
}
