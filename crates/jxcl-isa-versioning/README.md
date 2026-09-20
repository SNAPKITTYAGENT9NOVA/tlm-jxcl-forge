# jxcl-isa-versioning

ISA version negotiation and backward-compatibility checking for binaries.

## Purpose

Prevents silently misdecoding old binaries when the ISA evolves by enforcing semantic versioning:
- Major version changes are breaking (not compatible)
- Minor version changes are backward-compatible (newer decoders can understand old formats)
- Binary can only run on decoders with same major version and >= minor version

## Public API

- `IsaVersion::new(major, minor) -> IsaVersion` - Create a version
- `IsaVersion::current() -> IsaVersion` - Get the current ISA version
- `IsaVersion::is_compatible_with(binary_version) -> bool` - Check if decoder can run binary
- `IsaVersion::is_newer_than_or_equal(other) -> bool` - Version comparison
- `IsaVersion::is_older_than(other) -> bool` - Version comparison
- Display via `Display` trait, parsing via `FromStr` trait

## Semantic Versioning Model

A decoder supporting version X.Y is compatible with binaries built for X.Z where Z <= Y:
- Same major version required (breaking changes are not compatible)
- Decoder's minor >= binary's minor (newer decoders understand old formats)

Examples:
- Decoder 1.2 can run binaries compiled for 1.0, 1.1, 1.2 ✓
- Decoder 1.0 cannot run binaries compiled for 1.2 ✗
- Decoder 2.0 cannot run binaries compiled for 1.x ✗

## Testing

The crate includes 12 unit tests covering:
- Current version derivation from binary format version
- Compatibility checks (same version, older minor, newer minor)
- Version comparison operations
- Display formatting and parsing
- Boundary cases (max version numbers)
- Ordering relationships

## License

AGPLv3 / Commercial dual-license. See `LICENSE-AGPL`, `LICENSE-COMMERCIAL`, and `LICENSE-NOTICE` at the repository root.
