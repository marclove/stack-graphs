// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2023, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

//! Statistical utilities for tracking value frequencies.
//!
//! This module provides the [`FrequencyDistribution`][] type, which tracks how often different
//! values occur and provides statistical analysis methods like quantile computation.
//!
//! ## Use Cases
//!
//! Frequency distributions are useful for:
//! - Collecting performance metrics (e.g., how many paths of each length were found)
//! - Analyzing patterns in data (e.g., how many files have each number of definitions)
//! - Computing statistics like quantiles to understand data distribution
//! - Aggregating counts from multiple sources
//!
//! ## Example
//!
//! ```
//! use stack_graphs::stats::FrequencyDistribution;
//!
//! let mut dist = FrequencyDistribution::default();
//!
//! // Record some values
//! dist.record("apple");
//! dist.record("banana");
//! dist.record("apple");
//! dist.record("cherry");
//! dist.record("apple");
//!
//! assert_eq!(dist.count(), 5);     // Total values recorded
//! assert_eq!(dist.unique(), 3);    // Number of unique values
//! ```

use std::collections::HashMap;
use std::hash::Hash;

use itertools::Itertools;

/// Tracks the frequency distribution of values.
///
/// A frequency distribution maintains a count of how many times each distinct value
/// has been recorded. It provides methods to query the total count, number of unique
/// values, and compute quantiles for ordered types.
///
/// # Type Parameters
///
/// - `T`: The type of values to track. Must implement `Eq` and `Hash` to determine
///   uniqueness. For quantile computation, `T` must also implement `Ord`.
///
/// # Example
///
/// ```
/// use stack_graphs::stats::FrequencyDistribution;
///
/// let mut path_lengths = FrequencyDistribution::default();
///
/// // Record path lengths as we find them
/// path_lengths.record(3);
/// path_lengths.record(5);
/// path_lengths.record(3);
/// path_lengths.record(7);
/// path_lengths.record(3);
///
/// assert_eq!(path_lengths.count(), 5);    // 5 paths total
/// assert_eq!(path_lengths.unique(), 3);   // 3 distinct lengths
///
/// // Get quartiles (0%, 25%, 50%, 75%, 100%)
/// let quartiles = path_lengths.quantiles(4);
/// // quartiles[0] is the minimum, quartiles[2] is the median, quartiles[4] is the maximum
/// ```
#[derive(Clone, Debug, Default)]
pub struct FrequencyDistribution<T>
where
    T: Eq + Hash,
{
    /// Maps each unique value to the number of times it has been recorded.
    values: HashMap<T, usize>,
    /// The total number of values recorded (sum of all frequencies).
    total: usize,
}

impl<T: Eq + Hash> FrequencyDistribution<T> {
    /// Records an occurrence of a value.
    ///
    /// This increments the frequency count for the given value and updates the total count.
    ///
    /// # Example
    ///
    /// ```
    /// use stack_graphs::stats::FrequencyDistribution;
    ///
    /// let mut dist = FrequencyDistribution::default();
    /// dist.record("apple");
    /// dist.record("apple");
    /// dist.record("banana");
    ///
    /// assert_eq!(dist.count(), 3);    // 3 total values
    /// assert_eq!(dist.unique(), 2);   // 2 unique values
    /// ```
    pub fn record(&mut self, value: T) {
        *self.values.entry(value).or_default() += 1;
        self.total += 1;
    }

    /// Returns the total number of values recorded.
    ///
    /// This is the sum of all frequency counts, not the number of unique values.
    ///
    /// # Example
    ///
    /// ```
    /// use stack_graphs::stats::FrequencyDistribution;
    ///
    /// let mut dist = FrequencyDistribution::default();
    /// dist.record(1);
    /// dist.record(2);
    /// dist.record(1);
    ///
    /// assert_eq!(dist.count(), 3);    // 3 values recorded total
    /// assert_eq!(dist.unique(), 2);   // but only 2 unique values
    /// ```
    pub fn count(&self) -> usize {
        return self.total;
    }

    /// Returns the number of unique values recorded.
    ///
    /// # Example
    ///
    /// ```
    /// use stack_graphs::stats::FrequencyDistribution;
    ///
    /// let mut dist = FrequencyDistribution::default();
    /// dist.record("x");
    /// dist.record("y");
    /// dist.record("x");
    /// dist.record("z");
    ///
    /// assert_eq!(dist.unique(), 3);   // "x", "y", "z"
    /// ```
    pub fn unique(&self) -> usize {
        return self.values.len();
    }

    /// Computes the frequency distribution of frequencies.
    ///
    /// This returns a new frequency distribution where each value is a frequency count
    /// from the original distribution, and its frequency is how many values in the
    /// original distribution had that count.
    ///
    /// # Example
    ///
    /// ```
    /// use stack_graphs::stats::FrequencyDistribution;
    ///
    /// let mut dist = FrequencyDistribution::default();
    /// // Record values with different frequencies
    /// dist.record("a");  // appears 1 time
    /// dist.record("b");  // appears 2 times
    /// dist.record("b");
    /// dist.record("c");  // appears 2 times
    /// dist.record("c");
    /// dist.record("d");  // appears 3 times
    /// dist.record("d");
    /// dist.record("d");
    ///
    /// let freq_of_freq = dist.frequencies();
    /// // One value appears 1 time
    /// // Two values appear 2 times
    /// // One value appears 3 times
    /// assert_eq!(freq_of_freq.count(), 4);   // 4 unique values in original
    /// assert_eq!(freq_of_freq.unique(), 3);  // 3 distinct frequency counts (1, 2, 3)
    /// ```
    ///
    /// # Use Case
    ///
    /// This is useful for understanding the distribution of frequencies themselves.
    /// For example, in a word frequency analysis, `frequencies()` tells you how many
    /// words appear once, twice, three times, etc.
    pub fn frequencies(&self) -> FrequencyDistribution<usize> {
        let mut fs = FrequencyDistribution::default();
        for count in self.values.values() {
            fs.record(*count);
        }
        fs
    }
}

impl<T: Eq + Hash + Ord> FrequencyDistribution<T> {
    /// Computes quantiles for the distribution.
    ///
    /// Quantiles divide the recorded values into `q` equal-sized groups. This method
    /// returns `q + 1` values that are the boundaries between these groups.
    ///
    /// # Parameters
    ///
    /// - `q`: The number of groups to divide the data into. For example:
    ///   - `q = 2`: Returns minimum, median, maximum (3 values)
    ///   - `q = 4`: Returns quartiles (5 values: 0%, 25%, 50%, 75%, 100%)
    ///   - `q = 10`: Returns deciles (11 values)
    ///   - `q = 100`: Returns percentiles (101 values)
    ///
    /// # Returns
    ///
    /// A vector of `q + 1` references to values representing the quantile boundaries.
    /// Returns an empty vector if `q` is 0 or no values have been recorded.
    ///
    /// # Example
    ///
    /// ```
    /// use stack_graphs::stats::FrequencyDistribution;
    ///
    /// let mut dist = FrequencyDistribution::default();
    /// for i in 1..=100 {
    ///     dist.record(i);
    /// }
    ///
    /// // Get quartiles
    /// let quartiles = dist.quantiles(4);
    /// assert_eq!(quartiles.len(), 5);
    /// assert_eq!(*quartiles[0], 1);    // minimum (0th percentile)
    /// assert_eq!(*quartiles[2], 50);   // median (50th percentile)
    /// assert_eq!(*quartiles[4], 100);  // maximum (100th percentile)
    /// ```
    ///
    /// # Algorithm
    ///
    /// The algorithm:
    /// 1. Sorts all unique values in ascending order
    /// 2. For each quantile k/q, finds the value where the cumulative count reaches
    ///    `(total * k) / q` values
    /// 3. Returns references to the quantile boundary values
    ///
    /// # Note
    ///
    /// This method requires `T: Ord` to sort values. It works correctly with frequency
    /// distributions where values may have different counts.
    pub fn quantiles(&self, q: usize) -> Vec<&T> {
        // Handle edge cases: no quantiles requested or no data
        if q == 0 || self.total == 0 {
            return vec![];
        }

        // Sort all unique values
        let mut it = self.values.iter().sorted_by_key(|e| e.0);
        let mut total_count = 0;
        let mut last_value;
        let mut result = Vec::new();

        // Get the first value (minimum)
        if let Some((value, count)) = it.next() {
            total_count += count;
            last_value = value;
        } else {
            return vec![];
        }
        result.push(last_value);

        // Compute each quantile boundary
        for k in 1..=q {
            // Calculate how many values should be at or before this quantile
            let limit = ((self.total as f64 * k as f64) / q as f64).round() as usize;

            // Advance through values until we reach the limit
            while total_count < limit {
                if let Some((value, count)) = it.next() {
                    total_count += count;
                    last_value = value;
                } else {
                    // No more values; use the last one
                    break;
                }
            }
            result.push(last_value);
        }

        result
    }
}

/// Merges another frequency distribution into this one (consuming the other).
///
/// This adds all frequency counts from `rhs` into `self`, combining values that
/// appear in both distributions.
///
/// # Example
///
/// ```
/// use stack_graphs::stats::FrequencyDistribution;
///
/// let mut dist1 = FrequencyDistribution::default();
/// dist1.record("a");
/// dist1.record("b");
///
/// let mut dist2 = FrequencyDistribution::default();
/// dist2.record("b");
/// dist2.record("c");
///
/// dist1 += dist2;  // Consumes dist2
/// assert_eq!(dist1.count(), 4);     // 4 total values
/// assert_eq!(dist1.unique(), 3);    // "a", "b", "c"
/// ```
///
/// # Use Case
///
/// This is useful for aggregating statistics from multiple sources, such as combining
/// metrics from different files or threads.
impl<T> std::ops::AddAssign<Self> for FrequencyDistribution<T>
where
    T: Eq + Hash,
{
    fn add_assign(&mut self, rhs: Self) {
        // Merge all counts from rhs into self
        for (value, count) in rhs.values {
            *self.values.entry(value).or_default() += count;
        }
        self.total += rhs.total;
    }
}

/// Merges another frequency distribution into this one (borrowing the other).
///
/// This adds all frequency counts from `rhs` into `self`, without consuming `rhs`.
/// Values must implement `Clone` since they need to be copied from `rhs`.
///
/// # Example
///
/// ```
/// use stack_graphs::stats::FrequencyDistribution;
///
/// let mut dist1 = FrequencyDistribution::default();
/// dist1.record("a");
/// dist1.record("b");
///
/// let mut dist2 = FrequencyDistribution::default();
/// dist2.record("b");
/// dist2.record("c");
///
/// dist1 += &dist2;  // Borrows dist2
/// assert_eq!(dist1.count(), 4);     // 4 total values
/// assert_eq!(dist1.unique(), 3);    // "a", "b", "c"
/// // dist2 is still usable here
/// assert_eq!(dist2.count(), 2);
/// ```
///
/// # Use Case
///
/// Use this when you need to keep the source distribution available after merging,
/// such as when accumulating statistics while preserving per-file metrics.
impl<T> std::ops::AddAssign<&Self> for FrequencyDistribution<T>
where
    T: Eq + Hash + Clone,
{
    fn add_assign(&mut self, rhs: &Self) {
        // Merge all counts from rhs into self, cloning values as needed
        for (value, count) in &rhs.values {
            *self.values.entry(value.clone()).or_default() += count;
        }
        self.total += rhs.total;
    }
}
