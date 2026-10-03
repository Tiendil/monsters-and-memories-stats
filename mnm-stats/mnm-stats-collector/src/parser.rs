use crate::Result;
use chrono::{DateTime, Utc};
use mnm_stats_model::{Server, Snapshot, StartingZone};
use scraper::{Element, ElementRef, Html, Selector};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

const ZONES: [&str; 5] = [
    "ailvorith",
    "evergrove",
    "nightharbore",
    "nightharborw",
    "underdocks",
];

fn select<'a>(root: ElementRef<'a>, selector: &str) -> Vec<ElementRef<'a>> {
    root.select(&Selector::parse(selector).expect("constant CSS selector"))
        .collect()
}

pub(crate) fn one<'a>(root: ElementRef<'a>, selector: &str) -> Result<ElementRef<'a>> {
    let found = select(root, selector);
    if found.len() != 1 {
        return Err(format!("expected one {selector}, found {}", found.len()).into());
    }
    Ok(found[0])
}

fn id<'a>(root: ElementRef<'a>, expected: &str) -> Result<ElementRef<'a>> {
    let found: Vec<_> = select(root, "[id]")
        .into_iter()
        .filter(|el| el.value().id() == Some(expected))
        .collect();
    if found.len() != 1 {
        return Err(format!("expected one #{expected}, found {}", found.len()).into());
    }
    Ok(found[0])
}

fn text(el: ElementRef<'_>) -> String {
    el.text()
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn count(value: &str, context: &str) -> Result<u64> {
    if value.is_empty() || !value.bytes().all(|b| b.is_ascii_digit()) {
        return Err(
            format!("{context}: expected an unsigned decimal integer, got {value:?}").into(),
        );
    }
    value
        .parse()
        .map_err(|_| format!("{context}: count exceeds u64").into())
}

fn labeled_count(root: ElementRef<'_>, label: &str) -> Result<u64> {
    let labels: Vec<_> = select(root, "div, span")
        .into_iter()
        .filter(|el| text(*el) == label && el.child_elements().count() == 0)
        .collect();
    if labels.len() != 1 {
        return Err(format!(
            "expected one metric label {label:?}, found {}",
            labels.len()
        )
        .into());
    }
    let value = labels[0]
        .next_sibling_element()
        .ok_or_else(|| format!("{label}: missing count"))?;
    count(&text(value), label)
}

fn sum(mut values: impl Iterator<Item = u64>, context: &str) -> Result<u64> {
    values.try_fold(0_u64, |total, value| {
        total
            .checked_add(value)
            .ok_or_else(|| format!("{context}: total overflow").into())
    })
}

fn server_cards(root: ElementRef<'_>) -> Result<Vec<ElementRef<'_>>> {
    let container = id(root, "metrics-servers")?;
    let cards = select(container, "[id^='server-metrics-']");
    if cards.is_empty() {
        return Err("no server metric cards found".into());
    }
    if select(container, "h2").len() != cards.len() {
        return Err("server card has a missing or unexpected identity".into());
    }
    Ok(cards)
}

fn chart_data(card: ElementRef<'_>, server_id: &str) -> Result<Value> {
    let chart = id(card, &format!("ccu-history-{server_id}"))?;
    if chart.value().attr("data-server") != Some(server_id) {
        return Err("chart server identity mismatch".into());
    }
    let data: Value = serde_json::from_str(
        chart
            .value()
            .attr("data-chart-data")
            .ok_or("missing chart payload for readiness")?,
    )?;
    if !data.is_array() {
        return Err("invalid chart payload for readiness".into());
    }
    Ok(data)
}

/// Inspect only the fields needed to track asynchronous completion. Current
/// metric fields may still contain placeholders until acquisition is ready.
pub(crate) fn readiness_charts(html: &str) -> Result<BTreeMap<String, Value>> {
    let doc = Html::parse_document(html);
    let mut charts = BTreeMap::new();
    for card in server_cards(doc.root_element())? {
        let server_id = card
            .value()
            .id()
            .unwrap()
            .strip_prefix("server-metrics-")
            .unwrap();
        if server_id.is_empty() {
            return Err("empty server identity".into());
        }
        let data = chart_data(card, server_id).map_err(|e| format!("server {server_id:?}: {e}"))?;
        if charts.insert(server_id.into(), data).is_some() {
            return Err(format!("duplicate server identity {server_id:?}").into());
        }
    }
    Ok(charts)
}

/// Decode current fields from an already completed rendering. This alone does
/// not establish readiness; live collection and fixture replay use acquisition.
pub fn parse_snapshot(html: &str, observed_at: DateTime<Utc>) -> Result<Snapshot> {
    let doc = Html::parse_document(html);
    let root = doc.root_element();
    let heading = one(root, "h1")?;
    if text(heading) != "Server Metrics" {
        return Err("missing Server Metrics heading".into());
    }
    let summary = id(root, "public-metric-summary")?;
    let subscriptions = text(id(summary, "active-subscriptions-count")?);
    let subscriptions = subscriptions
        .strip_suffix(" Active Subscriptions")
        .ok_or("Active Subscriptions label changed")?;
    let active_subscriptions = count(subscriptions, "Active Subscriptions")?;
    let summary_fields = select(summary, "span");
    if summary_fields.len() != 2 {
        return Err("unexpected public summary metrics".into());
    }
    let total = summary_fields
        .into_iter()
        .find(|el| el.value().id() != Some("active-subscriptions-count"))
        .ok_or("missing Total Online")?;
    let total = text(total);
    let total_online = count(
        total
            .strip_suffix(" Total Online")
            .ok_or("Total Online label changed")?,
        "Total Online",
    )?;

    let cards = server_cards(root)?;
    let mut servers = Vec::new();
    for card in cards {
        let server_id = card
            .value()
            .id()
            .unwrap()
            .strip_prefix("server-metrics-")
            .unwrap();
        let result = (|| -> Result<Server> {
            let name = text(one(card, "h2")?);
            let stats = id(card, &format!("server-stats-{server_id}"))?;
            if stats.child_elements().count() != 2 {
                return Err("unexpected activity fields".into());
            }
            let daily_active = labeled_count(stats, "DAILY ACTIVE")?;
            let monthly_active = labeled_count(stats, "MONTHLY ACTIVE")?;
            let online = labeled_count(card, "Online")?;
            let section = id(card, &format!("starting-zones-{server_id}"))?;
            if text(one(section, "h3")?) != "Starting Zones" {
                return Err("Starting Zones label changed".into());
            }
            let total = id(section, &format!("starting-zone-total-{server_id}"))?;
            let label = total
                .next_sibling_element()
                .ok_or("missing starting-zone total label")?;
            if text(label) != "Players" {
                return Err("starting-zone total label changed".into());
            }
            let total = count(&text(total), "starting-zone total")?;
            let list = id(section, &format!("starting-zone-list-{server_id}"))?;
            let mut starting_zones = Vec::new();
            for zone in list.child_elements() {
                let prefix = format!("starting-zone-{server_id}-");
                let zone_id = zone
                    .value()
                    .id()
                    .and_then(|s| s.strip_prefix(&prefix))
                    .filter(|id| ZONES.contains(id))
                    .ok_or_else(|| format!(
                        "#starting-zone-list-{server_id}: missing or unexpected zone identity; expected prefix {prefix:?} and an approved zone ID, found <{}> with id {:?} and text {:?}",
                        zone.value().name(),
                        zone.value().id(),
                        text(zone).chars().take(160).collect::<String>()
                    ))?;
                let fields = select(zone, "span");
                if fields.len() != 2 {
                    return Err("zone must contain its name and count".into());
                }
                let name = text(fields[0]);
                let online = count(&text(fields[1]), "zone count")?;
                starting_zones.push(StartingZone {
                    id: zone_id.into(),
                    name,
                    online,
                });
            }
            let roster: BTreeSet<_> = starting_zones.iter().map(|z| z.id.as_str()).collect();
            if roster != BTreeSet::from(ZONES) || starting_zones.len() != ZONES.len() {
                return Err("starting-zone roster changed or contains duplicates".into());
            }
            if sum(starting_zones.iter().map(|z| z.online), "starting zones")? != total {
                return Err("starting-zone total does not equal zone counts".into());
            }
            starting_zones.sort_by(|a, b| a.id.cmp(&b.id));
            chart_data(card, server_id)?;
            Ok(Server {
                id: server_id.into(),
                name,
                daily_active,
                monthly_active,
                online,
                starting_zones,
            })
        })()
        .map_err(|e| format!("server {server_id:?}: {e}"))?;
        servers.push(result);
    }
    if sum(servers.iter().map(|s| s.online), "online")? != total_online {
        return Err("Total Online does not equal the server counts".into());
    }
    servers.sort_by(|a, b| a.id.cmp(&b.id));
    let snapshot = Snapshot {
        observed_at,
        active_subscriptions,
        servers,
    };
    snapshot.validate()?;
    Ok(snapshot)
}
