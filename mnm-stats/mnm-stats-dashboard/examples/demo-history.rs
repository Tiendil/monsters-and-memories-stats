//! Local preview data. Pass a UTC timestamp to reproduce the same history.
use chrono::{DateTime, Duration, Months, Timelike, Utc};
use mnm_stats_model::{History, Server, Snapshot, StartingZone};
use std::{collections::BTreeSet, error::Error, io::Write, time::SystemTime};

fn main() -> Result<(), Box<dyn Error>> {
    let now = match std::env::args().nth(1) {
        Some(value) => value.parse::<DateTime<Utc>>()?,
        None => SystemTime::now().into(),
    };
    let end = now
        .with_minute(0)
        .unwrap()
        .with_second(0)
        .unwrap()
        .with_nanosecond(0)
        .unwrap();
    let mut times = BTreeSet::new();
    for hour in 0..14 * 24 {
        times.insert(end - Duration::hours(hour));
    }
    for month in 1..=24 {
        let start = end.checked_sub_months(Months::new(month)).unwrap();
        for hour in 0..6 {
            times.insert(start - Duration::hours(hour));
        }
    }
    let snapshots = times
        .into_iter()
        .enumerate()
        .map(|(index, observed_at)| {
            let step = index as u64;
            Snapshot {
                observed_at,
                active_subscriptions: 2000 + step,
                servers: ["Amber", "Birch", "Cedar"]
                    .into_iter()
                    .enumerate()
                    .map(|(i, name)| {
                        let population = 100 + i as u64 * 40 + step % 37;
                        Server {
                            id: format!("demo-{i}"),
                            name: format!("Demo {name}"),
                            daily_active: population * 3,
                            monthly_active: population * 12,
                            online: population,
                            starting_zones: vec![
                                StartingZone {
                                    id: "harbor".into(),
                                    name: "Demo Harbor".into(),
                                    online: population / 4,
                                },
                                StartingZone {
                                    id: "hills".into(),
                                    name: "Demo Hills".into(),
                                    online: population / 5,
                                },
                            ],
                        }
                    })
                    .collect(),
            }
        })
        .collect();
    let history = History::new(snapshots)?;
    let mut output = std::io::stdout().lock();
    for snapshot in history.snapshots() {
        output.write_all(snapshot.to_jsonl_record()?.as_bytes())?;
    }
    Ok(())
}
