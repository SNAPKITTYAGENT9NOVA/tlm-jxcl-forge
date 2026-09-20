# jxcl-rpc

A minimal RPC server/client implementing jxcl-protocol over line-delimited JSON on TCP.

## Purpose

Provides `RpcServer` and `RpcClient` for remote control of jxcl-machines via a simple line-delimited JSON protocol over TCP. Each request and response is a single JSON object terminated by a newline, with no length-prefixing or other framing overhead.

## Public API

- `RpcServer`: Binds to a TCP port and accepts client connections, reading line-delimited JSON `Request` values and writing back line-delimited JSON `Response` values.
- `RpcClient`: Connects to a remote server, sends a `Request` as one JSON line, and reads back one `Response` JSON line.

## Wire Framing

The protocol uses line-delimited JSON:

- Each `Request` is serialized as a single JSON object and terminated with a newline (`\n`)
- Each `Response` is serialized as a single JSON object and terminated with a newline (`\n`)
- No length-prefixing, length fields, or other binary framing is used
- Empty lines are ignored by the server

## Implementation Notes

- Both `RpcServer` and `RpcClient` use tokio for async I/O
- `RpcServer::bind()` binds to an address (supporting ephemeral ports via `"127.0.0.1:0"` for testing)
- `RpcServer::accept_one()` accepts a single connection, handles all requests on it, and returns when the client closes
- `RpcServer::listen()` accepts connections in a loop, spawning a tokio task for each
- `RpcClient::send_request()` sends one request and reads back one response, suitable for simple request-response patterns
- Network connection strings are logged using `redact_url_credentials()` from `jxcl-network` to avoid logging sensitive data
- Uses `#![forbid(unsafe_code)]` — no unsafe code anywhere

## Testing

**Unit tests (5):**
- `request_serializes_to_json`: Verify `Request` serializes correctly
- `request_deserializes_from_json`: Verify `Request` deserializes from JSON
- `response_serializes_to_json`: Verify `Response` serializes correctly
- `response_deserializes_from_json`: Verify `Response` deserializes from JSON
- `rpc_error_display`: Verify `RpcError` displays correctly

**Integration tests (3):**
- `integration_client_server_get_state`: End-to-end roundtrip of `GetState` request/response over TCP
- `integration_client_server_assemble_program`: End-to-end `AssembleProgram` request over TCP
- `integration_client_server_run_program`: End-to-end `RunProgram` request over TCP

Integration tests spawn a real server on an ephemeral TCP port, connect a real client, and verify the JSON framing works correctly end-to-end.

## Dependencies

Workspace crates:

- `jxcl-protocol`: Request/Response types
- `jxcl-network`: URL credential redaction utilities

External crates:

- `tokio` (version 1, features: rt, net, io-util, time)
- `serde_json` (version 1.0)

## License

AGPLv3 / Commercial dual-license. See `LICENSE-AGPL`, `LICENSE-COMMERCIAL`, and `LICENSE-NOTICE` at the repository root.
