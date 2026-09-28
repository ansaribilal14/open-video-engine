//! State mirror — the canonical serde representation of the folded project
//! state (PROJECT_FORMAT_SPEC §4/§2).
//!
//! Canonicalization rule (normative, ADR-016): the state hash is BLAKE3-256
//! over `serde_json::to_vec` of the mirror struct — declaration-order object
//! keys, `BTreeMap` for track maps (sorted), sorted `used_ids`, rationals as
//! exact {num, den} pairs, no floats anywhere. It covers the DOCUMENT
//! (timeline + id-allocation state + asset registry identities); per
//! ADR-008 the undo stack is session state rebuilt by replay and is NOT
//! hashed (P-3 equivalence is about the document).

use std::collections::BTreeMap;

use ove_media::ContentHash;
use ove_time::Rational;
use ove_timeline::{Clip, ClipId, GapTrack, OracleTrack, Timeline, TrackId, TrackKind};
use serde::{Deserialize, Serialize};

/// Exact rational in the on-disk canonical shape (floats are FORBIDDEN by
/// the schema — serde type errors reject them at load, PROJECT_FORMAT_SPEC
/// §3.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NumPair {
    pub num: i64,
    pub den: i64,
}

impl NumPair {
    pub fn of(r: Rational) -> Self {
        NumPair {
            num: r.num(),
            den: r.den(),
        }
    }
    pub fn to_rational(self) -> Rational {
        Rational::new(self.num, self.den)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MirrorClip {
    pub id: ClipId,
    pub duration: NumPair,
    pub source_in: NumPair,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MirrorTrackKind {
    Gap,
    Avl,
    Oracle,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MirrorTrack {
    pub kind: MirrorTrackKind,
    /// Clips in playback order (position order; absolute starts are DERIVED,
    /// never stored — ove-timeline contract).
    pub clips: Vec<MirrorClip>,
}

/// The serialized document state.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateMirror {
    /// BTreeMap → JSON object keys in sorted order (canonical).
    pub tracks: BTreeMap<TrackId, MirrorTrack>,
    /// E-012: id-allocation state is DOCUMENT STATE — replay determinism
    /// requires next_id + used_ids to survive save/reload (a snapshot does
    /// not replay removed-id history, so both are stored).
    pub next_id: ClipId,
    pub used_ids: Vec<ClipId>,
}

impl StateMirror {
    /// Capture the exact state of `timeline`.
    pub fn from_timeline(tl: &Timeline) -> Self {
        let mut tracks = BTreeMap::new();
        for tid in tl.track_ids() {
            let kind_tag = match tl.track(tid) {
                Some(TrackKind::Gap(_)) => MirrorTrackKind::Gap,
                Some(TrackKind::Avl(_)) => MirrorTrackKind::Avl,
                Some(TrackKind::Oracle(_)) => MirrorTrackKind::Oracle,
                None => unreachable!("track_ids yields existing tracks"),
            };
            let mut clips = Vec::new();
            tl.track_ref(tid)
                .expect("track exists")
                .walk(&mut |_pos, _start, c: &Clip| {
                    clips.push(MirrorClip {
                        id: c.id,
                        duration: NumPair::of(c.duration),
                        source_in: NumPair::of(c.source_in),
                    });
                });
            tracks.insert(
                tid,
                MirrorTrack {
                    kind: kind_tag,
                    clips,
                },
            );
        }
        let mut used_ids: Vec<ClipId> = tl.used_ids().collect();
        used_ids.sort_unstable();
        StateMirror {
            tracks,
            next_id: tl.next_id_value(),
            used_ids,
        }
    }

    /// Rebuild a Timeline from the mirror. The container kind is restored
    /// (gap/avl/oracle); container internals rebuild canonically from the
    /// ordered clips (walk order is the state contract).
    pub fn to_timeline(&self) -> Result<Timeline, String> {
        // validate: every live clip id must be in used_ids (apply()'s
        // duplicate check depends on the exact set — E-012 state).
        let used: std::collections::HashSet<ClipId> = self.used_ids.iter().copied().collect();
        let mut tracks = BTreeMap::new();
        for (tid, mt) in &self.tracks {
            let kind = match mt.kind {
                MirrorTrackKind::Gap => TrackKind::Gap(GapTrack::new()),
                MirrorTrackKind::Avl => TrackKind::Avl(ove_timeline::AvlTrack::new()),
                MirrorTrackKind::Oracle => TrackKind::Oracle(OracleTrack::new()),
            };
            tracks.insert(*tid, kind);
            for (pos, mc) in mt.clips.iter().enumerate() {
                if !used.contains(&mc.id) {
                    return Err(format!(
                        "track {tid} clip {id} at position {pos} missing from used_ids",
                        id = mc.id
                    ));
                }
                let clip = Clip::new(mc.id, mc.duration.to_rational(), mc.source_in.to_rational());
                ove_timeline::TrackOps::insert_at(
                    tracks.get_mut(tid).expect("just inserted"),
                    pos,
                    clip,
                )
                .map_err(|e| format!("rebuilding track {tid} position {pos}: {e:?}"))?;
            }
        }
        Ok(Timeline::from_parts(tracks, self.next_id, used))
    }

    /// BLAKE3-256 over the canonical serialization (64 lowercase hex) —
    /// PROJECT_FORMAT_SPEC §2 hash policy; the manifest's `state_hash`.
    pub fn state_hash(&self) -> String {
        let bytes = serde_json::to_vec(self).expect("mirror serialization is infallible");
        ContentHash::from_bytes(&bytes).hex()
    }
}
