//! ove-timeline — editor-grade timeline state machine.
//!
//! Design authority:
//! * ADR-006: per-track ordered clip sequences, exact rational times, effects live in
//!   the render graph (not here). State = fold(command log).
//! * ADR-011: gap-buffer-class primary structure + ordered access for random/agent
//!   queries. This crate realizes BOTH roles in ONE structure per track
//!   ([`gap::CursorBuffer`], sorted by clip start, O(1) random access) — a second
//!   index structure is deferred until E-012 profiling demands it. This also avoids
//!   E-002c2's time-keyed rekey-collision bug class by construction (there is no
//!   second ordered copy to rekey).
//! * ADR-009: every command ships an exact inverse; batches are atomic via composite
//!   inverse (apply prefix, roll back on failure); undo/redo are session operations.
//! * ADR-007: all times are [`ove_time::Rational`]; no fp anywhere.
//!
//! State-hash rule: the hash covers observable clip content only (canonical order:
//! track index, start, id). The id-assignment counter is deliberately EXCLUDED — it
//! is not observable engine state; its determinism comes from replaying the same
//! journal in the same order (E-003 model). Undo therefore restores the hash exactly
//! without counter restoration.
//!
//! Error policy: validation is complete BEFORE any mutation; a rejected command
//! leaves state and journal untouched (MEDIA_ENGINE_SPEC §5).

pub mod clip;
pub mod command;
pub mod engine;
pub mod gap;
pub mod state;

pub use clip::{AssetId, ClipEntry, ClipId, TrackId, TrackKind};
pub use command::Command;
pub use engine::{apply_command, JournalEntry, Receipt, TimelineEngine, TimelineError};
pub use state::{TimelineState, TICK_AXIS_DEFAULT};
