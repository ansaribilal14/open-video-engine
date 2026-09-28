//! manifest.json — the project's identity + registry (PROJECT_FORMAT_SPEC
//! §2). The manifest is a HINT layer: the command log is the truth and the
//! snapshot meta is the snapshot truth (ADR-016 authority order). Stale
//! manifests (crash between log append and manifest rewrite) are reconciled
//! on open, never trusted blindly.

use serde::{Deserialize, Serialize};

use crate::state::NumPair;
use crate::ProjectError;

pub const SCHEMA_VERSION: u32 = 1;
pub const FORMAT_TAG: &str = "ove/project";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreatedBy {
    pub engine: String,
    pub version: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestState {
    /// Seq of the latest snapshot (0 = none). HINT: the loader prefers
    /// snapshot/meta.json when present (ADR-016 authority order).
    pub snapshot_seq: u64,
    /// Number of log entries at the last manifest write (hint for staleness
    /// detection).
    pub log_len: u64,
    /// BLAKE3-256 hex of the document state at the last manifest write.
    pub state_hash: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetEntry {
    /// Stable asset id used by command payloads/logs ("a-1", "a-2", ...).
    pub id: String,
    /// BLAKE3-256 hex — the asset IS the cache key (R-13: relink-by-path is
    /// forbidden; hash is identity).
    pub content_hash: String,
    /// Folder-relative path: assets/<content-hash>/src.<ext>
    pub path: String,
    /// Folder-relative probe sidecar when a probe was provided at import.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub probe: Option<String>,
}

/// Track registry entry (ADR-016): track structure is project SETUP, not a
/// command — ove-timeline has no AddTrack command by design (tracks are
/// scaffolding; clips are the command surface). The registry is applied to
/// the timeline before log replay; the snapshot mirror is the authority for
/// an opened snapshot and refreshes this registry on reconciliation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrackEntry {
    pub id: u64,
    /// "gap" | "avl" | "oracle"
    pub kind: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    pub schema_version: u32,
    pub format: String,
    /// Project-wide fixed rational axis for aggregates (ADR-007 refinement).
    pub tick_axis: NumPair,
    pub created_by: CreatedBy,
    pub state: ManifestState,
    pub assets: Vec<AssetEntry>,
    pub tracks: Vec<TrackEntry>,
    pub uuid: String,
}

impl Manifest {
    pub fn initial(tick_axis: NumPair) -> Self {
        Manifest {
            schema_version: SCHEMA_VERSION,
            format: FORMAT_TAG.to_string(),
            tick_axis,
            created_by: CreatedBy {
                engine: "ove".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
            state: ManifestState {
                snapshot_seq: 0,
                log_len: 0,
                state_hash: String::new(),
            },
            assets: Vec::new(),
            tracks: Vec::new(),
            uuid: uuid::Uuid::new_v4().to_string(),
        }
    }

    pub fn load(path: &std::path::Path) -> Result<Manifest, ProjectError> {
        let raw = std::fs::read_to_string(path)
            .map_err(|e| ProjectError::Io(format!("read manifest: {e}")))?;
        let m: Manifest = serde_json::from_str(&raw)
            .map_err(|e| ProjectError::ManifestInvalid(format!("schema: {e}")))?;
        if m.format != FORMAT_TAG {
            return Err(ProjectError::ManifestInvalid(format!(
                "format tag {:?} is not {FORMAT_TAG:?}",
                m.format
            )));
        }
        if m.schema_version > SCHEMA_VERSION {
            return Err(ProjectError::SchemaVersion {
                found: m.schema_version,
                supported: SCHEMA_VERSION,
            });
        }
        if m.tick_axis.den <= 0 {
            return Err(ProjectError::ManifestInvalid(
                "tick_axis.den must be positive".into(),
            ));
        }
        Ok(m)
    }

    /// Atomic write: temp file + rename (PROJECT_FORMAT_SPEC §4.4).
    pub fn store(&self, path: &std::path::Path) -> Result<(), ProjectError> {
        let tmp = path.with_extension("json.tmp");
        let raw = serde_json::to_vec_pretty(self)
            .map_err(|e| ProjectError::Internal(format!("manifest serialization: {e}")))?;
        std::fs::write(&tmp, raw)
            .map_err(|e| ProjectError::Io(format!("write manifest tmp: {e}")))?;
        std::fs::rename(&tmp, path)
            .map_err(|e| ProjectError::Io(format!("manifest rename: {e}")))?;
        Ok(())
    }
}

/// snapshot/meta.json — the SNAPSHOT AUTHORITY (see ADR-016 authority
/// order). Written atomically AFTER state-<seq>.json is complete; a torn
/// compaction leaves the previous meta + full log suffix (still replays).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotMeta {
    pub snapshot_seq: u64,
    pub state_hash: String,
    pub engine_version: String,
}

impl SnapshotMeta {
    pub fn load(dir: &std::path::Path) -> Result<Option<SnapshotMeta>, ProjectError> {
        let p = dir.join("meta.json");
        if !p.exists() {
            return Ok(None);
        }
        let raw = std::fs::read_to_string(&p)
            .map_err(|e| ProjectError::Io(format!("read snapshot meta: {e}")))?;
        serde_json::from_str(&raw)
            .map(Some)
            .map_err(|e| ProjectError::ManifestInvalid(format!("snapshot meta schema: {e}")))
    }

    pub fn store(&self, dir: &std::path::Path) -> Result<(), ProjectError> {
        let tmp = dir.join("meta.json.tmp");
        let fin = dir.join("meta.json");
        let raw = serde_json::to_vec_pretty(self)
            .map_err(|e| ProjectError::Internal(format!("meta serialization: {e}")))?;
        std::fs::write(&tmp, raw).map_err(|e| ProjectError::Io(format!("write meta tmp: {e}")))?;
        std::fs::rename(&tmp, &fin).map_err(|e| ProjectError::Io(format!("meta rename: {e}")))?;
        Ok(())
    }
}
