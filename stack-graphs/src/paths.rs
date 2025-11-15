// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2021, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

//! Paths represent name bindings in a source language.
//!
//! In stack graphs, a **path** through the graph represents a potential name binding from a
//! reference to a definition. This module defines the path data structures and the rules for
//! valid paths.
//!
//! ## Path Validity
//!
//! A valid path must satisfy several constraints:
//!
//! 1. **Stack Discipline**: The symbol and scope stacks must be empty at both the start and end
//! 2. **Symbol Matching**: Pop operations must match their corresponding push operations
//! 3. **Edge Continuity**: Each edge's source must match the previous edge's sink
//! 4. **Scope Validity**: Jump operations must have scopes available on the scope stack
//!
//! ## How Paths Work
//!
//! Consider resolving a reference to a variable `x`:
//!
//! ```text
//! [reference to x]
//!     → push "x" onto symbol stack
//! [lexical scope edge]
//!     → search in current scope
//! [definition of x]
//!     → pop "x" from symbol stack (matches!)
//!
//! Symbol stack: [] → ["x"] → []  ✓ Valid binding!
//! ```
//!
//! ## Path Types
//!
//! This module supports two kinds of paths:
//!
//! - **Complete paths**: Both stacks empty at start and end (represent complete name bindings)
//! - **Partial paths**: Stacks may be non-empty (see [`partial`](../partial/index.html) module)
//!
//! ## Path Finding
//!
//! The path-finding algorithm:
//!
//! 1. Starts at a reference node (pushes a symbol onto the symbol stack)
//! 2. Follows edges, maintaining stack state
//! 3. Validates each step against stack discipline
//! 4. Succeeds when reaching a definition node with empty stacks
//!
//! Invalid paths are rejected as soon as a constraint is violated.

use std::collections::VecDeque;

/// Errors that can occur during the path resolution process.
///
/// These errors indicate violations of the path validity constraints. When the path-finding
/// algorithm encounters one of these errors, it abandons that path and tries other routes.
///
/// # Error Categories
///
/// ## Stack Errors
/// - [`EmptyScopeStack`](PathResolutionError::EmptyScopeStack): Tried to jump to a scope when none available
/// - [`EmptySymbolStack`](PathResolutionError::EmptySymbolStack): Tried to pop a symbol when stack is empty
/// - [`IncorrectPoppedSymbol`](PathResolutionError::IncorrectPoppedSymbol): Popped symbol doesn't match expected
///
/// ## Structure Errors
/// - [`IncorrectSourceNode`](PathResolutionError::IncorrectSourceNode): Edges don't connect properly
/// - [`IncorrectFile`](PathResolutionError::IncorrectFile): Path spans multiple files incorrectly
///
/// ## Partial Path Errors
/// - [`ScopeStackUnsatisfied`](PathResolutionError::ScopeStackUnsatisfied): Precondition not met
/// - [`SymbolStackUnsatisfied`](PathResolutionError::SymbolStackUnsatisfied): Precondition not met
/// - [`UnboundSymbolStackVariable`](PathResolutionError::UnboundSymbolStackVariable): Invalid variable reference
/// - [`UnboundScopeStackVariable`](PathResolutionError::UnboundScopeStackVariable): Invalid variable reference
///
/// ## Other Errors
/// - [`DisallowedCycle`](PathResolutionError::DisallowedCycle): Cycle detection triggered
/// - [`UnknownAttachedScope`](PathResolutionError::UnknownAttachedScope): Referenced scope doesn't exist
#[derive(Debug)]
pub enum PathResolutionError {
    /// The path contains a cycle, and the cycle is disallowed.
    ///
    /// Cycles can lead to infinite loops during path finding, so they're detected and prevented.
    /// Some cycles (like looking up a symbol in the same scope) are normal, but others indicate
    /// invalid paths.
    DisallowedCycle,

    /// The path contains a _jump to scope_ node, but there are no scopes on the scope stack.
    ///
    /// A jump-to node attempts to jump back to a scope that was previously pushed onto the
    /// scope stack. If the scope stack is empty, there's nowhere to jump to.
    EmptyScopeStack,

    /// The path contains a _pop symbol_ or _pop scoped symbol_ node, but the symbol stack is empty.
    ///
    /// Pop nodes expect a symbol to be on the symbol stack. If the stack is empty, this indicates
    /// an invalid path (likely missing the corresponding push operation).
    EmptySymbolStack,

    /// The partial path contains multiple references to a scope stack variable, and those
    /// references can't unify on a single scope stack.
    ///
    /// When stitching partial paths together, scope stack variables must be consistent.
    /// This error indicates that two parts of the path have incompatible expectations
    /// about what's on the scope stack.
    IncompatibleScopeStackVariables,

    /// The partial path contains multiple references to a symbol stack variable, and those
    /// references can't unify on a single symbol stack.
    ///
    /// Similar to `IncompatibleScopeStackVariables`, but for the symbol stack.
    IncompatibleSymbolStackVariables,

    /// The partial path contains edges from multiple files.
    ///
    /// A partial path should be contained within a single file. Cross-file paths are handled
    /// by stitching partial paths together through the root node.
    IncorrectFile,

    /// The path contains a _pop symbol_ or _pop scoped symbol_ node, but the symbol at the top of
    /// the symbol stack does not match.
    ///
    /// This is the most common error - it indicates that we're trying to pop symbol "x" but
    /// the symbol stack has "y" on top. This means this path doesn't lead to the definition
    /// we're looking for.
    IncorrectPoppedSymbol,

    /// The path contains an edge whose source node does not match the sink node of the preceding
    /// edge.
    ///
    /// Edges must form a connected path. This error indicates a bug in path construction.
    IncorrectSourceNode,

    /// The path contains a _pop scoped symbol_ node, but the symbol at the top of the symbol stack
    /// does not have an attached scope list.
    ///
    /// Pop-scoped-symbol nodes expect the symbol to have an attached scope (for member access
    /// like `obj.field`). If there's no attached scope, we can't resolve the member.
    MissingAttachedScopeList,

    /// The path's scope stack does not satisfy the partial path's scope stack precondition.
    ///
    /// When extending a path with a partial path, the current scope stack must match what
    /// the partial path expects.
    ScopeStackUnsatisfied,

    /// The path's symbol stack does not satisfy the partial path's symbol stack precondition.
    ///
    /// When extending a path with a partial path, the current symbol stack must match what
    /// the partial path expects.
    SymbolStackUnsatisfied,

    /// The partial path's postcondition references a symbol stack variable that isn't present in
    /// the precondition.
    ///
    /// Partial paths use variables to represent stack contents. A variable can only be used in
    /// the postcondition if it was introduced in the precondition.
    UnboundSymbolStackVariable,

    /// The partial path's postcondition references a scope stack variable that isn't present in
    /// the precondition.
    ///
    /// Similar to `UnboundSymbolStackVariable`, but for scope stack variables.
    UnboundScopeStackVariable,

    /// The path contains a _pop symbol_ node, but the symbol at the top of the symbol stack has an
    /// attached scope list that we weren't expecting.
    ///
    /// Pop-symbol nodes expect plain symbols (not scoped symbols). If there's an attached scope,
    /// this indicates a type mismatch.
    UnexpectedAttachedScopeList,

    /// A _push scoped symbol_ node refers to an exported scope node that doesn't exist.
    ///
    /// Push-scoped-symbol nodes reference an exported scope by ID. If that scope isn't in the
    /// graph, this error is returned.
    UnknownAttachedScope,
}

/// A collection that can be used to receive the results of the [`Path::extend`][] method.
///
/// Note: There's an [open issue][std-extend] to add these methods to std's `Extend` trait.  If
/// that gets merged, we can drop this trait and use the std one instead.
///
/// [std-extend]: https://github.com/rust-lang/rust/issues/72631
pub trait Extend<T> {
    /// Reserve space for `additional` elements in the collection.
    fn reserve(&mut self, additional: usize);
    /// Add a new element to the collection.
    fn push(&mut self, item: T);
}

impl<T> Extend<T> for Vec<T> {
    fn reserve(&mut self, additional: usize) {
        self.reserve(additional);
    }

    fn push(&mut self, item: T) {
        self.push(item);
    }
}

impl<T> Extend<T> for VecDeque<T> {
    fn reserve(&mut self, additional: usize) {
        self.reserve(additional);
    }

    fn push(&mut self, item: T) {
        self.push_back(item);
    }
}
