use chrono::{SecondsFormat, Utc};
use mnm_stats_collector::{Result, SOURCE, acquisition, storage::HistoryFile};
use mnm_stats_model::History;
use std::{env, fs, path::PathBuf, process::ExitCode, time::Duration};

const USAGE: &str = "usage: mnm-stats-collector validate-history PATH [--latest-observed-at] | collect --history PATH [--source URL | --replay FILE [--observed-at RFC3339]]";

fn run() -> Result<()> {
    let mut args = env::args_os().skip(1);
    let command = args.next();
    match command.as_ref().and_then(|s| s.to_str()) {
        Some("validate-history") => {
            let path = args.next().ok_or("validate-history needs a path")?;
            let latest_observed_at = match args.next() {
                None => false,
                Some(flag) if flag == "--latest-observed-at" => true,
                Some(_) => return Err("unexpected validate-history argument".into()),
            };
            if args.next().is_some() {
                return Err("unexpected validate-history argument".into());
            }
            let history = History::from_jsonl(&fs::read_to_string(path)?)?;
            if latest_observed_at {
                if let Some(snapshot) = history.snapshots().last() {
                    println!(
                        "{}",
                        snapshot
                            .observed_at
                            .to_rfc3339_opts(SecondsFormat::AutoSi, true)
                    );
                }
            } else {
                println!("Valid history: {} observations", history.snapshots().len());
            }
        }
        Some("collect") => {
            let mut history = None;
            let mut source = None;
            let mut replay = None;
            let mut observed_at = None;
            while let Some(flag) = args.next() {
                let value = args.next().ok_or("collect option requires a value")?;
                match flag.to_str() {
                    Some("--history") if history.is_none() => history = Some(PathBuf::from(value)),
                    Some("--source") if source.is_none() => {
                        source = Some(value.into_string().map_err(|_| "source must be UTF-8")?)
                    }
                    Some("--replay") if replay.is_none() => replay = Some(PathBuf::from(value)),
                    Some("--observed-at") if observed_at.is_none() => {
                        observed_at = Some(
                            chrono::DateTime::parse_from_rfc3339(
                                value.to_str().ok_or("timestamp must be UTF-8")?,
                            )?
                            .to_utc(),
                        )
                    }
                    _ => return Err("unknown or repeated collect option".into()),
                }
            }
            if replay.is_some() && source.is_some() {
                return Err("--replay and --source are mutually exclusive".into());
            }
            if observed_at.is_some() && replay.is_none() {
                return Err("--observed-at is only available with fixture replay".into());
            }
            let history = history.ok_or("collect requires --history PATH")?;
            let timeout = Duration::from_secs(20);
            let mut output = HistoryFile::open(&history, timeout)?;
            if output.contains_hour(observed_at.unwrap_or_else(Utc::now)) {
                println!("Current UTC hour already recorded; history unchanged");
                return Ok(());
            }
            let snapshot = match replay {
                Some(path) => acquisition::replay(
                    &serde_json::from_str::<Vec<_>>(&fs::read_to_string(path)?)?,
                    observed_at.unwrap_or_else(Utc::now),
                )?,
                None => acquisition::collect(source.as_deref().unwrap_or(SOURCE), timeout)?,
            };
            let observed_at = snapshot.observed_at;
            if output.append(snapshot)? {
                println!("Recorded observation at {observed_at}");
            } else {
                println!("UTC hour already recorded; history unchanged");
            }
        }
        _ => return Err(USAGE.into()),
    }
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
