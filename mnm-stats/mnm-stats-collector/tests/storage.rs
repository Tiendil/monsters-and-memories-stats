mod common;
use common::*;
use mnm_stats_collector::{parser::parse_snapshot, storage::HistoryFile};
use mnm_stats_model::History;
use std::{fs, time::Duration};

#[test]
fn appends_one_hour_preserving_original_bytes_and_missing_intervals() {
    let scratch = Scratch::new();
    let path = scratch.0.join("history.jsonl");
    let first = parse_snapshot(CONNECTED, time()).unwrap();
    let original = first
        .to_jsonl_record()
        .unwrap()
        .replace("\"schema_version\":1", "\"schema_version\": 1")
        .trim_end()
        .to_owned();
    fs::write(&path, &original).unwrap();
    let mut file = HistoryFile::open(&path, Duration::from_secs(1)).unwrap();
    let mut same = first.clone();
    same.active_subscriptions = 1;
    assert!(!file.append(same).unwrap());
    assert_eq!(fs::read_to_string(&path).unwrap(), original);
    let mut next = first.clone();
    next.observed_at += chrono::Duration::hours(3);
    assert!(file.append(next.clone()).unwrap());
    let expected = format!("{original}\n{}", next.to_jsonl_record().unwrap());
    assert_eq!(fs::read_to_string(&path).unwrap(), expected);
    assert_eq!(History::from_jsonl(&expected).unwrap().snapshots().len(), 2);
    let mut past = first;
    past.observed_at -= chrono::Duration::hours(1);
    assert!(file.append(past).is_err());
    assert_eq!(fs::read_to_string(&path).unwrap(), expected);
}

#[test]
fn corruption_write_failure_and_interrupted_staging_preserve_history() {
    let scratch = Scratch::new();
    let path = scratch.0.join("history.jsonl");
    fs::write(&path, "{broken").unwrap();
    assert!(HistoryFile::open(&path, Duration::from_secs(1)).is_err());
    assert_eq!(fs::read_to_string(&path).unwrap(), "{broken");
    fs::write(&path, "").unwrap();
    let mut file = HistoryFile::open(&path, Duration::from_secs(1)).unwrap();
    fs::create_dir(scratch.0.join("history.jsonl.tmp")).unwrap();
    assert!(
        file.append(parse_snapshot(CONNECTED, time()).unwrap())
            .is_err()
    );
    assert_eq!(fs::read_to_string(&path).unwrap(), "");
    fs::remove_dir(scratch.0.join("history.jsonl.tmp")).unwrap();
    // A terminated writer can leave a partial staging file, never partial history.
    fs::write(scratch.0.join("history.jsonl.tmp"), "{partial").unwrap();
    assert!(
        file.append(parse_snapshot(CONNECTED, time()).unwrap())
            .unwrap()
    );
    assert_eq!(
        History::from_jsonl(&fs::read_to_string(&path).unwrap())
            .unwrap()
            .snapshots()
            .len(),
        1
    );
    assert!(!scratch.0.join("history.jsonl.tmp").exists());
}

#[test]
fn overlapping_writers_lock_the_stable_sidecar() {
    let scratch = Scratch::new();
    let path = scratch.0.join("history.jsonl");
    let mut first = HistoryFile::open(&path, Duration::from_secs(1)).unwrap();
    assert!(HistoryFile::open(&path, Duration::from_millis(20)).is_err());
    let other_path = path.clone();
    let second = std::thread::spawn(move || {
        let mut second = HistoryFile::open(&other_path, Duration::from_secs(2)).unwrap();
        assert!(
            !second
                .append(parse_snapshot(CONNECTED, time()).unwrap())
                .unwrap()
        );
        let mut next = parse_snapshot(CONNECTED, time()).unwrap();
        next.observed_at += chrono::Duration::hours(1);
        assert!(second.append(next).unwrap());
    });
    first
        .append(parse_snapshot(CONNECTED, time()).unwrap())
        .unwrap();
    drop(first);
    second.join().unwrap();
    assert_eq!(
        History::from_jsonl(&fs::read_to_string(path).unwrap())
            .unwrap()
            .snapshots()
            .len(),
        2
    );
}
