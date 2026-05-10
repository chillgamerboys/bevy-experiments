//! Bevy experiments — a sandbox for iterating on small game ideas.
//!
//! Each experiment lives under [`games`] as a self-contained module that
//! exposes a `pub fn run()`. Binary wrappers in `src/bin/` dispatch to those.

pub mod games;
