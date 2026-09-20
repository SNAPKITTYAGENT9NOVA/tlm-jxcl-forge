// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! A structured audit-event schema and logging system for JXCL machine operations.
//!
//! This module provides comprehensive audit logging capabilities distinct from
//! `jxcl-logging`'s generic initialization. It records significant events in the
//! machine's operation lifecycle, including security-relevant actions, state
//! transitions, and error conditions.
//!
//! # Overview
//!
//! The audit system consists of three main components:
//!
//! - [`AuditLevel`]: Enumeration of severity levels (Debug, Info, Warning, Error)
//! - [`AuditEvent`]: A timestamped event with level, message, and contextual information
//! - [`AuditLog`]: A collection of events with methods to log, retrieve, and clear them
//!
//! # Example
//!
//! ```
//! use jxcl_audit::{AuditLog, AuditLevel};
//!
//! let mut audit = AuditLog::new();
//! audit.log(AuditLevel::Info, "Machine initialized", "startup");
//! audit.log(AuditLevel::Warning, "Unusual instruction sequence", "execution");
//!
//! assert_eq!(audit.events().len(), 2);
//! ```
//!
//! # Features
//!
//! - Timestamped event recording with microsecond precision
//! - Four severity levels for event categorization
//! - Contextual information for each event (e.g., "startup", "execution")
//! - Efficient event retrieval and manipulation

#![forbid(unsafe_code)]

use std::time::{SystemTime, UNIX_EPOCH};

/// Severity levels for audit events.
///
/// Audit events are categorized into four severity levels to allow
/// operators to filter and prioritize audit logs based on urgency
/// and importance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AuditLevel {
    /// Debug-level events for development and troubleshooting.
    Debug,

    /// Informational events marking normal operation milestones.
    Info,

    /// Warning-level events indicating unusual but recoverable conditions.
    Warning,

    /// Error-level events indicating failure or security-relevant issues.
    Error,
}

/// A single audit event with timestamp, level, message, and context.
///
/// Each event is timestamped with microseconds since the Unix epoch and
/// includes contextual information indicating where in the machine's
/// lifecycle the event occurred.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditEvent {
    /// Microseconds since Unix epoch (January 1, 1970, 00:00:00 UTC).
    pub timestamp: u64,

    /// Severity level of the event.
    pub level: AuditLevel,

    /// Human-readable description of the event.
    pub message: String,

    /// Contextual label indicating where the event originated
    /// (e.g., "startup", "execution", "shutdown").
    pub context: String,
}

/// An audit log storing a sequence of timestamped events.
///
/// The audit log maintains an ordered collection of events that occurred
/// during the machine's operation. It provides methods to log new events,
/// retrieve existing events, and clear the log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditLog {
    events: Vec<AuditEvent>,
}

impl AuditLog {
    /// Creates a new, empty audit log.
    ///
    /// # Example
    ///
    /// ```
    /// use jxcl_audit::AuditLog;
    ///
    /// let audit = AuditLog::new();
    /// assert_eq!(audit.events().len(), 0);
    /// ```
    pub fn new() -> Self {
        AuditLog { events: Vec::new() }
    }

    /// Records a new audit event with the specified level, message, and context.
    ///
    /// The event is automatically timestamped with the current system time in
    /// microseconds since the Unix epoch. Events are appended to the audit log
    /// in the order they are recorded.
    ///
    /// # Arguments
    ///
    /// * `level` - The severity level of the event
    /// * `message` - A human-readable description of the event
    /// * `context` - Contextual information about where the event occurred
    ///
    /// # Example
    ///
    /// ```
    /// use jxcl_audit::{AuditLog, AuditLevel};
    ///
    /// let mut audit = AuditLog::new();
    /// audit.log(AuditLevel::Info, "System started", "startup");
    /// audit.log(AuditLevel::Warning, "Low memory", "execution");
    /// audit.log(AuditLevel::Error, "Failed to load instruction", "loading");
    ///
    /// assert_eq!(audit.events().len(), 3);
    /// assert_eq!(audit.events()[0].level, AuditLevel::Info);
    /// assert_eq!(audit.events()[1].level, AuditLevel::Warning);
    /// assert_eq!(audit.events()[2].level, AuditLevel::Error);
    /// ```
    pub fn log(&mut self, level: AuditLevel, message: &str, context: &str) {
        let timestamp = Self::current_timestamp();
        let event = AuditEvent {
            timestamp,
            level,
            message: message.to_string(),
            context: context.to_string(),
        };
        self.events.push(event);
    }

    /// Returns a reference to all events in the audit log.
    ///
    /// # Example
    ///
    /// ```
    /// use jxcl_audit::{AuditLog, AuditLevel};
    ///
    /// let mut audit = AuditLog::new();
    /// audit.log(AuditLevel::Info, "Event 1", "context");
    /// audit.log(AuditLevel::Debug, "Event 2", "context");
    ///
    /// let events = audit.events();
    /// assert_eq!(events.len(), 2);
    /// assert_eq!(events[0].message, "Event 1");
    /// assert_eq!(events[1].message, "Event 2");
    /// ```
    pub fn events(&self) -> &[AuditEvent] {
        &self.events
    }

    /// Clears all events from the audit log.
    ///
    /// # Example
    ///
    /// ```
    /// use jxcl_audit::{AuditLog, AuditLevel};
    ///
    /// let mut audit = AuditLog::new();
    /// audit.log(AuditLevel::Info, "Event", "context");
    /// assert_eq!(audit.events().len(), 1);
    ///
    /// audit.clear();
    /// assert_eq!(audit.events().len(), 0);
    /// ```
    pub fn clear(&mut self) {
        self.events.clear();
    }

    /// Gets the current system time in microseconds since the Unix epoch.
    fn current_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_micros() as u64)
            .unwrap_or(0)
    }
}

impl Default for AuditLog {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_new_audit_log() {
        let audit = AuditLog::new();
        assert_eq!(audit.events().len(), 0);
    }

    #[test]
    fn log_debug_level_event() {
        let mut audit = AuditLog::new();
        audit.log(AuditLevel::Debug, "Debug message", "test_context");

        assert_eq!(audit.events().len(), 1);
        assert_eq!(audit.events()[0].level, AuditLevel::Debug);
        assert_eq!(audit.events()[0].message, "Debug message");
        assert_eq!(audit.events()[0].context, "test_context");
    }

    #[test]
    fn log_info_level_event() {
        let mut audit = AuditLog::new();
        audit.log(AuditLevel::Info, "Info message", "startup");

        assert_eq!(audit.events().len(), 1);
        assert_eq!(audit.events()[0].level, AuditLevel::Info);
        assert_eq!(audit.events()[0].message, "Info message");
        assert_eq!(audit.events()[0].context, "startup");
    }

    #[test]
    fn log_warning_level_event() {
        let mut audit = AuditLog::new();
        audit.log(AuditLevel::Warning, "Warning message", "execution");

        assert_eq!(audit.events().len(), 1);
        assert_eq!(audit.events()[0].level, AuditLevel::Warning);
        assert_eq!(audit.events()[0].message, "Warning message");
        assert_eq!(audit.events()[0].context, "execution");
    }

    #[test]
    fn log_error_level_event() {
        let mut audit = AuditLog::new();
        audit.log(AuditLevel::Error, "Error message", "failure");

        assert_eq!(audit.events().len(), 1);
        assert_eq!(audit.events()[0].level, AuditLevel::Error);
        assert_eq!(audit.events()[0].message, "Error message");
        assert_eq!(audit.events()[0].context, "failure");
    }

    #[test]
    fn log_multiple_events() {
        let mut audit = AuditLog::new();
        audit.log(AuditLevel::Info, "First event", "startup");
        audit.log(AuditLevel::Debug, "Second event", "execution");
        audit.log(AuditLevel::Warning, "Third event", "execution");
        audit.log(AuditLevel::Error, "Fourth event", "failure");

        assert_eq!(audit.events().len(), 4);
        assert_eq!(audit.events()[0].level, AuditLevel::Info);
        assert_eq!(audit.events()[1].level, AuditLevel::Debug);
        assert_eq!(audit.events()[2].level, AuditLevel::Warning);
        assert_eq!(audit.events()[3].level, AuditLevel::Error);
    }

    #[test]
    fn events_are_timestamped() {
        let mut audit = AuditLog::new();
        let before = AuditLog::current_timestamp();
        audit.log(AuditLevel::Info, "Timestamped event", "test");
        let after = AuditLog::current_timestamp();

        let event_timestamp = audit.events()[0].timestamp;
        assert!(event_timestamp >= before);
        assert!(event_timestamp <= after);
    }

    #[test]
    fn events_maintain_chronological_order() {
        let mut audit = AuditLog::new();
        audit.log(AuditLevel::Info, "First", "context");
        audit.log(AuditLevel::Info, "Second", "context");
        audit.log(AuditLevel::Info, "Third", "context");

        let events = audit.events();
        assert_eq!(events[0].message, "First");
        assert_eq!(events[1].message, "Second");
        assert_eq!(events[2].message, "Third");

        // Timestamps should be in non-decreasing order
        assert!(events[0].timestamp <= events[1].timestamp);
        assert!(events[1].timestamp <= events[2].timestamp);
    }

    #[test]
    fn clear_removes_all_events() {
        let mut audit = AuditLog::new();
        audit.log(AuditLevel::Info, "Event 1", "context");
        audit.log(AuditLevel::Info, "Event 2", "context");
        audit.log(AuditLevel::Info, "Event 3", "context");

        assert_eq!(audit.events().len(), 3);

        audit.clear();
        assert_eq!(audit.events().len(), 0);
    }

    #[test]
    fn can_log_after_clear() {
        let mut audit = AuditLog::new();
        audit.log(AuditLevel::Info, "First event", "context");
        audit.clear();
        audit.log(AuditLevel::Info, "Second event", "context");

        assert_eq!(audit.events().len(), 1);
        assert_eq!(audit.events()[0].message, "Second event");
    }

    #[test]
    fn audit_event_clone() {
        let event = AuditEvent {
            timestamp: 1000000,
            level: AuditLevel::Info,
            message: "Test event".to_string(),
            context: "test".to_string(),
        };

        let cloned = event.clone();
        assert_eq!(event, cloned);
    }

    #[test]
    fn audit_level_ordering() {
        assert!(AuditLevel::Debug < AuditLevel::Info);
        assert!(AuditLevel::Info < AuditLevel::Warning);
        assert!(AuditLevel::Warning < AuditLevel::Error);
    }

    #[test]
    fn audit_log_default() {
        let audit = AuditLog::default();
        assert_eq!(audit.events().len(), 0);
    }

    #[test]
    fn audit_log_with_empty_message_and_context() {
        let mut audit = AuditLog::new();
        audit.log(AuditLevel::Info, "", "");

        assert_eq!(audit.events().len(), 1);
        assert_eq!(audit.events()[0].message, "");
        assert_eq!(audit.events()[0].context, "");
    }

    #[test]
    fn audit_log_with_long_strings() {
        let mut audit = AuditLog::new();
        let long_message = "A".repeat(1000);
        let long_context = "B".repeat(1000);

        audit.log(AuditLevel::Warning, &long_message, &long_context);

        assert_eq!(audit.events().len(), 1);
        assert_eq!(audit.events()[0].message.len(), 1000);
        assert_eq!(audit.events()[0].context.len(), 1000);
    }
}
