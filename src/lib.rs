// lib.rs — Library entry point
//
// Re-exports all modules so integration tests in tests/ can import them.
// The binary entry point (main.rs) declares modules directly; this file
// exposes the same modules for external test access.

pub mod lexer;
pub mod types;
pub mod dictionary;
pub mod stack;
pub mod arithmetic;
pub mod boolean;
pub mod strings;
pub mod control;
pub mod io_ops;
pub mod interpreter;