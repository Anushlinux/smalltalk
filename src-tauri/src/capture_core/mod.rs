//! Work-first capture core.
//!
//! This module is the parallel, policy-oriented capture core. The existing
//! Tauri command facade still lives in `capture.rs`; code is moved here when it
//! is deterministic, testable, and not tied to UI/runtime plumbing.

#![allow(dead_code)]

pub mod browser_adapter;
pub mod episode;
pub mod event_governor;
pub mod extractors;
pub mod privacy;
pub mod quality;
pub mod resume_dossier;
pub mod snapshot;
pub mod store;
