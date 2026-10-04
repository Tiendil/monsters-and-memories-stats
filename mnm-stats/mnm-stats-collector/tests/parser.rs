mod common;
use common::*;
use mnm_stats_collector::parser::parse_snapshot;
use scraper::{Html, Selector};

#[test]
fn extracts_current_values_and_source_names_without_importing_charts() {
    let snapshot = parse_snapshot(CONNECTED, time()).unwrap();
    assert_eq!(snapshot.active_subscriptions, 18_866);
    let expected = [
        ("estaire", 4156, 4387),
        ("kravvin", 1172, 1249),
        ("nunavoth", 745, 797),
        ("tilustra", 3211, 3300),
        ("trem", 5566, 6011),
        ("vespyra", 2003, 2106),
    ];
    assert_eq!(
        snapshot
            .servers
            .iter()
            .map(|s| (s.id.as_str(), s.daily_active, s.monthly_active))
            .collect::<Vec<_>>(),
        expected
    );
    assert_eq!(snapshot.servers.iter().map(|s| s.online).sum::<u64>(), 4488);
    let server = &snapshot.servers[1];
    assert_eq!(server.name, "Kravvin - God of Slaughter (NA PvP)");
    assert_eq!(
        server
            .starting_zones
            .iter()
            .map(|z| (z.id.as_str(), z.online))
            .collect::<Vec<_>>(),
        [
            ("ailvorith", 24),
            ("evergrove", 22),
            ("nightharbore", 54),
            ("nightharborw", 44),
            ("underdocks", 60)
        ]
    );
    assert_eq!(server.starting_zones[0].name, "Ail'vorith");
    assert_eq!(snapshot.observed_at, time());
    let serialized = snapshot.to_jsonl_record().unwrap();
    assert!(
        !serialized.contains("2026-10-01")
            && !serialized.contains("chart")
            && !serialized.contains("history")
    );
}

#[test]
fn missing_malformed_duplicated_or_renamed_fields_fail() {
    for (from, to) in [
        ("DAILY ACTIVE", "DAILY USERS"),
        ("MONTHLY ACTIVE", "MONTHLY USERS"),
        ("Total Online", "Currently Online"),
        (
            "\n                          Online\n",
            "\n                          Current Players\n",
        ),
        ("Active Subscriptions", "Subscribers"),
        ("Starting Zones", "New Players"),
        (
            "\n                          Players\n",
            "\n                          Accounts\n",
        ),
        ("1172", "-1"),
        ("1172", "1,172"),
        ("1172", "1.5"),
        ("1172", ""),
        ("4488 Total Online", "4489 Total Online"),
        ("starting-zone-total-kravvin", "missing-zone-total"),
        (
            "starting-zone-kravvin-ailvorith",
            "starting-zone-kravvin-unknown",
        ),
        ("server-metrics-kravvin", "server-metrics-"),
        ("server-metrics-kravvin", "missing-server-identity"),
        ("server-stats-kravvin", "missing-stats"),
        ("ccu-history-kravvin", "missing-current-chart"),
        ("data-server=\"kravvin\"", "data-server=\"another-server\""),
    ] {
        let input = CONNECTED.replacen(from, to, 1);
        assert!(input != CONNECTED, "test mutation absent: {from}");
        assert!(
            parse_snapshot(&input, time()).is_err(),
            "accepted changed {from}"
        );
    }
    let doc = Html::parse_document(CONNECTED);
    let card = doc
        .select(&Selector::parse("#server-metrics-kravvin").unwrap())
        .next()
        .unwrap()
        .html();
    let duplicate = CONNECTED.replace(&card, &format!("{card}{card}"));
    // scraper normalizes HTML; use a normalized document for this replacement.
    let normalized = doc.html();
    let duplicate = if duplicate == CONNECTED {
        normalized.replace(&card, &format!("{card}{card}"))
    } else {
        duplicate
    };
    assert!(parse_snapshot(&duplicate, time()).is_err());
    let zone = doc
        .select(&Selector::parse("#starting-zone-kravvin-ailvorith").unwrap())
        .next()
        .unwrap()
        .html();
    let duplicate = normalized.replace(&zone, &format!("{zone}{zone}"));
    assert!(parse_snapshot(&duplicate, time()).is_err());
    let total = doc
        .select(&Selector::parse("#starting-zone-total-kravvin").unwrap())
        .next()
        .unwrap()
        .html();
    let wrong_total = normalized.replace(&total, &total.replace("204", "205"));
    assert!(wrong_total != normalized, "zone-total mutation absent");
    assert!(parse_snapshot(&wrong_total, time()).is_err());
}

#[test]
fn cosmetic_changes_preserve_values_and_zero_is_not_missing() {
    let input = CONNECTED
        .replace("class=\"", "data-unused=\"cosmetic\" class=\"")
        .replace("1172", "0");
    let snapshot = parse_snapshot(&input, time()).unwrap();
    assert_eq!(
        snapshot
            .servers
            .iter()
            .find(|s| s.id == "kravvin")
            .unwrap()
            .daily_active,
        0
    );
}

#[test]
fn server_membership_and_order_are_discovered() {
    let doc = Html::parse_document(CONNECTED);
    let normalized = doc.html();
    let cards: Vec<_> = doc
        .select(&Selector::parse("[id^='server-metrics-']").unwrap())
        .map(|card| card.html())
        .collect();
    let mut reordered = normalized.clone();
    for (index, card) in cards.iter().enumerate() {
        reordered = reordered.replace(card, &format!("<!--server-{index}-->"));
    }
    for (index, card) in cards.iter().rev().enumerate() {
        reordered = reordered.replace(&format!("<!--server-{index}-->"), card);
    }
    assert!(reordered != normalized, "server reorder absent");
    assert_eq!(
        parse_snapshot(&reordered, time()).unwrap(),
        parse_snapshot(CONNECTED, time()).unwrap()
    );
    let card = doc
        .select(&Selector::parse("#server-metrics-kravvin").unwrap())
        .next()
        .unwrap()
        .html();
    let new_card = card.replace("kravvin", "new-server").replace(
        "Kravvin - God of Slaughter (NA PvP)",
        "A completely new name",
    );
    let input = normalized
        .replace(&card, &format!("{new_card}{card}"))
        .replace("4488 Total Online", "4822 Total Online");
    let snapshot = parse_snapshot(&input, time()).unwrap();
    assert_eq!(snapshot.servers.len(), 7);
    assert_eq!(
        snapshot
            .servers
            .iter()
            .find(|s| s.id == "new-server")
            .unwrap()
            .name,
        "A completely new name"
    );
    let input = normalized
        .replace(&card, "")
        .replace("4488 Total Online", "4154 Total Online");
    assert_eq!(parse_snapshot(&input, time()).unwrap().servers.len(), 5);
    let input = normalized
        .replace(&card, "")
        .replace("</body>", &format!("{card}</body>"));
    assert!(
        parse_snapshot(&input, time()).is_err(),
        "out-of-container server must not be counted"
    );
}
