// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2023, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

//! Serialization and deserialization support for stack graphs.
//!
//! This module provides serializable representations of stack graph data structures,
//! allowing stack graphs, partial paths, and related data to be saved to disk and loaded
//! back later. This is essential for caching and incremental analysis.
//!
//! ## Overview
//!
//! The stack graphs library uses arena allocation and handle-based references internally,
//! which cannot be directly serialized. This module provides alternative representations
//! that:
//! - Can be serialized using `serde` or `bincode`
//! - Preserve all necessary information
//! - Can be loaded back into the arena-based structures
//!
//! ## Key Components
//!
//! ### Graph Serialization
//!
//! The [`graph`][] module provides serializable versions of:
//! - [`StackGraph`][graph::StackGraph]: The main stack graph structure
//! - [`Files`][graph::Files]: File information
//! - [`Nodes`][graph::Nodes]: All nodes in the graph
//! - [`Edges`][graph::Edges]: All edges in the graph
//!
//! ### Partial Paths Serialization
//!
//! The [`partial`][] module provides:
//! - [`PartialPaths`][partial::PartialPaths]: Serializable partial path database
//! - [`PartialPath`][partial::PartialPath]: Individual partial paths
//!
//! ### Stitching Data Serialization
//!
//! The [`stitching`][] module provides:
//! - [`Database`][stitching::Database]: Serializable path database
//! - Pre-computed partial paths for efficient querying
//!
//! ### Filtering
//!
//! The [`filter`][] module provides:
//! - [`Filter`][]: Trait for selectively serializing graph elements
//! - [`NoFilter`][]: Serialize everything
//! - [`FileFilter`][]: Serialize specific files only
//!
//! ## Usage Example
//!
//! ```rust,ignore
//! use stack_graphs::graph::StackGraph;
//! use stack_graphs::serde;
//!
//! // Create and build a stack graph
//! let mut graph = StackGraph::new();
//! // ... build the graph ...
//!
//! // Serialize to a format that can be saved
//! let serializable = serde::StackGraph::from_graph(&graph);
//!
//! // Save to JSON (requires serde feature)
//! #[cfg(feature = "serde")]
//! {
//!     let json = serde_json::to_string(&serializable)?;
//!     std::fs::write("graph.json", json)?;
//! }
//!
//! // Or save to bincode (requires bincode feature)
//! #[cfg(feature = "bincode")]
//! {
//!     let encoded = bincode::encode_to_vec(&serializable, bincode::config::standard())?;
//!     std::fs::write("graph.bin", encoded)?;
//! }
//!
//! // Later, load it back
//! let mut new_graph = StackGraph::new();
//! serializable.load_into(&mut new_graph)?;
//! ```
//!
//! ## Cargo Features
//!
//! This module requires one of these features to be enabled:
//! - **`serde`**: Enables JSON/YAML serialization via `serde`
//! - **`bincode`**: Enables binary serialization via `bincode`
//!
//! Enable in your `Cargo.toml`:
//! ```toml
//! [dependencies]
//! stack-graphs = { version = "...", features = ["serde"] }
//! ```
//!
//! ## Performance Considerations
//!
//! ### Binary vs Text Formats
//!
//! - **Bincode**: Faster and more compact, but not human-readable
//! - **JSON**: Human-readable, but larger files and slower serialization
//!
//! ### Filtering
//!
//! Use filters when serializing to reduce file size:
//!
//! ```rust,ignore
//! use stack_graphs::serde::{StackGraph, FileFilter};
//!
//! // Only serialize specific files
//! let filter = FileFilter::new(vec!["src/main.rs", "src/lib.rs"]);
//! let serializable = StackGraph::from_graph_filter(&graph, &filter);
//! ```
//!
//! ### Incremental Saving
//!
//! For large repositories:
//! 1. Serialize each file's data separately
//! 2. Only reserialize files that changed
//! 3. Use bincode for maximum performance
//!
//! ## Error Handling
//!
//! Loading serialized data can fail if:
//! - File references are invalid
//! - Node IDs are out of range
//! - Data is corrupted
//!
//! Always handle [`Error`][graph::Error] when loading:
//!
//! ```rust,ignore
//! match serializable.load_into(&mut graph) {
//!     Ok(()) => println!("Loaded successfully"),
//!     Err(e) => eprintln!("Failed to load: {}", e),
//! }
//! ```
//!
//! ## Thread Safety
//!
//! Serializable types implement `Send` and `Sync` when appropriate, making them
//! safe to use across threads for parallel serialization.

mod filter;
mod graph;
mod partial;
mod stitching;

pub use filter::*;
pub use graph::*;
pub use partial::*;
pub use stitching::*;
