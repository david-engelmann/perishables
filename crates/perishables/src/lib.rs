//! Command-line adapter library for `perishables`.
//!
//! Exposes deterministic shell completion scripts in [`completions`], the
//! Attested Computation execution engine in [`execute`], and structured report
//! formatters in [`render`].

pub mod completions;
pub mod execute;
pub mod render;
