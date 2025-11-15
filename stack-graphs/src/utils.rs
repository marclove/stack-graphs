// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2021, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

//! Internal utility functions used throughout the stack-graphs crate.
//!
//! This module provides small helper functions that are used across multiple modules.
//! These utilities are crate-private and not exposed in the public API.

/// Compares two `Option` values using a custom equality function.
///
/// This generalizes `Option::==` to allow comparing `Option<A>` with `Option<B>` using
/// a custom equality function that compares `A` and `B`.
///
/// # Rules
///
/// - `Some(a) == Some(b)` if and only if `eq(a, b)` returns `true`
/// - `None == None` always
/// - `Some(_) != None` always
/// - `None != Some(_)` always
///
/// # Example
///
/// ```no_run
/// # use stack_graphs::utils::equals_option;
/// // Compare Option<&str> with Option<String>
/// let a: Option<&str> = Some("hello");
/// let b: Option<String> = Some("hello".to_string());
///
/// assert!(equals_option(a, b, |a, b| a == b));
/// ```
///
/// # Parameters
///
/// - `a`: First optional value
/// - `b`: Second optional value
/// - `eq`: Function that compares the inner values
///
/// # Returns
///
/// `true` if both options are `None`, or both are `Some` and `eq` returns `true`.
pub(crate) fn equals_option<A, B, F>(a: Option<A>, b: Option<B>, mut eq: F) -> bool
where
    F: FnMut(A, B) -> bool,
{
    match a {
        Some(a) => match b {
            // Both are Some: use custom equality
            Some(b) => eq(a, b),
            // Some vs None: not equal
            None => false,
        },
        None => match b {
            // None vs Some: not equal
            Some(_) => false,
            // Both None: equal
            None => true,
        },
    }
}

/// Compares two `Option` values using a custom comparison function.
///
/// This extends `Option::cmp` to use a custom comparison function for the inner values.
///
/// # Ordering Rules
///
/// The ordering follows these rules:
/// - `Some(_)` is always **greater** than `None`
/// - `None` is always **less** than `Some(_)`
/// - `None` equals `None`
/// - For `Some(a)` and `Some(b)`, use the custom `cmp` function
///
/// # Example
///
/// ```no_run
/// # use stack_graphs::utils::cmp_option;
/// use std::cmp::Ordering;
///
/// let a = Some(5);
/// let b = Some(10);
/// let c: Option<i32> = None;
///
/// assert_eq!(cmp_option(a, b, |a, b| a.cmp(&b)), Ordering::Less);
/// assert_eq!(cmp_option(a, c, |a, b| a.cmp(&b)), Ordering::Greater);
/// assert_eq!(cmp_option(c, c, |a, b| a.cmp(&b)), Ordering::Equal);
/// ```
///
/// # Parameters
///
/// - `a`: First optional value
/// - `b`: Second optional value
/// - `cmp`: Function that compares the inner values
///
/// # Returns
///
/// The ordering relationship between `a` and `b`.
pub(crate) fn cmp_option<T, F>(a: Option<T>, b: Option<T>, mut cmp: F) -> std::cmp::Ordering
where
    F: FnMut(T, T) -> std::cmp::Ordering,
{
    use std::cmp::Ordering;
    match a {
        Some(a) => match b {
            // Both Some: use custom comparison
            Some(b) => cmp(a, b),
            // Some > None
            None => Ordering::Greater,
        },
        None => match b {
            // None < Some
            Some(_) => Ordering::Less,
            // None == None
            None => Ordering::Equal,
        },
    }
}
