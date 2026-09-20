// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! ISA/binary-format version negotiation so old binaries fail closed against an incompatible newer decoder rather than silently misdecoding.
//!
//! Owns: `IsaVersion`, its embedding in the binary container header, and
//! the compatibility check (`is_compatible_with`).
//!
//! The ISA version is a semantic versioning scheme where compatibility is
//! determined by comparing major and minor versions: a binary compiled
//! against ISA version X.Y can only run on a decoder supporting >= X.Y
//! (i.e., same major version and minor version >= binary's minor version).
#![forbid(unsafe_code)]

use jxcl_constants::BINARY_VERSION;

/// An ISA semantic version (major.minor) embedded in binary containers
/// to prevent silently misdecoding old binaries when the ISA evolves.
///
/// Following semantic versioning: major changes are breaking, minor
/// changes are backward-compatible. A decoder supporting version X.Y can
/// safely decode binaries built for X.0 through X.Y (same major, any
/// minor <= decoder's minor).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IsaVersion {
    /// Major version number (breaking changes).
    pub major: u16,
    /// Minor version number (backward-compatible additions).
    pub minor: u16,
}

impl IsaVersion {
    /// Create a new ISA version from major and minor components.
    pub const fn new(major: u16, minor: u16) -> Self {
        IsaVersion { major, minor }
    }

    /// The current ISA version embedded in all newly created binaries.
    /// Sourced from `jxcl-constants::BINARY_VERSION` which is the
    /// single authoritative version number.
    pub const fn current() -> Self {
        // BINARY_VERSION is a u16; we treat it as major=1, minor=VERSION
        // for forward compatibility (allows future major versions).
        IsaVersion {
            major: 1,
            minor: BINARY_VERSION,
        }
    }

    /// Check if this decoder's ISA version is compatible with a binary
    /// compiled for `binary_version`.
    ///
    /// A decoder supporting version X.Y is compatible with binaries
    /// built for X.Z where Z <= Y (same major, any older minor).
    /// This is true iff:
    /// - Major versions match (breaking changes are not forward-compatible)
    /// - Decoder's minor >= binary's minor (newer decoders understand old formats)
    pub const fn is_compatible_with(self, binary_version: IsaVersion) -> bool {
        self.major == binary_version.major && self.minor >= binary_version.minor
    }

    /// Check if this is a newer or equal version than another.
    pub const fn is_newer_than_or_equal(self, other: IsaVersion) -> bool {
        self.major > other.major || (self.major == other.major && self.minor >= other.minor)
    }

    /// Check if this is older than another version.
    pub const fn is_older_than(self, other: IsaVersion) -> bool {
        self.major < other.major || (self.major == other.major && self.minor < other.minor)
    }
}

impl std::fmt::Display for IsaVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)
    }
}

impl std::str::FromStr for IsaVersion {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 2 {
            return Err(format!(
                "invalid version format: expected 'major.minor', got '{}'",
                s
            ));
        }

        let major = parts[0]
            .parse::<u16>()
            .map_err(|_| format!("invalid major version: {}", parts[0]))?;
        let minor = parts[1]
            .parse::<u16>()
            .map_err(|_| format!("invalid minor version: {}", parts[1]))?;

        Ok(IsaVersion { major, minor })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_version_is_well_formed() {
        let current = IsaVersion::current();
        assert_eq!(current.major, 1);
        assert_eq!(current.minor, BINARY_VERSION);
    }

    #[test]
    fn compatible_with_same_version() {
        let v1_0 = IsaVersion::new(1, 0);
        assert!(v1_0.is_compatible_with(v1_0));
    }

    #[test]
    fn compatible_with_older_minor() {
        let decoder = IsaVersion::new(1, 2);
        let binary = IsaVersion::new(1, 0);
        assert!(decoder.is_compatible_with(binary));
    }

    #[test]
    fn compatible_with_equal_minor() {
        let decoder = IsaVersion::new(1, 2);
        let binary = IsaVersion::new(1, 2);
        assert!(decoder.is_compatible_with(binary));
    }

    #[test]
    fn not_compatible_with_newer_minor() {
        let decoder = IsaVersion::new(1, 0);
        let binary = IsaVersion::new(1, 2);
        assert!(!decoder.is_compatible_with(binary));
    }

    #[test]
    fn not_compatible_with_different_major() {
        let decoder = IsaVersion::new(1, 5);
        let binary = IsaVersion::new(2, 0);
        assert!(!decoder.is_compatible_with(binary));
    }

    #[test]
    fn version_comparison_operations() {
        let v1_0 = IsaVersion::new(1, 0);
        let v1_2 = IsaVersion::new(1, 2);
        let v2_0 = IsaVersion::new(2, 0);

        assert!(v1_2.is_newer_than_or_equal(v1_0));
        assert!(v1_2.is_newer_than_or_equal(v1_2));
        assert!(!v1_0.is_newer_than_or_equal(v1_2));

        assert!(v1_0.is_older_than(v1_2));
        assert!(!v1_2.is_older_than(v1_0));
        assert!(v1_2.is_older_than(v2_0));
    }

    #[test]
    fn version_display_format() {
        let v = IsaVersion::new(1, 5);
        assert_eq!(v.to_string(), "1.5");
    }

    #[test]
    fn version_parsing() {
        let v: IsaVersion = "1.5".parse().unwrap();
        assert_eq!(v.major, 1);
        assert_eq!(v.minor, 5);
    }

    #[test]
    fn version_parsing_errors() {
        assert!("1".parse::<IsaVersion>().is_err());
        assert!("1.2.3".parse::<IsaVersion>().is_err());
        assert!("a.b".parse::<IsaVersion>().is_err());
    }

    #[test]
    fn version_ordering() {
        let v1_0 = IsaVersion::new(1, 0);
        let v1_2 = IsaVersion::new(1, 2);
        let v2_0 = IsaVersion::new(2, 0);

        assert!(v1_0 < v1_2);
        assert!(v1_2 < v2_0);
        assert!(v1_0 < v2_0);
    }

    #[test]
    fn boundary_max_version_numbers() {
        let v_max = IsaVersion::new(u16::MAX, u16::MAX);
        assert_eq!(v_max.major, u16::MAX);
        assert_eq!(v_max.minor, u16::MAX);
        assert!(v_max.is_compatible_with(v_max));
    }
}
