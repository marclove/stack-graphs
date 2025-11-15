// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2023, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

//! Filtering for selective serialization of stack graphs.
//!
//! This module provides the [`Filter`][] trait and implementations for controlling which
//! elements of a stack graph are included during serialization. Filters are useful for
//! reducing serialized data size by excluding irrelevant files, nodes, edges, or paths.
//!
//! ## Use Cases
//!
//! Filters allow you to:
//! - **Serialize specific files only**: Save just the files you need
//! - **Reduce data size**: Exclude large dependency graphs
//! - **Privacy/security**: Omit sensitive files from serialized output
//! - **Incremental updates**: Serialize only changed files
//!
//! ## Filter Trait
//!
//! The [`Filter`][] trait defines four methods for selective inclusion:
//!
//! - **`include_file`**: Should this file be included?
//! - **`include_node`**: Should this node be included?
//! - **`include_edge`**: Should this edge be included?
//! - **`include_partial_path`**: Should this partial path be included?
//!
//! Filters form a hierarchy: if a file is excluded, all its nodes are automatically
//! excluded. If a node is excluded, all edges touching it are excluded, and so on.
//!
//! ## Built-in Filters
//!
//! ### NoFilter
//!
//! The [`NoFilter`][] type includes everything:
//!
//! ```rust,ignore
//! use stack_graphs::serde::{StackGraph, NoFilter};
//!
//! let graph = /* ... */;
//! let serializable = StackGraph::from_graph_filter(&graph, &NoFilter);
//! ```
//!
//! ### FileFilter
//!
//! The [`FileFilter`][] type includes only specified files:
//!
//! ```rust,ignore
//! use stack_graphs::serde::{StackGraph, FileFilter};
//!
//! let files = vec!["src/main.rs", "src/lib.rs"];
//! let filter = FileFilter::new(files);
//! let serializable = StackGraph::from_graph_filter(&graph, &filter);
//! ```
//!
//! ### Function Filters
//!
//! Any function with signature `Fn(&StackGraph, &Handle<File>) -> bool` automatically
//! implements `Filter`:
//!
//! ```rust,ignore
//! use stack_graphs::serde::StackGraph;
//!
//! // Include only files in src/ directory
//! let filter = |graph: &StackGraph, file: &Handle<File>| {
//!     graph[*file].name().starts_with("src/")
//! };
//! let serializable = StackGraph::from_graph_filter(&graph, &filter);
//! ```
//!
//! ## Custom Filters
//!
//! Implement the [`Filter`][] trait for fine-grained control:
//!
//! ```rust,ignore
//! use stack_graphs::serde::Filter;
//! use stack_graphs::graph::{StackGraph, Node, File};
//! use stack_graphs::arena::Handle;
//! use stack_graphs::partial::{PartialPath, PartialPaths};
//!
//! struct MyFilter;
//!
//! impl Filter for MyFilter {
//!     fn include_file(&self, graph: &StackGraph, file: &Handle<File>) -> bool {
//!         // Include all files
//!         true
//!     }
//!
//!     fn include_node(&self, graph: &StackGraph, node: &Handle<Node>) -> bool {
//!         // Exclude internal nodes
//!         !graph[*node].is_internal()
//!     }
//!
//!     fn include_edge(&self, graph: &StackGraph, source: &Handle<Node>, sink: &Handle<Node>) -> bool {
//!         // Include all edges (that connect non-excluded nodes)
//!         true
//!     }
//!
//!     fn include_partial_path(
//!         &self,
//!         graph: &StackGraph,
//!         paths: &PartialPaths,
//!         path: &PartialPath,
//!     ) -> bool {
//!         // Include all paths (that use non-excluded nodes/edges)
//!         true
//!     }
//! }
//! ```
//!
//! ## Performance
//!
//! Filters are called frequently during serialization. Keep filter logic simple and fast:
//! - Pre-compute sets of included files rather than checking strings repeatedly
//! - Use hash sets for O(1) lookups
//! - Avoid expensive graph traversals in filter methods
//!
//! ## Example: Filtering by File Extension
//!
//! ```rust,ignore
//! use stack_graphs::serde::{StackGraph, Filter};
//! use stack_graphs::graph::{StackGraph as SG, File, Node};
//! use stack_graphs::arena::Handle;
//! use stack_graphs::partial::{PartialPath, PartialPaths};
//!
//! struct ExtensionFilter {
//!     extensions: Vec<String>,
//! }
//!
//! impl Filter for ExtensionFilter {
//!     fn include_file(&self, graph: &SG, file: &Handle<File>) -> bool {
//!         let name = graph[*file].name();
//!         self.extensions.iter().any(|ext| name.ends_with(ext))
//!     }
//!
//!     fn include_node(&self, _graph: &SG, _node: &Handle<Node>) -> bool {
//!         true
//!     }
//!
//!     fn include_edge(&self, _graph: &SG, _source: &Handle<Node>, _sink: &Handle<Node>) -> bool {
//!         true
//!     }
//!
//!     fn include_partial_path(&self, _graph: &SG, _paths: &PartialPaths, _path: &PartialPath) -> bool {
//!         true
//!     }
//! }
//!
//! // Use the filter
//! let filter = ExtensionFilter {
//!     extensions: vec![".rs".to_string(), ".toml".to_string()],
//! };
//! let serializable = StackGraph::from_graph_filter(&graph, &filter);
//! ```

use itertools::Itertools;

use crate::arena::Handle;
use crate::graph::File;
use crate::graph::Node;
use crate::graph::StackGraph;
use crate::partial::PartialPath;
use crate::partial::PartialPaths;

/// Trait for filtering stack graph elements during serialization.
///
/// Implement this trait to control which files, nodes, edges, and paths are included
/// when serializing a stack graph. Filters form a hierarchy where excluding a higher-level
/// element (like a file) automatically excludes all dependent elements (like its nodes).
///
/// ## Filter Hierarchy
///
/// 1. **Files**: If a file is excluded, all its nodes/edges/paths are excluded
/// 2. **Nodes**: If a node is excluded, all edges touching it are excluded
/// 3. **Edges**: If an edge is excluded, paths using it may be excluded
/// 4. **Paths**: Final decision on partial path inclusion
///
/// ## Implementation Notes
///
/// - All methods should be pure functions (no side effects)
/// - Methods are called frequently; keep them fast
/// - Return `true` to include an element, `false` to exclude it
/// - Exclusions cascade: excluded elements exclude their dependents
///
/// ## Example
///
/// ```rust,ignore
/// use stack_graphs::serde::Filter;
///
/// struct TestFilesOnly;
///
/// impl Filter for TestFilesOnly {
///     fn include_file(&self, graph: &StackGraph, file: &Handle<File>) -> bool {
///         graph[*file].name().contains("test")
///     }
///
///     // Default implementations for other methods include everything
///     fn include_node(&self, _: &StackGraph, _: &Handle<Node>) -> bool { true }
///     fn include_edge(&self, _: &StackGraph, _: &Handle<Node>, _: &Handle<Node>) -> bool { true }
///     fn include_partial_path(&self, _: &StackGraph, _: &PartialPaths, _: &PartialPath) -> bool { true }
/// }
/// ```
pub trait Filter {
    /// Returns whether elements for the given file should be included.
    ///
    /// This is the primary filter method. If this returns `false` for a file,
    /// all nodes, edges, and paths associated with that file are automatically excluded.
    ///
    /// # Parameters
    ///
    /// - `graph`: The stack graph being filtered
    /// - `file`: Handle to the file to check
    ///
    /// # Returns
    ///
    /// `true` to include the file and allow its elements, `false` to exclude everything from this file
    fn include_file(&self, graph: &StackGraph, file: &Handle<File>) -> bool;

    /// Returns whether the given node should be included.
    ///
    /// This method is only called for nodes in files that passed `include_file`.
    /// Nodes of excluded files are never passed to this method.
    ///
    /// # Parameters
    ///
    /// - `graph`: The stack graph being filtered
    /// - `node`: Handle to the node to check
    ///
    /// # Returns
    ///
    /// `true` to include the node, `false` to exclude it
    fn include_node(&self, graph: &StackGraph, node: &Handle<Node>) -> bool;

    /// Returns whether the given edge should be included.
    ///
    /// This method is only called for edges between nodes that both passed `include_node`.
    /// Edges via excluded nodes are never passed to this method.
    ///
    /// # Parameters
    ///
    /// - `graph`: The stack graph being filtered
    /// - `source`: Handle to the edge's source node
    /// - `sink`: Handle to the edge's sink node
    ///
    /// # Returns
    ///
    /// `true` to include the edge, `false` to exclude it
    fn include_edge(&self, graph: &StackGraph, source: &Handle<Node>, sink: &Handle<Node>) -> bool;

    /// Returns whether the given partial path should be included.
    ///
    /// This method is only called for paths that use included nodes and edges.
    /// Paths via excluded nodes or edges are never passed to this method.
    ///
    /// # Parameters
    ///
    /// - `graph`: The stack graph being filtered
    /// - `paths`: The partial paths container
    /// - `path`: The partial path to check
    ///
    /// # Returns
    ///
    /// `true` to include the path, `false` to exclude it
    fn include_partial_path(
        &self,
        graph: &StackGraph,
        paths: &PartialPaths,
        path: &PartialPath,
    ) -> bool;
}

/// Blanket implementation of [`Filter`][] for file-filtering functions.
///
/// Any function with signature `Fn(&StackGraph, &Handle<File>) -> bool` automatically
/// implements the full `Filter` trait, with the function determining file inclusion
/// and default implementations accepting all nodes/edges/paths.
///
/// This allows using simple closures as filters:
///
/// ```rust,ignore
/// let filter = |graph: &StackGraph, file: &Handle<File>| {
///     graph[*file].name().starts_with("src/")
/// };
/// ```
impl<F> Filter for F
where
    F: Fn(&StackGraph, &Handle<File>) -> bool,
{
    fn include_file(&self, graph: &StackGraph, file: &Handle<File>) -> bool {
        self(graph, file)
    }

    fn include_node(&self, _graph: &StackGraph, _node: &Handle<Node>) -> bool {
        true
    }

    fn include_edge(
        &self,
        _graph: &StackGraph,
        _source: &Handle<Node>,
        _sink: &Handle<Node>,
    ) -> bool {
        true
    }

    fn include_partial_path(
        &self,
        _graph: &StackGraph,
        _paths: &PartialPaths,
        _path: &PartialPath,
    ) -> bool {
        true
    }
}

/// A filter that includes all elements.
///
/// This is the default filter that passes all files, nodes, edges, and paths through
/// without exclusion. Use this when you want to serialize the complete graph.
///
/// # Example
///
/// ```rust,ignore
/// use stack_graphs::serde::{StackGraph, NoFilter};
///
/// let graph = /* ... */;
/// let serializable = StackGraph::from_graph_filter(&graph, &NoFilter);
/// // All elements from the graph will be serialized
/// ```
pub struct NoFilter;

impl Filter for NoFilter {
    fn include_file(&self, _graph: &StackGraph, _file: &Handle<File>) -> bool {
        true
    }

    fn include_node(&self, _graph: &StackGraph, _node: &Handle<Node>) -> bool {
        true
    }

    fn include_edge(
        &self,
        _graph: &StackGraph,
        _source: &Handle<Node>,
        _sink: &Handle<Node>,
    ) -> bool {
        true
    }

    fn include_partial_path(
        &self,
        _graph: &StackGraph,
        _paths: &PartialPaths,
        _path: &PartialPath,
    ) -> bool {
        true
    }
}

/// A filter that includes only a single specific file.
///
/// This filter includes all elements from one file and excludes everything else.
/// Useful when you need to serialize or process just one file from a larger graph.
///
/// # Example
///
/// ```rust,ignore
/// use stack_graphs::serde::{StackGraph, FileFilter};
///
/// let graph = /* ... */;
/// let file = graph.get_or_create_file("src/main.rs");
///
/// // Serialize only src/main.rs
/// let filter = FileFilter(file);
/// let serializable = StackGraph::from_graph_filter(&graph, &filter);
/// ```
///
/// # Note
///
/// For filtering multiple files by name, use a closure filter instead:
///
/// ```rust,ignore
/// let files_to_include = HashSet::from(["src/main.rs", "src/lib.rs"]);
/// let filter = |graph: &StackGraph, file: &Handle<File>| {
///     files_to_include.contains(graph[*file].name())
/// };
/// ```
pub struct FileFilter(pub Handle<File>);

impl Filter for FileFilter {
    fn include_file(&self, _graph: &StackGraph, file: &Handle<File>) -> bool {
        *file == self.0
    }

    fn include_node(&self, _graph: &StackGraph, _node: &Handle<Node>) -> bool {
        true
    }

    fn include_edge(
        &self,
        _graph: &StackGraph,
        _source: &Handle<Node>,
        _sink: &Handle<Node>,
    ) -> bool {
        true
    }

    fn include_partial_path(
        &self,
        _graph: &StackGraph,
        _paths: &PartialPaths,
        _path: &PartialPath,
    ) -> bool {
        true
    }
}

/// Internal filter wrapper that enforces the filter hierarchy.
///
/// This filter wraps another filter and ensures that filter decisions cascade properly:
/// - Nodes from excluded files are automatically excluded
/// - Edges between excluded nodes are automatically excluded
/// - Paths using excluded edges are automatically excluded
///
/// This wrapper is used internally by the serialization code to ensure consistency.
/// You typically don't need to use this directly; the serialization API handles it.
///
/// # Implementation Details
///
/// For each element, this filter:
/// 1. Checks that all parent elements (files for nodes, nodes for edges, etc.) are included
/// 2. Then calls the wrapped filter's method
/// 3. Returns `true` only if both checks pass
///
/// This prevents inconsistent filter results where a node might be included but its
/// file is excluded, for example.
pub(crate) struct ImplicationFilter<'a>(pub &'a dyn Filter);

impl Filter for ImplicationFilter<'_> {
    fn include_file(&self, graph: &StackGraph, file: &Handle<File>) -> bool {
        self.0.include_file(graph, file)
    }

    fn include_node(&self, graph: &StackGraph, node: &Handle<Node>) -> bool {
        graph[*node]
            .id()
            .file()
            .map_or(true, |f| self.include_file(graph, &f))
            && self.0.include_node(graph, node)
    }

    fn include_edge(&self, graph: &StackGraph, source: &Handle<Node>, sink: &Handle<Node>) -> bool {
        self.include_node(graph, source)
            && self.include_node(graph, sink)
            && self.0.include_edge(graph, source, sink)
    }

    fn include_partial_path(
        &self,
        graph: &StackGraph,
        paths: &PartialPaths,
        path: &PartialPath,
    ) -> bool {
        let super_ok = self.0.include_partial_path(graph, paths, path);
        if !super_ok {
            return false;
        }
        let all_included_edges = path
            .edges
            .iter_unordered(paths)
            .map(|e| graph.node_for_id(e.source_node_id).unwrap())
            .chain(std::iter::once(path.end_node))
            .tuple_windows()
            .all(|(source, sink)| self.include_edge(graph, &source, &sink));
        if !all_included_edges {
            return false;
        }
        true
    }
}
