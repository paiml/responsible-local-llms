//! Chapter 3 demo logic (bias, safety and privacy, tested). Each lesson is a module with a
//! `run` function; the demo bins in `src/bin/` call it, assert the contract and print the
//! verdict. Later parts of the chapter add their modules to this list.
//! The checker, schema and runner live in `rll-evals`.

pub mod bias;
pub mod error;
pub mod injection;
pub mod responder;
