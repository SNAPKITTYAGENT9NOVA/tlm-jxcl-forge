// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! Consolidates scattered policy decisions (TLS-required-in-production, minimum seed/key length) into one crate with a real Policy trait, replacing duplicated ad hoc checks in photo-cache-service and pq-crypto.
//!
//! # Policy trait
//!
//! The [`Policy`] trait defines the interface for crypto operation policy enforcement,
//! allowing callers to define and check constraints on cryptographic operations
//! (e.g., algorithm selection, key constraints, environment requirements).
//!
//! # Built-in policies
//!
//! - [`TlsRequiredInProduction`]: Enforces that TLS is required in production environments.
//! - [`MinimumSeedLength`]: Enforces a minimum seed length for key derivation.
#![forbid(unsafe_code)]

use std::fmt;

/// Represents an error from a policy check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyError {
    /// A required policy constraint was not met.
    ConstraintViolation(String),
    /// The policy check itself encountered an error.
    EvaluationError(String),
}

impl fmt::Display for PolicyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PolicyError::ConstraintViolation(msg) => {
                write!(f, "policy constraint violated: {}", msg)
            }
            PolicyError::EvaluationError(msg) => write!(f, "policy evaluation error: {}", msg),
        }
    }
}

impl std::error::Error for PolicyError {}

/// A trait for enforcing cryptographic operation policies.
///
/// Policies define constraints on cryptographic operations, such as
/// requiring TLS in production or enforcing minimum key lengths.
pub trait Policy: Send + Sync + fmt::Debug {
    /// Check whether this policy is satisfied given the provided context.
    ///
    /// Returns `Ok(())` if the policy is satisfied, or `Err(PolicyError)`
    /// if a constraint is violated.
    fn check(&self, context: &PolicyContext) -> Result<(), PolicyError>;

    /// A human-readable name for this policy.
    fn name(&self) -> &str;
}

/// Context information for a policy check.
#[derive(Debug, Clone)]
pub struct PolicyContext {
    /// Whether the environment is production.
    pub is_production: bool,
    /// The seed or key material length in bytes.
    pub key_length: usize,
    /// Whether TLS is enabled.
    pub tls_enabled: bool,
    /// Optional custom data for policy-specific checks.
    pub custom_data: Option<String>,
}

impl PolicyContext {
    /// Create a new policy context.
    pub fn new(is_production: bool, key_length: usize, tls_enabled: bool) -> Self {
        PolicyContext {
            is_production,
            key_length,
            tls_enabled,
            custom_data: None,
        }
    }

    /// Add custom data to the context.
    pub fn with_custom_data(mut self, data: String) -> Self {
        self.custom_data = Some(data);
        self
    }
}

/// Policy that enforces TLS requirement in production environments.
#[derive(Debug, Clone)]
pub struct TlsRequiredInProduction;

impl Policy for TlsRequiredInProduction {
    fn check(&self, context: &PolicyContext) -> Result<(), PolicyError> {
        if context.is_production && !context.tls_enabled {
            return Err(PolicyError::ConstraintViolation(
                "TLS is required in production environments".to_string(),
            ));
        }
        Ok(())
    }

    fn name(&self) -> &str {
        "TlsRequiredInProduction"
    }
}

/// Policy that enforces minimum seed/key length.
#[derive(Debug, Clone)]
pub struct MinimumSeedLength {
    /// Minimum required length in bytes.
    pub min_length: usize,
}

impl MinimumSeedLength {
    /// Create a new MinimumSeedLength policy with the specified minimum length.
    pub fn new(min_length: usize) -> Self {
        MinimumSeedLength { min_length }
    }

    /// Create a policy with the recommended minimum length (32 bytes).
    pub fn recommended() -> Self {
        MinimumSeedLength { min_length: 32 }
    }
}

impl Policy for MinimumSeedLength {
    fn check(&self, context: &PolicyContext) -> Result<(), PolicyError> {
        if context.key_length < self.min_length {
            return Err(PolicyError::ConstraintViolation(format!(
                "seed/key length {} is below minimum of {}",
                context.key_length, self.min_length
            )));
        }
        Ok(())
    }

    fn name(&self) -> &str {
        "MinimumSeedLength"
    }
}

/// A composite policy that checks multiple policies.
pub struct CompositePolicy {
    policies: Vec<Box<dyn Policy>>,
}

impl CompositePolicy {
    /// Create a new composite policy.
    pub fn new() -> Self {
        CompositePolicy {
            policies: Vec::new(),
        }
    }

    /// Add a policy to this composite.
    pub fn with_policy<P: Policy + 'static>(mut self, policy: P) -> Self {
        self.policies.push(Box::new(policy));
        self
    }
}

impl Default for CompositePolicy {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for CompositePolicy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CompositePolicy")
            .field("policy_count", &self.policies.len())
            .finish()
    }
}

impl Policy for CompositePolicy {
    fn check(&self, context: &PolicyContext) -> Result<(), PolicyError> {
        for policy in &self.policies {
            policy.check(context)?;
        }
        Ok(())
    }

    fn name(&self) -> &str {
        "CompositePolicy"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tls_required_in_production_passes_when_tls_enabled() {
        let policy = TlsRequiredInProduction;
        let context = PolicyContext::new(true, 64, true);
        assert!(policy.check(&context).is_ok());
    }

    #[test]
    fn test_tls_required_in_production_fails_without_tls_in_prod() {
        let policy = TlsRequiredInProduction;
        let context = PolicyContext::new(true, 64, false);
        assert!(policy.check(&context).is_err());
    }

    #[test]
    fn test_tls_required_in_production_allows_no_tls_in_dev() {
        let policy = TlsRequiredInProduction;
        let context = PolicyContext::new(false, 64, false);
        assert!(policy.check(&context).is_ok());
    }

    #[test]
    fn test_minimum_seed_length_passes_when_sufficient() {
        let policy = MinimumSeedLength::new(32);
        let context = PolicyContext::new(false, 64, false);
        assert!(policy.check(&context).is_ok());
    }

    #[test]
    fn test_minimum_seed_length_fails_when_insufficient() {
        let policy = MinimumSeedLength::new(32);
        let context = PolicyContext::new(false, 16, false);
        assert!(policy.check(&context).is_err());
    }

    #[test]
    fn test_minimum_seed_length_exact_boundary() {
        let policy = MinimumSeedLength::new(32);
        let context = PolicyContext::new(false, 32, false);
        assert!(policy.check(&context).is_ok());
    }

    #[test]
    fn test_recommended_minimum_seed_length() {
        let policy = MinimumSeedLength::recommended();
        let context = PolicyContext::new(false, 32, false);
        assert!(policy.check(&context).is_ok());

        let context_too_short = PolicyContext::new(false, 31, false);
        assert!(policy.check(&context_too_short).is_err());
    }

    #[test]
    fn test_composite_policy_all_pass() {
        let composite = CompositePolicy::new()
            .with_policy(TlsRequiredInProduction)
            .with_policy(MinimumSeedLength::new(32));

        let context = PolicyContext::new(true, 64, true);
        assert!(composite.check(&context).is_ok());
    }

    #[test]
    fn test_composite_policy_one_fails() {
        let composite = CompositePolicy::new()
            .with_policy(TlsRequiredInProduction)
            .with_policy(MinimumSeedLength::new(32));

        let context = PolicyContext::new(true, 64, false);
        assert!(composite.check(&context).is_err());
    }

    #[test]
    fn test_policy_context_with_custom_data() {
        let context =
            PolicyContext::new(true, 64, true).with_custom_data("custom info".to_string());
        assert_eq!(context.custom_data, Some("custom info".to_string()));
    }

    #[test]
    fn test_policy_names() {
        let tls_policy = TlsRequiredInProduction;
        assert_eq!(tls_policy.name(), "TlsRequiredInProduction");

        let seed_policy = MinimumSeedLength::new(32);
        assert_eq!(seed_policy.name(), "MinimumSeedLength");
    }

    #[test]
    fn test_policy_error_display() {
        let err = PolicyError::ConstraintViolation("test violation".to_string());
        assert!(err.to_string().contains("test violation"));

        let err = PolicyError::EvaluationError("test error".to_string());
        assert!(err.to_string().contains("test error"));
    }
}
