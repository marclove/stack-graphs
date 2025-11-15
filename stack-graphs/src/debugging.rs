// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2021, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

//! Debugging utilities for development.
//!
//! This module provides conditional debugging macros that can be enabled or disabled
//! at compile time using cargo features.
//!
//! ## The `copious_debugging!` Macro
//!
//! The [`copious_debugging!`][] macro works like `eprintln!`, printing debug messages to
//! stderr, but only when the `copious-debugging` cargo feature is enabled. When the
//! feature is disabled, the macro compiles to nothing, ensuring zero runtime overhead.
//!
//! ## Usage
//!
//! To enable copious debugging output, build with the feature flag:
//!
//! ```bash
//! cargo build --features copious-debugging
//! ```
//!
//! In your code, use the macro like `println!` or `eprintln!`:
//!
//! ```rust,ignore
//! use stack_graphs::copious_debugging;
//!
//! copious_debugging!("Processing node {:?}", node_id);
//! copious_debugging!("Found {} paths", path_count);
//! ```
//!
//! When the feature is enabled, these messages will be printed to stderr. When disabled,
//! the macro calls compile to nothing and have no performance impact.
//!
//! ## When to Use
//!
//! Use `copious_debugging!` for:
//! - Verbose trace-level logging during algorithm development
//! - Detailed output that would be too noisy for regular use
//! - Performance-sensitive code where you want zero overhead in release builds
//!
//! ## Implementation Details
//!
//! The macro has two implementations:
//! - **With feature enabled**: Expands to `eprintln!($($arg)*)`
//! - **With feature disabled**: Expands to an empty block `{}`
//!
//! This conditional compilation ensures that debugging code doesn't affect performance
//! or binary size in production builds.

/// Conditionally prints debugging output to stderr.
///
/// This macro behaves like `eprintln!` when the `copious-debugging` cargo feature is
/// enabled, and compiles to nothing when the feature is disabled.
///
/// # Examples
///
/// ```rust,ignore
/// use stack_graphs::copious_debugging;
///
/// let node_id = 42;
/// copious_debugging!("Processing node {}", node_id);
/// copious_debugging!("State: {:?}", state);
/// ```
///
/// # Feature Flag
///
/// Enable with:
/// ```bash
/// cargo build --features copious-debugging
/// ```
///
/// # Performance
///
/// When the feature is disabled (the default), this macro compiles to nothing and has
/// absolutely zero runtime cost. This makes it safe to leave debugging statements in
/// performance-critical code.
#[cfg(feature = "copious-debugging")]
#[macro_export]
macro_rules! copious_debugging {
    ($($arg:tt)*) => {{ ::std::eprintln!($($arg)*); }}

}

/// Conditionally prints debugging output to stderr (disabled version).
///
/// When the `copious-debugging` feature is not enabled, this macro expands to an empty
/// block and generates no code.
///
/// See the enabled version for documentation and examples.
#[cfg(not(feature = "copious-debugging"))]
#[macro_export]
macro_rules! copious_debugging {
    ($($arg:tt)*) => {};
}
