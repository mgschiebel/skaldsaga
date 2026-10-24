//! Core library for Skaldsaga.
//!
//! The deterministic parts of Skaldsaga (spec checks, the single committer,
//! the index rebuild, the session meter) live here so that the command-line
//! front end and a later standalone app share one implementation. The core
//! stays portable: no desktop-only dependencies, and file access goes through
//! [`fsio`].
//!
//! Built so far: the session meter (the pilot), status line wiring, the
//! session-start hook, the typed spec model with its loader, first-run setup
//! (`init`), the attribute registry, the generated index (`index`), and the
//! dependency checker (`check`) with the operation engine it runs on, the
//! single committer (`commit`), the demo answers (`demo`), and `complete`: the
//! script soft checks, the gates and the writer's stored decisions.

pub mod check;
pub mod commit;
pub mod complete;
pub mod decisions;
pub mod demo;
pub mod fsio;
pub mod hooks;
pub mod ids;
pub mod index;
pub mod init;
pub mod json;
pub mod meter;
pub mod model;
pub mod node_file;
pub mod ops;
pub mod project;
pub mod proposal;
pub mod registry;
pub mod settings;
pub mod softchecks;
pub mod spec;
pub mod statusline;
pub mod timeline;

#[cfg(test)]
pub(crate) mod testspec;
#[cfg(test)]
pub(crate) mod testutil;

/// Version shared by the plugin and the binary.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
