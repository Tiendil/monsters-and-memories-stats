use mnm_stats_model::History;
use serde_json::{Value, json};

fn record() -> Value {
    json!({
        "schema_version": 1, "observed_at": "2026-02-28T23:17:12.345Z", "active_subscriptions": 40,
        "servers": [
            {"id": "new-server", "name": "New Server Ω", "daily_active": 12, "monthly_active": 30, "online": 5,
             "starting_zones": [{"id": "harbor", "name": "Harbor \"West\"", "online": 0}]},
            {"id": "a", "name": "A", "daily_active": 0, "monthly_active": 1, "online": 1,
             "starting_zones": [{"id": "harbor", "name": "Different display name", "online": 1}]}
        ]
    })
}

#[test]
fn empty_history() {
    let history = History::from_jsonl("").unwrap();
    assert!(history.snapshots().is_empty());
}

#[test]
fn preserves_every_snapshot_value() {
    let first = record();
    let mut second = first.clone();
    second["observed_at"] = json!("2026-03-01T01:03:00Z");
    second["servers"].as_array_mut().unwrap().remove(0);
    second["active_subscriptions"] = json!(u64::MAX);
    let input = format!("{first}\n{second}\n");
    let history = History::from_jsonl(&input).unwrap();
    let snapshots = serde_json::to_value(history.snapshots()).unwrap();
    let expected: Vec<_> = [first, second]
        .into_iter()
        .map(|mut v| {
            v.as_object_mut().unwrap().remove("schema_version");
            v
        })
        .collect();
    assert_eq!(snapshots, json!(expected));
    assert_eq!(
        history.snapshots()[0].servers[0].starting_zones[0].online,
        0
    );
}

#[test]
fn serialized_records_are_compact_terminated_and_ordered_by_identity() {
    let history = History::from_jsonl(&record().to_string()).unwrap();
    let snapshot = &history.snapshots()[0];
    let mut reordered = snapshot.clone();
    reordered.servers.reverse();
    reordered.servers[0]
        .starting_zones
        .push(mnm_stats_model::StartingZone {
            id: "before-harbor".into(),
            name: "Another zone".into(),
            online: 2,
        });
    let mut reordered_again = reordered.clone();
    reordered_again.servers.reverse();
    for server in &mut reordered_again.servers {
        server.starting_zones.reverse();
    }
    let line = reordered.to_jsonl_record().unwrap();
    assert_eq!(line, reordered_again.to_jsonl_record().unwrap());
    assert!(line.ends_with('\n'));
    assert_eq!(line.lines().count(), 1);
    let loaded = History::from_jsonl(&line).unwrap();
    assert_eq!(loaded.snapshots()[0].servers[0].id, "a");
    assert_eq!(
        loaded.snapshots()[0].servers[0].starting_zones[0].id,
        "before-harbor"
    );
    assert_eq!(snapshot.servers[0].id, "new-server"); // Serialization does not mutate source values.
}

#[test]
fn rejects_malformed_incomplete_unknown_and_blank_records() {
    for bad in [
        "\n".to_owned(),
        " \n".to_owned(),
        "{}".to_owned(),
        "{\"schema_version\":1".to_owned(),
        format!("{}\n\n", record()),
        format!("{}\n{{", record()),
        record().to_string().replacen(
            "\"schema_version\":1",
            "\"schema_version\":1,\"schema_version\":1",
            1,
        ),
    ] {
        assert!(History::from_jsonl(&bad).is_err(), "accepted {bad}");
    }
    for (field, value) in [
        ("schema_version", json!(2)),
        ("active_subscriptions", json!(-1)),
        ("active_subscriptions", json!(1.5)),
        ("active_subscriptions", json!("12")),
        ("active_subscriptions", Value::Null),
        ("unexpected", json!(1)),
    ] {
        let mut bad = record();
        bad[field] = value;
        assert!(
            History::from_jsonl(&bad.to_string()).is_err(),
            "accepted {bad}"
        );
    }
    for field in [
        "observed_at",
        "active_subscriptions",
        "servers",
        "schema_version",
    ] {
        let mut bad = record();
        bad.as_object_mut().unwrap().remove(field);
        assert!(
            History::from_jsonl(&bad.to_string()).is_err(),
            "missing {field}"
        );
    }
    let overflow = record().to_string().replace(
        "\"active_subscriptions\":40",
        "\"active_subscriptions\":18446744073709551616",
    );
    assert!(History::from_jsonl(&overflow).is_err());
}

#[test]
fn rejects_bad_counts_and_scoped_identities() {
    for field in ["daily_active", "monthly_active", "online"] {
        let mut bad = record();
        bad["servers"][0][field] = json!(-1);
        assert!(History::from_jsonl(&bad.to_string()).is_err());
    }
    let mut bad = record();
    bad["servers"][0]["starting_zones"][0]["online"] = json!(0.5);
    assert!(History::from_jsonl(&bad.to_string()).is_err());
    for scope in ["server", "zone"] {
        for field in ["id", "name"] {
            let mut bad = record();
            let entity = if scope == "server" {
                &mut bad["servers"][0]
            } else {
                &mut bad["servers"][0]["starting_zones"][0]
            };
            entity[field] = json!("  ");
            assert!(History::from_jsonl(&bad.to_string()).is_err());
        }
    }
    let mut bad = record();
    bad["servers"][1]["id"] = bad["servers"][0]["id"].clone();
    assert!(
        History::from_jsonl(&bad.to_string())
            .unwrap_err()
            .to_string()
            .contains("duplicate server")
    );
    let mut bad = record();
    let zone = bad["servers"][0]["starting_zones"][0].clone();
    bad["servers"][0]["starting_zones"]
        .as_array_mut()
        .unwrap()
        .push(zone);
    assert!(
        History::from_jsonl(&bad.to_string())
            .unwrap_err()
            .to_string()
            .contains("duplicate zone")
    );
}

#[test]
fn rejects_invalid_timestamps_order_and_repeated_utc_hours() {
    for timestamp in [
        "not a time",
        "2026-02-29T00:00:00Z",
        "2026-03-01T00:00:00",
        "2026-03-01T01:00:00+01:00",
    ] {
        let mut bad = record();
        bad["observed_at"] = json!(timestamp);
        assert!(
            History::from_jsonl(&bad.to_string()).is_err(),
            "accepted {timestamp}"
        );
    }
    for timestamp in [
        "2026-02-28T23:17:12.345Z",
        "2026-02-28T22:00:00Z",
        "2026-02-28T23:59:59Z",
    ] {
        let mut second = record();
        second["observed_at"] = json!(timestamp);
        assert!(
            History::from_jsonl(&format!("{}\n{second}\n", record())).is_err(),
            "accepted {timestamp}"
        );
    }
    let mut next = record();
    next["observed_at"] = json!("2026-03-01T00:00:00Z");
    assert_eq!(
        History::from_jsonl(&format!("{}\n{next}", record()))
            .unwrap()
            .snapshots()
            .len(),
        2
    );
}
