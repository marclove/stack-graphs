// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2022, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

//! Assertions for testing and validating stack graphs.
//!
//! This module provides types for declaring and verifying assertions about name resolution
//! in stack graphs. Assertions are used primarily for testing language implementations to
//! ensure that references resolve to the correct definitions.
//!
//! ## Use Cases
//!
//! Assertions allow you to:
//! - **Test language implementations**: Verify that TSG rules correctly resolve names
//! - **Validate name resolution**: Ensure references find their intended definitions
//! - **Check symbol presence**: Verify that specific symbols are defined or referenced
//! - **Regression testing**: Ensure changes don't break existing name resolution
//!
//! ## Assertion Types
//!
//! Three types of assertions are supported:
//!
//! ### 1. Defined Assertions
//!
//! Assert that a reference at a given position resolves to specific definitions:
//!
//! ```ignore
//! // In a test file with annotation:
//! result = greet("World")
//! //       ^ defined: 5
//! ```
//!
//! This asserts that the reference to `greet` on this line resolves to the definition
//! on line 5.
//!
//! ### 2. Defines Assertions
//!
//! Assert that a position contains definitions for specific symbols:
//!
//! ```ignore
//! def my_function(param1, param2):
//! //  ^ defines: my_function
//! ```
//!
//! ### 3. Refers Assertions
//!
//! Assert that a position contains references to specific symbols:
//!
//! ```ignore
//! print(my_variable)
//! //    ^ refers: my_variable
//! ```
//!
//! ## Assertion Workflow
//!
//! 1. **Parse annotations** from test files to create [`Assertion`][] objects
//! 2. **Build the stack graph** for the test file
//! 3. **Run assertions** using [`Assertion::run()`][]
//! 4. **Check results** - assertions return `Ok(())` on success or [`AssertionError`][] on failure
//!
//! ## Example Usage
//!
//! ```rust,ignore
//! use stack_graphs::assert::{Assertion, AssertionSource, AssertionTarget};
//! use stack_graphs::graph::StackGraph;
//! use stack_graphs::partial::PartialPaths;
//! use stack_graphs::stitching::{Database, StitcherConfig};
//! use stack_graphs::NoCancellation;
//!
//! let graph = /* ... build stack graph ... */;
//! let mut partials = PartialPaths::new();
//! let mut db = Database::new();
//!
//! // Create an assertion
//! let assertion = Assertion::Defined {
//!     source: AssertionSource { file, position },
//!     targets: vec![AssertionTarget { file, line: 5 }],
//! };
//!
//! // Run the assertion
//! assertion.run(
//!     &graph,
//!     &mut partials,
//!     &mut db,
//!     StitcherConfig::default(),
//!     &NoCancellation,
//! )?;
//! ```

use itertools::Itertools;
use lsp_positions::Position;

use crate::arena::Handle;
use crate::graph::File;
use crate::graph::Node;
use crate::graph::StackGraph;
use crate::graph::Symbol;
use crate::partial::PartialPath;
use crate::partial::PartialPaths;
use crate::stitching::Database;
use crate::stitching::DatabaseCandidates;
use crate::stitching::ForwardPartialPathStitcher;
use crate::stitching::StitcherConfig;
use crate::CancellationError;
use crate::CancellationFlag;

/// A stack graph assertion to be verified.
///
/// An assertion specifies a condition that should hold true in a stack graph,
/// such as "this reference should resolve to that definition" or "this position
/// should define a specific symbol".
///
/// # Variants
///
/// - **`Defined`**: Asserts that references at a source position resolve to specific
///   target definitions. This is the most common assertion type, used to verify that
///   name resolution works correctly.
///
/// - **`Defines`**: Asserts that a source position contains definitions for specific
///   symbols. Used to verify that definitions are created with the correct symbol names.
///
/// - **`Refers`**: Asserts that a source position contains references to specific
///   symbols. Used to verify that references are created with the correct symbol names.
///
/// # Example
///
/// ```rust,ignore
/// // Assert that a reference resolves to a definition on line 10
/// let assertion = Assertion::Defined {
///     source: AssertionSource {
///         file: file_handle,
///         position: Position { line: 5, column: ColumnIndex::from_utf8(10) },
///     },
///     targets: vec![
///         AssertionTarget {
///             file: file_handle,
///             line: 10,
///         }
///     ],
/// };
/// ```
#[derive(Debug, Clone)]
pub enum Assertion {
    /// Asserts that references at the source position resolve to the specified targets.
    ///
    /// This is used in test annotations like:
    /// ```ignore
    /// result = my_function()
    /// //       ^ defined: 5
    /// ```
    Defined {
        /// The position containing the reference(s) to check
        source: AssertionSource,
        /// The expected definition target(s) the reference should resolve to
        targets: Vec<AssertionTarget>,
    },

    /// Asserts that the source position contains definitions for the specified symbols.
    ///
    /// This is used in test annotations like:
    /// ```ignore
    /// def my_function():
    /// //  ^ defines: my_function
    /// ```
    Defines {
        /// The position that should contain definition(s)
        source: AssertionSource,
        /// The symbols that should be defined at this position
        symbols: Vec<Handle<Symbol>>,
    },

    /// Asserts that the source position contains references to the specified symbols.
    ///
    /// This is used in test annotations like:
    /// ```ignore
    /// print(my_variable)
    /// //    ^ refers: my_variable
    /// ```
    Refers {
        /// The position that should contain reference(s)
        source: AssertionSource,
        /// The symbols that should be referenced at this position
        symbols: Vec<Handle<Symbol>>,
    },
}

/// The source position of an assertion.
///
/// Identifies a specific location in a file where an assertion should be checked.
/// The position typically corresponds to an annotated line in a test file.
///
/// # Example
///
/// For a test file annotation like:
/// ```ignore
/// result = greet("World")
/// //       ^ defined: 5
/// ```
///
/// The `AssertionSource` would identify the position of the `^` marker (pointing to `greet`).
#[derive(Debug, Clone)]
pub struct AssertionSource {
    /// The file containing the assertion
    pub file: Handle<File>,
    /// The position in the file (line and column)
    pub position: Position,
}

impl AssertionSource {
    /// Returns an iterator over all definition nodes at this position.
    ///
    /// Finds all nodes in the stack graph that:
    /// - Are marked as definitions (`is_definition()`)
    /// - Have source information whose span contains this position
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let source = AssertionSource { file, position };
    /// for def in source.iter_definitions(&graph) {
    ///     println!("Found definition: {:?}", graph[def]);
    /// }
    /// ```
    pub fn iter_definitions<'a>(
        &'a self,
        graph: &'a StackGraph,
    ) -> impl Iterator<Item = Handle<Node>> + 'a {
        graph.nodes_for_file(self.file).filter(move |n| {
            graph[*n].is_definition()
                && graph
                    .source_info(*n)
                    .map(|s| s.span.contains(&self.position))
                    .unwrap_or(false)
        })
    }

    /// Returns an iterator over all reference nodes at this position.
    ///
    /// Finds all nodes in the stack graph that:
    /// - Are marked as references (`is_reference()`)
    /// - Have source information whose span contains this position
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let source = AssertionSource { file, position };
    /// for ref_node in source.iter_references(&graph) {
    ///     println!("Found reference: {:?}", graph[ref_node]);
    /// }
    /// ```
    pub fn iter_references<'a>(
        &'a self,
        graph: &'a StackGraph,
    ) -> impl Iterator<Item = Handle<Node>> + 'a {
        graph.nodes_for_file(self.file).filter(move |n| {
            graph[*n].is_reference()
                && graph
                    .source_info(*n)
                    .map(|s| s.span.contains(&self.position))
                    .unwrap_or(false)
        })
    }

    /// Returns a displayable representation of this assertion source.
    ///
    /// The format is `filename:line:column` (with 1-based line and column numbers).
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let source = AssertionSource { file, position };
    /// println!("Assertion at: {}", source.display(&graph));
    /// // Output: "example.py:10:5"
    /// ```
    pub fn display<'a>(&'a self, graph: &'a StackGraph) -> impl std::fmt::Display + 'a {
        struct Displayer<'a>(&'a AssertionSource, &'a StackGraph);
        impl std::fmt::Display for Displayer<'_> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(
                    f,
                    "{}:{}:{}",
                    self.1[self.0.file],
                    self.0.position.line + 1,
                    self.0.position.column.grapheme_offset + 1
                )
            }
        }
        Displayer(self, graph)
    }
}

/// The target line of a "defined" assertion.
///
/// Specifies which line a reference should resolve to. The target matches a definition
/// if the definition's span includes the specified line.
///
/// # Example
///
/// For the annotation:
/// ```ignore
/// result = greet("World")
/// //       ^ defined: 5
/// ```
///
/// The `AssertionTarget` would be `{ file, line: 5 }`, indicating that the reference
/// should resolve to a definition whose span includes line 5.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AssertionTarget {
    /// The file containing the expected definition
    pub file: Handle<File>,
    /// The line number (0-based) that should be within the definition's span
    pub line: usize,
}

impl AssertionTarget {
    /// Checks if this target matches a given node in the stack graph.
    ///
    /// A match occurs when:
    /// - The node is in the same file as the target
    /// - The target's line falls within the node's source span
    ///
    /// # Parameters
    ///
    /// - `node`: The node to check
    /// - `graph`: The stack graph containing the node
    ///
    /// # Returns
    ///
    /// `true` if the node matches this target, `false` otherwise
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let target = AssertionTarget { file, line: 10 };
    /// if target.matches_node(definition_node, &graph) {
    ///     println!("Definition is on the expected line!");
    /// }
    /// ```
    pub fn matches_node(&self, node: Handle<Node>, graph: &StackGraph) -> bool {
        let file = graph[node].file().unwrap();
        let si = graph.source_info(node).unwrap();
        let start_line = si.span.start.line;
        let end_line = si.span.end.line;
        file == self.file && start_line <= self.line && self.line <= end_line
    }
}

/// Errors that occur when an assertion fails.
///
/// These errors describe what went wrong when verifying an assertion, providing
/// detailed information about missing or unexpected nodes.
#[derive(Clone)]
pub enum AssertionError {
    /// No reference nodes were found at the assertion source position.
    ///
    /// This occurs when a "defined" assertion points to a position that doesn't
    /// contain any reference nodes, making the assertion impossible to verify.
    NoReferences {
        /// The source position where no references were found
        source: AssertionSource,
    },

    /// References resolved to incorrect definitions.
    ///
    /// This occurs when:
    /// - Some expected targets were not reached by any paths
    /// - Some paths reached unexpected targets not in the expected list
    IncorrectlyDefined {
        /// The source position of the assertion
        source: AssertionSource,
        /// The reference nodes that were checked
        references: Vec<Handle<Node>>,
        /// Expected targets that were not reached
        missing_targets: Vec<AssertionTarget>,
        /// Paths that reached unexpected targets
        unexpected_paths: Vec<PartialPath>,
    },

    /// The position has incorrect definitions.
    ///
    /// This occurs when a "defines" assertion fails because:
    /// - Some expected symbols are not defined at the position
    /// - Some unexpected symbols are defined at the position
    IncorrectDefinitions {
        /// The source position of the assertion
        source: AssertionSource,
        /// Symbols that were expected but not found
        missing_symbols: Vec<Handle<Symbol>>,
        /// Symbols that were found but not expected
        unexpected_symbols: Vec<Handle<Symbol>>,
    },

    /// The position has incorrect references.
    ///
    /// This occurs when a "refers" assertion fails because:
    /// - Some expected symbols are not referenced at the position
    /// - Some unexpected symbols are referenced at the position
    IncorrectReferences {
        /// The source position of the assertion
        source: AssertionSource,
        /// Symbols that were expected but not found
        missing_symbols: Vec<Handle<Symbol>>,
        /// Symbols that were found but not expected
        unexpected_symbols: Vec<Handle<Symbol>>,
    },

    /// The assertion was cancelled before completion.
    ///
    /// This occurs when the cancellation flag is triggered during path finding.
    Cancelled(CancellationError),
}

impl From<CancellationError> for AssertionError {
    fn from(value: CancellationError) -> Self {
        Self::Cancelled(value)
    }
}

impl Assertion {
    /// Runs this assertion against a stack graph.
    ///
    /// Verifies that the assertion holds true by checking the stack graph and, for
    /// "defined" assertions, performing path stitching to find complete paths from
    /// references to definitions.
    ///
    /// # Parameters
    ///
    /// - `graph`: The stack graph to check
    /// - `partials`: Partial paths container for path finding
    /// - `db`: Database of precomputed partial paths
    /// - `stitcher_config`: Configuration for the path stitcher
    /// - `cancellation_flag`: Flag to check for cancellation during long operations
    ///
    /// # Returns
    ///
    /// - `Ok(())` if the assertion passes
    /// - `Err(AssertionError)` if the assertion fails, with details about what went wrong
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use stack_graphs::assert::Assertion;
    /// use stack_graphs::stitching::StitcherConfig;
    /// use stack_graphs::NoCancellation;
    ///
    /// let assertion = /* ... create assertion ... */;
    /// let mut partials = PartialPaths::new();
    /// let mut db = Database::new();
    ///
    /// match assertion.run(
    ///     &graph,
    ///     &mut partials,
    ///     &mut db,
    ///     StitcherConfig::default(),
    ///     &NoCancellation,
    /// ) {
    ///     Ok(()) => println!("Assertion passed!"),
    ///     Err(e) => println!("Assertion failed: {:?}", e),
    /// }
    /// ```
    ///
    /// # Performance
    ///
    /// - **Defined** assertions perform path stitching, which can be expensive
    /// - **Defines** and **Refers** assertions only check local nodes (fast)
    pub fn run(
        &self,
        graph: &StackGraph,
        partials: &mut PartialPaths,
        db: &mut Database,
        stitcher_config: StitcherConfig,
        cancellation_flag: &dyn CancellationFlag,
    ) -> Result<(), AssertionError> {
        match self {
            Self::Defined { source, targets } => self.run_defined(
                graph,
                partials,
                db,
                source,
                targets,
                stitcher_config,
                cancellation_flag,
            ),
            Self::Defines { source, symbols } => self.run_defines(graph, source, symbols),
            Self::Refers { source, symbols } => self.run_refers(graph, source, symbols),
        }
    }

    /// Runs a "defined" assertion by finding all paths from references to definitions.
    ///
    /// This method:
    /// 1. Finds all reference nodes at the source position
    /// 2. Performs path stitching to find complete paths from each reference
    /// 3. Filters out shadowed paths
    /// 4. Checks that paths reach exactly the expected targets
    fn run_defined(
        &self,
        graph: &StackGraph,
        partials: &mut PartialPaths,
        db: &mut Database,
        source: &AssertionSource,
        expected_targets: &Vec<AssertionTarget>,
        stitcher_config: StitcherConfig,
        cancellation_flag: &dyn CancellationFlag,
    ) -> Result<(), AssertionError> {
        // Find all reference nodes at the source position
        let references = source.iter_references(graph).collect::<Vec<_>>();
        if references.is_empty() {
            return Err(AssertionError::NoReferences {
                source: source.clone(),
            });
        }

        // Find all complete paths from the references
        let mut actual_paths = Vec::new();
        for reference in &references {
            let mut reference_paths = Vec::new();

            // Use path stitching to find all complete paths from this reference
            ForwardPartialPathStitcher::find_all_complete_partial_paths(
                &mut DatabaseCandidates::new(graph, partials, db),
                vec![*reference],
                stitcher_config,
                cancellation_flag,
                |_, _, p| {
                    reference_paths.push(p.clone());
                },
            )?;

            // Filter out shadowed paths (keep only non-shadowed ones)
            // A path is shadowed if another path with the same start and end
            // is more specific (has more precise scope information)
            for reference_path in &reference_paths {
                if reference_paths
                    .iter()
                    .all(|other| !other.shadows(partials, reference_path))
                {
                    actual_paths.push(reference_path.clone());
                }
            }
        }

        // Check that actual paths match expected targets
        let missing_targets = expected_targets
            .iter()
            .filter(|t| {
                !actual_paths
                    .iter()
                    .any(|p| t.matches_node(p.end_node, graph))
            })
            .cloned()
            .unique()
            .collect::<Vec<_>>();

        let unexpected_paths = actual_paths
            .iter()
            .filter(|p| {
                !expected_targets
                    .iter()
                    .any(|t| t.matches_node(p.end_node, graph))
            })
            .cloned()
            .collect::<Vec<_>>();

        if !missing_targets.is_empty() || !unexpected_paths.is_empty() {
            return Err(AssertionError::IncorrectlyDefined {
                source: source.clone(),
                references,
                missing_targets,
                unexpected_paths,
            });
        }

        Ok(())
    }

    /// Runs a "defines" assertion by checking symbols at a position.
    ///
    /// This method:
    /// 1. Finds all definition nodes at the source position
    /// 2. Extracts their symbols
    /// 3. Verifies they match the expected symbols exactly
    fn run_defines(
        &self,
        graph: &StackGraph,
        source: &AssertionSource,
        expected_symbols: &Vec<Handle<Symbol>>,
    ) -> Result<(), AssertionError> {
        // Get symbols from all definitions at this position
        let actual_symbols = source
            .iter_definitions(graph)
            .filter_map(|d| graph[d].symbol())
            .collect::<Vec<_>>();

        // Find discrepancies
        let missing_symbols = expected_symbols
            .iter()
            .filter(|x| !actual_symbols.contains(*x))
            .cloned()
            .unique()
            .collect::<Vec<_>>();

        let unexpected_symbols = actual_symbols
            .iter()
            .filter(|x| !expected_symbols.contains(*x))
            .cloned()
            .unique()
            .collect::<Vec<_>>();

        if !missing_symbols.is_empty() || !unexpected_symbols.is_empty() {
            return Err(AssertionError::IncorrectDefinitions {
                source: source.clone(),
                missing_symbols,
                unexpected_symbols,
            });
        }

        Ok(())
    }

    /// Runs a "refers" assertion by checking symbols at a position.
    ///
    /// This method:
    /// 1. Finds all reference nodes at the source position
    /// 2. Extracts their symbols
    /// 3. Verifies they match the expected symbols exactly
    fn run_refers(
        &self,
        graph: &StackGraph,
        source: &AssertionSource,
        expected_symbols: &Vec<Handle<Symbol>>,
    ) -> Result<(), AssertionError> {
        // Get symbols from all references at this position
        let actual_symbols = source
            .iter_references(graph)
            .filter_map(|d| graph[d].symbol())
            .collect::<Vec<_>>();

        // Find discrepancies
        let missing_symbols = expected_symbols
            .iter()
            .filter(|x| !actual_symbols.contains(*x))
            .cloned()
            .unique()
            .collect::<Vec<_>>();

        let unexpected_symbols = actual_symbols
            .iter()
            .filter(|x| !expected_symbols.contains(*x))
            .cloned()
            .unique()
            .collect::<Vec<_>>();

        if !missing_symbols.is_empty() || !unexpected_symbols.is_empty() {
            return Err(AssertionError::IncorrectReferences {
                source: source.clone(),
                missing_symbols,
                unexpected_symbols,
            });
        }

        Ok(())
    }
}
