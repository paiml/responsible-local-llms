//! Evals as code. A test set is a jsonl file, one [`Case`] per line: a prompt plus the
//! properties its answer must have (never an exact string). [`run`] asks a [`Responder`]
//! every case once per seed and reports pass rates, so a flaky answer shows up as a
//! rate between 0 and 1 instead of one lucky green run.
//!
//! The public surface is small on purpose:
//! - [`parse_jsonl`] reads and validates a set (errors name the line).
//! - [`Case::check`] and [`Property::check`] judge one answer.
//! - [`run`] and [`Report`] measure pass rates for a seed list and a temperature.
//! - [`Responder`] is the one seam: [`ClientResponder`] drives a real runtime through
//!   `rll-client`, [`RecordedVariants`] replays recorded answers offline.

pub mod check;
pub mod runner;
pub mod schema;

pub use runner::{run, CaseStat, ClientResponder, RecordedVariants, Report, Responder, RunError};
pub use schema::{parse_jsonl, Case, Property, SchemaError};
