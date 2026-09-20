# jxcl-network

Low-level network utilities: connection-string credential redaction, address/port parsing.

## Purpose

Provides safe, best-effort credential redaction for URLs and connection strings before logging. Prevents accidental exposure of embedded credentials (passwords, API tokens) in logs by replacing the userinfo portion (`user:password@`) with `***@`.

## Architecture

**Owns:** `redact_url_credentials` and address-parsing helpers.

**Category:** network · **Source:** extraction:photo-cache-service/src/lib.rs

## Public API

`redact_url_credentials(url: &str) -> String` — Replaces any `user:password@` userinfo in a URL with `***@` for safe logging.

## Dependencies

Workspace crates: *(none)*

External crates: *(none)*

## Implementation Notes

- **Best-effort string surgery:** Deliberately uses simple string operations rather than full URL parsing. This ensures parse failures never fall back to printing the original credential-bearing string.
- **Scheme detection:** Looks for `://` to identify where the scheme ends, then searches for `@` in the remainder to locate the userinfo/host boundary.
- **Safe default:** Returns the original string unchanged if no scheme or userinfo is detected, rather than attempting to parse or guess.
- **Special character handling:** Works with special characters in passwords (including `@` and percent-encoded sequences) because it treats the last `@` before a path/port as the userinfo boundary.

## Testing

Unit tests cover:
- Redis URLs with password only (`:password@`)
- Redis URLs with username and password (`user:password@`)
- Credentials in various schemes (http, https, postgres, mysql, redis)
- URLs without credentials (returned unchanged)
- URLs without schemes (returned unchanged)
- Malformed URLs (returned unchanged)
- Special characters in passwords
- Multiple `@` signs in password (only first treated as separator)
- IPv6 addresses in URLs
- Paths, query strings, and fragments preservation

Total: 11 unit tests.

## License

AGPLv3 / Commercial dual-license. See `LICENSE-AGPL`, `LICENSE-COMMERCIAL`, and `LICENSE-NOTICE` at the repository root.
