use crate::Result;
use chrono::{DateTime, Utc};
use mnm_stats_model::{History, Snapshot};
use std::{
    fs::{self, File, OpenOptions, TryLockError},
    io::Write,
    path::{Path, PathBuf},
    thread,
    time::{Duration, Instant},
};

pub struct HistoryFile {
    path: PathBuf,
    original: String,
    history: History,
    // Lock a stable sidecar: replacing the history inode must not release mutual exclusion.
    _lock: File,
}

impl HistoryFile {
    pub fn open(path: &Path, timeout: Duration) -> Result<Self> {
        let parent = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        fs::create_dir_all(parent)?;
        let lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(sidecar(path, ".lock"))?;
        let deadline = Instant::now() + timeout;
        loop {
            match lock.try_lock() {
                Ok(()) => break,
                Err(TryLockError::WouldBlock) if Instant::now() < deadline => {
                    thread::sleep(Duration::from_millis(10))
                }
                Err(error) => {
                    return Err(format!("cannot lock history {}: {error}", path.display()).into());
                }
            }
        }
        let original = match fs::read_to_string(path) {
            Ok(value) => value,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
            Err(error) => return Err(error.into()),
        };
        let history = History::from_jsonl(&original)?;
        Ok(Self {
            path: path.into(),
            original,
            history,
            _lock: lock,
        })
    }

    pub fn contains_hour(&self, time: DateTime<Utc>) -> bool {
        self.history.snapshots().iter().any(|s| {
            s.observed_at.timestamp().div_euclid(3600) == time.timestamp().div_euclid(3600)
        })
    }

    /// Returns false for an already-recorded hour, without replacing its sample.
    pub fn append(&mut self, snapshot: Snapshot) -> Result<bool> {
        if self.contains_hour(snapshot.observed_at) {
            return Ok(false);
        }
        let record = snapshot.to_jsonl_record()?;
        let mut snapshots = self.history.snapshots().to_vec();
        snapshots.push(snapshot);
        let history = History::new(snapshots)?;
        let mut updated = self.original.clone();
        if !updated.is_empty() && !updated.ends_with('\n') {
            updated.push('\n');
        }
        updated.push_str(&record);
        let temporary = sidecar(&self.path, ".tmp");
        let result = (|| -> Result<()> {
            let mut output = OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(&temporary)?;
            output.write_all(updated.as_bytes())?;
            output.sync_all()?;
            drop(output);
            fs::rename(&temporary, &self.path)?;
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temporary);
        }
        result?;
        self.original = updated;
        self.history = history;
        Ok(true)
    }
}

fn sidecar(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(suffix);
    name.into()
}
