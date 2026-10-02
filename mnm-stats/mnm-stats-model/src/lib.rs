//! Shared current-state observations, JSONL storage, and full-history JSON export.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize, de};
use std::{collections::HashSet, fmt};

pub const SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StartingZone {
    pub id: String,
    pub name: String,
    pub online: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Server {
    pub id: String,
    pub name: String,
    pub daily_active: u64,
    pub monthly_active: u64,
    pub online: u64,
    pub starting_zones: Vec<StartingZone>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    #[serde(deserialize_with = "deserialize_utc")]
    pub observed_at: DateTime<Utc>,
    pub active_subscriptions: u64,
    pub servers: Vec<Server>,
}

fn deserialize_utc<'de, D: Deserializer<'de>>(deserializer: D) -> Result<DateTime<Utc>, D::Error> {
    let text = String::deserialize(deserializer)?;
    let timestamp = DateTime::parse_from_rfc3339(&text).map_err(de::Error::custom)?;
    if timestamp.offset().local_minus_utc() != 0 {
        return Err(de::Error::custom("observed_at must be UTC"));
    }
    Ok(timestamp.with_timezone(&Utc))
}

#[derive(Debug)]
pub struct Error(String);

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl std::error::Error for Error {}

fn identity(id: &str, name: &str, scope: &str) -> Result<(), Error> {
    if id.trim().is_empty() || name.trim().is_empty() {
        return Err(Error(format!(
            "{scope}: identity and name must not be empty"
        )));
    }
    Ok(())
}

impl Snapshot {
    pub fn validate(&self) -> Result<(), Error> {
        let mut servers = HashSet::new();
        for server in &self.servers {
            identity(&server.id, &server.name, "server")?;
            if !servers.insert(&server.id) {
                return Err(Error(format!("duplicate server identity {:?}", server.id)));
            }
            let mut zones = HashSet::new();
            for zone in &server.starting_zones {
                identity(
                    &zone.id,
                    &zone.name,
                    &format!("server {:?} zone", server.id),
                )?;
                if !zones.insert(&zone.id) {
                    return Err(Error(format!(
                        "server {:?}: duplicate zone identity {:?}",
                        server.id, zone.id
                    )));
                }
            }
        }
        Ok(())
    }

    /// Encode one compact, newline-terminated record in stable identity order.
    pub fn to_jsonl_record(&self) -> Result<String, Error> {
        self.validate()?;
        let mut snapshot = self.clone();
        snapshot.servers.sort_by(|a, b| a.id.cmp(&b.id));
        for server in &mut snapshot.servers {
            server.starting_zones.sort_by(|a, b| a.id.cmp(&b.id));
        }
        #[derive(Serialize)]
        struct Record<'a> {
            schema_version: u32,
            #[serde(flatten)]
            snapshot: &'a Snapshot,
        }
        let mut output = serde_json::to_string(&Record {
            schema_version: SCHEMA_VERSION,
            snapshot: &snapshot,
        })
        .map_err(|e| Error(e.to_string()))?;
        output.push('\n');
        Ok(output)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct History(Vec<Snapshot>);

impl History {
    pub fn new(snapshots: Vec<Snapshot>) -> Result<Self, Error> {
        for (index, snapshot) in snapshots.iter().enumerate() {
            snapshot
                .validate()
                .map_err(|e| Error(format!("snapshot {}: {e}", index + 1)))?;
            if let Some(previous) = index.checked_sub(1).map(|i| &snapshots[i]) {
                if snapshot.observed_at <= previous.observed_at {
                    return Err(Error(format!(
                        "snapshot {}: observations must be chronological",
                        index + 1
                    )));
                }
                if snapshot.observed_at.timestamp().div_euclid(3600)
                    == previous.observed_at.timestamp().div_euclid(3600)
                {
                    return Err(Error(format!("snapshot {}: duplicate UTC hour", index + 1)));
                }
            }
        }
        Ok(Self(snapshots))
    }

    pub fn snapshots(&self) -> &[Snapshot] {
        &self.0
    }

    pub fn from_jsonl(input: &str) -> Result<Self, Error> {
        // Keep the wire record explicit so serde rejects duplicate and unknown
        // fields instead of losing them through an intermediate JSON Value map.
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Record {
            schema_version: u32,
            #[serde(deserialize_with = "deserialize_utc")]
            observed_at: DateTime<Utc>,
            active_subscriptions: u64,
            servers: Vec<Server>,
        }
        let mut snapshots = Vec::new();
        for (index, line) in input.lines().enumerate() {
            let record: Record = serde_json::from_str(line)
                .map_err(|e| Error(format!("line {}: {e}", index + 1)))?;
            if record.schema_version != SCHEMA_VERSION {
                return Err(Error(format!(
                    "line {}: unsupported schema_version {}",
                    index + 1,
                    record.schema_version
                )));
            }
            snapshots.push(Snapshot {
                observed_at: record.observed_at,
                active_subscriptions: record.active_subscriptions,
                servers: record.servers,
            });
        }
        Self::new(snapshots)
    }

    /// Export every observation; per-record versions become one root version.
    pub fn to_json(&self) -> Result<String, Error> {
        #[derive(Serialize)]
        struct Export<'a> {
            schema_version: u32,
            snapshots: &'a [Snapshot],
        }
        serde_json::to_string(&Export {
            schema_version: SCHEMA_VERSION,
            snapshots: &self.0,
        })
        .map_err(|e| Error(e.to_string()))
    }
}
