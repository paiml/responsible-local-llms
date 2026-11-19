//! Chapter 2 demo logic (evals as code). Each lesson is a module with a `run` function;
//! the demo bins in `src/bin/` call it, assert the contract and print the verdict.
//! The checker, schema and runner live in `rll-evals`.

pub mod flakiness;
pub mod test_set_spec;
