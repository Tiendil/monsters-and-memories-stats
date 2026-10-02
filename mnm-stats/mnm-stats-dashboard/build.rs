use mnm_stats_model::History;
use std::{env, fmt::Write, fs, path::PathBuf};

fn main() {
    println!("cargo:rerun-if-env-changed=MNM_STATS_DEMO");
    println!(
        "cargo:rustc-env=MNM_STATS_DEMO={}",
        env::var("MNM_STATS_DEMO").unwrap_or_default()
    );
    println!("cargo:rerun-if-env-changed=MNM_STATS_HISTORY");
    let input = env::var_os("MNM_STATS_HISTORY").map_or_else(
        || {
            PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap())
                .join("../../data/history.jsonl")
        },
        PathBuf::from,
    );
    println!("cargo:rerun-if-changed={}", input.display());
    let source = fs::read_to_string(&input)
        .unwrap_or_else(|e| panic!("cannot read history {}: {e}", input.display()));
    let history = History::from_jsonl(&source)
        .unwrap_or_else(|e| panic!("invalid history {}: {e}", input.display()));

    // Compile typed values, not a JSON string requiring browser-side parsing.
    // Debug formatting of strings emits escaped Rust literals, including quotes
    // and newlines in source display names.
    let mut code = String::from(
        "pub fn embedded_history() -> mnm_stats_model::History {\nmnm_stats_model::History::new(vec![\n",
    );
    for snapshot in history.snapshots() {
        writeln!(code, "mnm_stats_model::Snapshot {{ observed_at: chrono::DateTime::from_timestamp({}, {}).expect(\"validated timestamp\"), active_subscriptions: {}, servers: vec![",
            snapshot.observed_at.timestamp(), snapshot.observed_at.timestamp_subsec_nanos(), snapshot.active_subscriptions).unwrap();
        for server in &snapshot.servers {
            writeln!(code, "mnm_stats_model::Server {{ id: {:?}.into(), name: {:?}.into(), daily_active: {}, monthly_active: {}, online: {}, starting_zones: vec![",
                server.id, server.name, server.daily_active, server.monthly_active, server.online).unwrap();
            for zone in &server.starting_zones {
                writeln!(code, "mnm_stats_model::StartingZone {{ id: {:?}.into(), name: {:?}.into(), online: {} }},", zone.id, zone.name, zone.online).unwrap();
            }
            code.push_str("] },\n");
        }
        code.push_str("] },\n");
    }
    code.push_str("]).expect(\"build-validated history\")\n}\n");
    fs::write(
        PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("history.rs"),
        code,
    )
    .unwrap();
}
