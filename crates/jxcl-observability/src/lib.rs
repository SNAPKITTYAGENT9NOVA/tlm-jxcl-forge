#![forbid(unsafe_code)]
// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! Metrics and instrumentation for JXCL machines.
//!
//! This module provides a comprehensive metrics collection system for observing JXCL machine
//! behavior and performance. It supports counters (monotonically increasing u64 values),
//! gauges (floating-point measurements), and histogram data for detailed performance analysis.
//!
//! # Overview
//!
//! The observability system centers on the [`Metrics`] struct, which maintains collections of
//! named counters and gauges. Each metric can be updated, queried, and snapshoted for analysis.
//!
//! # Examples
//!
//! ```
//! use jxcl_observability::Metrics;
//!
//! let mut metrics = Metrics::new();
//! metrics.increment_counter("requests", 1).expect("counter increment");
//! metrics.set_gauge("memory_usage", 512.5).expect("gauge set");
//!
//! assert_eq!(metrics.get_counter("requests"), Some(1));
//! assert_eq!(metrics.get_gauge("memory_usage"), Some(512.5));
//! ```

use std::collections::HashMap;

/// Represents a single metric value with multiple variant types.
///
/// Metrics can be counters (u64), gauges (f64), or histograms (vector of u64 samples).
#[derive(Debug, Clone, PartialEq)]
pub enum MetricValue {
    /// A counter metric: monotonically increasing unsigned 64-bit integer.
    Counter(u64),
    /// A gauge metric: a floating-point measurement that can go up or down.
    Gauge(f64),
    /// A histogram metric: a collection of u64 samples for distribution analysis.
    Histogram(Vec<u64>),
}

/// Manages metrics collection for JXCL machines.
///
/// The `Metrics` struct provides methods to track and retrieve both counter and gauge metrics.
/// Counters are typically used for tracking cumulative events (like request counts), while gauges
/// measure instantaneous values (like memory usage or CPU utilization).
#[derive(Debug, Clone)]
pub struct Metrics {
    counters: HashMap<String, u64>,
    gauges: HashMap<String, f64>,
}

impl Metrics {
    /// Creates a new, empty `Metrics` instance.
    ///
    /// # Examples
    ///
    /// ```
    /// use jxcl_observability::Metrics;
    ///
    /// let metrics = Metrics::new();
    /// assert_eq!(metrics.get_counter("any_name"), None);
    /// ```
    pub fn new() -> Self {
        Metrics {
            counters: HashMap::new(),
            gauges: HashMap::new(),
        }
    }

    /// Increments a counter by the specified value.
    ///
    /// If the counter does not exist, it is initialized with the provided value.
    /// If the counter already exists, the value is added to it.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the counter metric
    /// * `value` - The amount to increment by
    ///
    /// # Errors
    ///
    /// Returns an error if the metric name is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use jxcl_observability::Metrics;
    ///
    /// let mut metrics = Metrics::new();
    /// metrics.increment_counter("requests", 5).expect("counter increment");
    /// metrics.increment_counter("requests", 3).expect("counter increment");
    /// assert_eq!(metrics.get_counter("requests"), Some(8));
    /// ```
    pub fn increment_counter(&mut self, name: &str, value: u64) -> Result<(), String> {
        if name.is_empty() {
            return Err("metric name cannot be empty".to_string());
        }

        self.counters
            .entry(name.to_string())
            .and_modify(|v| *v = v.saturating_add(value))
            .or_insert(value);

        Ok(())
    }

    /// Sets a gauge to a specific floating-point value.
    ///
    /// If the gauge does not exist, it is created with the provided value.
    /// If the gauge already exists, its value is replaced.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the gauge metric
    /// * `value` - The value to set
    ///
    /// # Errors
    ///
    /// Returns an error if the metric name is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use jxcl_observability::Metrics;
    ///
    /// let mut metrics = Metrics::new();
    /// metrics.set_gauge("temperature", 98.6).expect("gauge set");
    /// assert_eq!(metrics.get_gauge("temperature"), Some(98.6));
    /// ```
    pub fn set_gauge(&mut self, name: &str, value: f64) -> Result<(), String> {
        if name.is_empty() {
            return Err("metric name cannot be empty".to_string());
        }

        self.gauges.insert(name.to_string(), value);

        Ok(())
    }

    /// Retrieves a counter value by name.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the counter metric
    ///
    /// # Returns
    ///
    /// `Some(value)` if the counter exists, `None` otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use jxcl_observability::Metrics;
    ///
    /// let mut metrics = Metrics::new();
    /// metrics.increment_counter("events", 42).expect("counter increment");
    /// assert_eq!(metrics.get_counter("events"), Some(42));
    /// assert_eq!(metrics.get_counter("nonexistent"), None);
    /// ```
    pub fn get_counter(&self, name: &str) -> Option<u64> {
        self.counters.get(name).copied()
    }

    /// Retrieves a gauge value by name.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the gauge metric
    ///
    /// # Returns
    ///
    /// `Some(value)` if the gauge exists, `None` otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use jxcl_observability::Metrics;
    ///
    /// let mut metrics = Metrics::new();
    /// metrics.set_gauge("load", 2.5).expect("gauge set");
    /// assert_eq!(metrics.get_gauge("load"), Some(2.5));
    /// assert_eq!(metrics.get_gauge("missing"), None);
    /// ```
    pub fn get_gauge(&self, name: &str) -> Option<f64> {
        self.gauges.get(name).copied()
    }

    /// Returns a snapshot of all metrics as a HashMap of metric names to values.
    ///
    /// This method captures all current counter and gauge values, converting them into
    /// a unified representation via the `MetricValue` enum. Counters are returned as
    /// `MetricValue::Counter` and gauges as `MetricValue::Gauge`.
    ///
    /// # Returns
    ///
    /// A HashMap mapping metric names to their current `MetricValue`.
    ///
    /// # Examples
    ///
    /// ```
    /// use jxcl_observability::{Metrics, MetricValue};
    ///
    /// let mut metrics = Metrics::new();
    /// metrics.increment_counter("requests", 100).expect("counter increment");
    /// metrics.set_gauge("uptime", 3600.5).expect("gauge set");
    ///
    /// let snapshot = metrics.snapshot();
    /// assert_eq!(snapshot.get("requests"), Some(&MetricValue::Counter(100)));
    /// assert_eq!(snapshot.get("uptime"), Some(&MetricValue::Gauge(3600.5)));
    /// ```
    pub fn snapshot(&self) -> HashMap<String, MetricValue> {
        let mut result = HashMap::new();

        for (name, value) in &self.counters {
            result.insert(name.clone(), MetricValue::Counter(*value));
        }

        for (name, value) in &self.gauges {
            result.insert(name.clone(), MetricValue::Gauge(*value));
        }

        result
    }
}

impl Default for Metrics {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_new() {
        let metrics = Metrics::new();
        assert_eq!(metrics.get_counter("any"), None);
        assert_eq!(metrics.get_gauge("any"), None);
    }

    #[test]
    fn test_increment_counter_new() {
        let mut metrics = Metrics::new();
        metrics
            .increment_counter("requests", 5)
            .expect("should succeed");
        assert_eq!(metrics.get_counter("requests"), Some(5));
    }

    #[test]
    fn test_increment_counter_existing() {
        let mut metrics = Metrics::new();
        metrics
            .increment_counter("requests", 5)
            .expect("should succeed");
        metrics
            .increment_counter("requests", 3)
            .expect("should succeed");
        assert_eq!(metrics.get_counter("requests"), Some(8));
    }

    #[test]
    fn test_increment_counter_multiple_times() {
        let mut metrics = Metrics::new();
        for i in 1..=10 {
            metrics
                .increment_counter("counter", i)
                .expect("should succeed");
        }
        assert_eq!(metrics.get_counter("counter"), Some(55)); // sum of 1..=10
    }

    #[test]
    fn test_increment_counter_empty_name() {
        let mut metrics = Metrics::new();
        let result = metrics.increment_counter("", 5);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "metric name cannot be empty");
    }

    #[test]
    fn test_set_gauge_new() {
        let mut metrics = Metrics::new();
        metrics
            .set_gauge("temperature", 98.6)
            .expect("should succeed");
        assert_eq!(metrics.get_gauge("temperature"), Some(98.6));
    }

    #[test]
    fn test_set_gauge_overwrite() {
        let mut metrics = Metrics::new();
        metrics
            .set_gauge("temperature", 98.6)
            .expect("should succeed");
        metrics
            .set_gauge("temperature", 100.2)
            .expect("should succeed");
        assert_eq!(metrics.get_gauge("temperature"), Some(100.2));
    }

    #[test]
    fn test_set_gauge_empty_name() {
        let mut metrics = Metrics::new();
        let result = metrics.set_gauge("", 50.0);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "metric name cannot be empty");
    }

    #[test]
    fn test_get_counter_nonexistent() {
        let metrics = Metrics::new();
        assert_eq!(metrics.get_counter("missing"), None);
    }

    #[test]
    fn test_get_gauge_nonexistent() {
        let metrics = Metrics::new();
        assert_eq!(metrics.get_gauge("missing"), None);
    }

    #[test]
    fn test_snapshot_empty() {
        let metrics = Metrics::new();
        let snapshot = metrics.snapshot();
        assert!(snapshot.is_empty());
    }

    #[test]
    fn test_snapshot_with_metrics() {
        let mut metrics = Metrics::new();
        metrics
            .increment_counter("requests", 100)
            .expect("should succeed");
        metrics.set_gauge("uptime", 3600.5).expect("should succeed");

        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.len(), 2);
        assert_eq!(snapshot.get("requests"), Some(&MetricValue::Counter(100)));
        assert_eq!(snapshot.get("uptime"), Some(&MetricValue::Gauge(3600.5)));
    }

    #[test]
    fn test_snapshot_multiple_counters_and_gauges() {
        let mut metrics = Metrics::new();
        metrics
            .increment_counter("requests", 50)
            .expect("should succeed");
        metrics
            .increment_counter("errors", 10)
            .expect("should succeed");
        metrics.set_gauge("memory", 512.5).expect("should succeed");
        metrics.set_gauge("cpu", 75.3).expect("should succeed");

        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.len(), 4);
        assert_eq!(snapshot.get("requests"), Some(&MetricValue::Counter(50)));
        assert_eq!(snapshot.get("errors"), Some(&MetricValue::Counter(10)));
        assert_eq!(snapshot.get("memory"), Some(&MetricValue::Gauge(512.5)));
        assert_eq!(snapshot.get("cpu"), Some(&MetricValue::Gauge(75.3)));
    }

    #[test]
    fn test_metric_value_equality() {
        let counter1 = MetricValue::Counter(42);
        let counter2 = MetricValue::Counter(42);
        let counter3 = MetricValue::Counter(43);

        assert_eq!(counter1, counter2);
        assert_ne!(counter1, counter3);
    }

    #[test]
    fn test_metric_value_debug() {
        let counter = MetricValue::Counter(42);
        let debug_str = format!("{:?}", counter);
        assert!(debug_str.contains("Counter"));
        assert!(debug_str.contains("42"));
    }

    #[test]
    fn test_metrics_default() {
        let metrics = Metrics::default();
        assert_eq!(metrics.get_counter("test"), None);
        assert_eq!(metrics.get_gauge("test"), None);
    }

    #[test]
    fn test_counter_saturation() {
        let mut metrics = Metrics::new();
        // Set counter to near max
        metrics.counters.insert("big".to_string(), u64::MAX - 5);
        // Try to add 10 - should saturate to u64::MAX
        metrics
            .increment_counter("big", 10)
            .expect("should succeed");
        assert_eq!(metrics.get_counter("big"), Some(u64::MAX));
    }

    #[test]
    fn test_different_metric_names() {
        let mut metrics = Metrics::new();
        metrics
            .increment_counter("counter1", 10)
            .expect("should succeed");
        metrics
            .increment_counter("counter2", 20)
            .expect("should succeed");
        metrics.set_gauge("gauge1", 5.5).expect("should succeed");
        metrics.set_gauge("gauge2", 9.9).expect("should succeed");

        assert_eq!(metrics.get_counter("counter1"), Some(10));
        assert_eq!(metrics.get_counter("counter2"), Some(20));
        assert_eq!(metrics.get_gauge("gauge1"), Some(5.5));
        assert_eq!(metrics.get_gauge("gauge2"), Some(9.9));
    }
}
