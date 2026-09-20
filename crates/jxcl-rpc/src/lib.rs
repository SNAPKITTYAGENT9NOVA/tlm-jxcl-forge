// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! A minimal RPC server/client implementing jxcl-protocol over line-delimited JSON on TCP.
//!
//! # Purpose
//!
//! Provides `RpcServer` and `RpcClient` for remote control of jxcl-machines via a simple
//! line-delimited JSON protocol over TCP. Requests and responses are serialized as single
//! JSON objects per line, newline-terminated.
//!
//! # Wire Framing
//!
//! - Each `Request` is serialized as JSON and terminated with a newline (`\n`)
//! - Each `Response` is serialized as JSON and terminated with a newline (`\n`)
//! - No length-prefixing or other framing is used; only line-based framing

#![forbid(unsafe_code)]

use jxcl_network::redact_url_credentials;
use jxcl_protocol::{Request, Response};
use std::net::SocketAddr;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};

/// An error type for RPC operations.
#[derive(Debug)]
pub enum RpcError {
    /// I/O error during connection or read/write.
    Io(std::io::Error),
    /// JSON serialization error.
    Serialization(serde_json::Error),
    /// JSON deserialization error.
    Deserialization(serde_json::Error),
    /// Other error.
    Other(String),
}

impl From<std::io::Error> for RpcError {
    fn from(e: std::io::Error) -> Self {
        RpcError::Io(e)
    }
}

impl From<serde_json::Error> for RpcError {
    fn from(e: serde_json::Error) -> Self {
        if e.is_io() {
            RpcError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                e.to_string(),
            ))
        } else {
            RpcError::Deserialization(e)
        }
    }
}

impl std::fmt::Display for RpcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RpcError::Io(e) => write!(f, "I/O error: {}", e),
            RpcError::Serialization(e) => write!(f, "serialization error: {}", e),
            RpcError::Deserialization(e) => write!(f, "deserialization error: {}", e),
            RpcError::Other(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for RpcError {}

/// A minimal RPC server that listens on a TCP port and handles requests.
pub struct RpcServer {
    /// The local address this server is listening on.
    listener: TcpListener,
    /// The actual bound address (including assigned port if 0 was requested).
    local_addr: SocketAddr,
}

impl RpcServer {
    /// Create and bind a new RPC server to the given address.
    ///
    /// # Arguments
    ///
    /// * `addr` - The address and port to bind to (e.g., "127.0.0.1:0" for ephemeral port)
    ///
    /// # Returns
    ///
    /// A new `RpcServer` if binding succeeds, or an error if it fails.
    pub async fn bind(addr: &str) -> Result<Self, RpcError> {
        let listener = TcpListener::bind(addr).await?;
        let local_addr = listener.local_addr()?;
        Ok(RpcServer {
            listener,
            local_addr,
        })
    }

    /// Get the local address this server is bound to.
    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    /// Accept one connection and handle it. Returns when the connection closes.
    ///
    /// This method reads one or more requests from the connected client and
    /// echoes back the corresponding response for each. It is designed for testing
    /// and minimal RPC use cases.
    pub async fn accept_one(&self) -> Result<(), RpcError> {
        let (socket, peer_addr) = self.listener.accept().await?;
        let safe_peer = redact_url_credentials(&peer_addr.to_string());
        eprintln!("RpcServer: accepted connection from {}", safe_peer);

        handle_client(socket).await
    }

    /// Accept connections in a loop until an error occurs or the listener is dropped.
    ///
    /// Each connection is handled by spawning a tokio task.
    pub async fn listen(self) -> Result<(), RpcError> {
        loop {
            let (socket, peer_addr) = self.listener.accept().await?;
            let safe_peer = redact_url_credentials(&peer_addr.to_string());
            eprintln!("RpcServer: accepted connection from {}", safe_peer);

            tokio::spawn(async move {
                if let Err(e) = handle_client(socket).await {
                    eprintln!("RpcServer: error handling client {}: {}", safe_peer, e);
                }
            });
        }
    }
}

/// Internal helper to handle a single client connection.
///
/// Reads line-delimited JSON requests and writes line-delimited JSON responses.
async fn handle_client(socket: TcpStream) -> Result<(), RpcError> {
    let (reader, mut writer) = socket.into_split();
    let mut buf_reader = BufReader::new(reader);
    let mut line = String::new();

    loop {
        line.clear();
        let n = buf_reader.read_line(&mut line).await?;
        if n == 0 {
            // EOF: client closed connection
            break;
        }

        // Trim the newline for deserialization
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue; // Skip empty lines
        }

        // Deserialize the request
        let _request: Request = match serde_json::from_str(trimmed) {
            Ok(req) => req,
            Err(e) => {
                let error_response = Response::error(format!("invalid request JSON: {}", e));
                let response_line = format!("{}\n", serde_json::to_string(&error_response)?);
                writer.write_all(response_line.as_bytes()).await?;
                continue;
            }
        };

        // In a real implementation, this would actually execute the request.
        // For now, we just echo back an error response.
        let response = Response::error("RPC handler not yet implemented; this is a test echo");

        // Serialize and send the response
        let response_json = serde_json::to_string(&response)?;
        let response_line = format!("{}\n", response_json);
        writer.write_all(response_line.as_bytes()).await?;
    }

    Ok(())
}

/// A minimal RPC client that connects to a server and sends requests.
pub struct RpcClient {
    /// The connected socket.
    socket: TcpStream,
}

impl RpcClient {
    /// Connect to a remote RPC server.
    ///
    /// # Arguments
    ///
    /// * `addr` - The address and port of the server (e.g., "127.0.0.1:9000")
    ///
    /// # Returns
    ///
    /// A new `RpcClient` if connection succeeds, or an error if it fails.
    pub async fn connect(addr: &str) -> Result<Self, RpcError> {
        let socket = TcpStream::connect(addr).await?;
        let safe_addr = redact_url_credentials(addr);
        eprintln!("RpcClient: connected to {}", safe_addr);
        Ok(RpcClient { socket })
    }

    /// Send a request and read back the response.
    ///
    /// # Arguments
    ///
    /// * `request` - The request to send
    ///
    /// # Returns
    ///
    /// The response from the server, or an error if communication fails.
    pub async fn send_request(&mut self, request: Request) -> Result<Response, RpcError> {
        // Serialize the request to JSON
        let request_json = serde_json::to_string(&request)?;
        let request_line = format!("{}\n", request_json);

        // Send the request
        self.socket.write_all(request_line.as_bytes()).await?;

        // Read the response
        let (reader, _) = self.socket.split();
        let mut buf_reader = BufReader::new(reader);
        let mut line = String::new();

        let n = buf_reader.read_line(&mut line).await?;
        if n == 0 {
            return Err(RpcError::Other("server closed connection".to_string()));
        }

        // Deserialize the response
        let trimmed = line.trim();
        let response = serde_json::from_str(trimmed)?;
        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- Unit tests: framing and serialization ----

    #[test]
    fn request_serializes_to_json() {
        let req = Request::GetState;
        let json = serde_json::to_string(&req).expect("serialization failed");
        assert!(json.contains("GetState"));
    }

    #[test]
    fn request_deserializes_from_json() {
        let json = r#"{"type":"GetState"}"#;
        let req: Request = serde_json::from_str(json).expect("deserialization failed");
        assert_eq!(req, Request::GetState);
    }

    #[test]
    fn response_serializes_to_json() {
        let resp = Response::error("test error");
        let json = serde_json::to_string(&resp).expect("serialization failed");
        assert!(json.contains("error"));
        assert!(json.contains("test error"));
    }

    #[test]
    fn response_deserializes_from_json() {
        let json = r#"{"status":"error","message":"test error"}"#;
        let resp: Response = serde_json::from_str(json).expect("deserialization failed");
        match resp {
            Response::Error { message } => {
                assert_eq!(message, "test error");
            }
            _ => panic!("expected Error variant"),
        }
    }

    // ---- Integration tests: client-server roundtrip ----

    #[tokio::test]
    async fn integration_client_server_get_state() {
        // Start server on ephemeral port
        let server = RpcServer::bind("127.0.0.1:0")
            .await
            .expect("failed to bind server");
        let server_addr = server.local_addr();

        // Spawn server in background
        let server_handle = tokio::spawn(async move {
            let _ = server.accept_one().await;
        });

        // Give server time to start listening
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Connect client
        let mut client = RpcClient::connect(&server_addr.to_string())
            .await
            .expect("failed to connect client");

        // Send request
        let request = Request::GetState;
        let response = client
            .send_request(request)
            .await
            .expect("failed to get response");

        // Verify response
        match response {
            Response::Error { message } => {
                // Server echoes back an error for now (not implemented)
                assert!(message.contains("not yet implemented"));
            }
            _ => panic!("expected Error variant"),
        }

        // Wait for server task to finish
        let _ = tokio::time::timeout(tokio::time::Duration::from_secs(1), server_handle).await;
    }

    #[tokio::test]
    async fn integration_client_server_assemble_program() {
        // Start server on ephemeral port
        let server = RpcServer::bind("127.0.0.1:0")
            .await
            .expect("failed to bind server");
        let server_addr = server.local_addr();

        // Spawn server in background
        let server_handle = tokio::spawn(async move {
            let _ = server.accept_one().await;
        });

        // Give server time to start listening
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Connect client
        let mut client = RpcClient::connect(&server_addr.to_string())
            .await
            .expect("failed to connect client");

        // Send AssembleProgram request
        let request = Request::AssembleProgram {
            bytecode: vec![0x60],
            memory_size: None,
        };
        let response = client
            .send_request(request)
            .await
            .expect("failed to get response");

        // Verify response is an error (not implemented)
        match response {
            Response::Error { .. } => {
                // Expected
            }
            _ => panic!("expected Error variant"),
        }

        // Wait for server task to finish
        let _ = tokio::time::timeout(tokio::time::Duration::from_secs(1), server_handle).await;
    }

    #[tokio::test]
    async fn integration_client_server_run_program() {
        // Start server on ephemeral port
        let server = RpcServer::bind("127.0.0.1:0")
            .await
            .expect("failed to bind server");
        let server_addr = server.local_addr();

        // Spawn server in background
        let server_handle = tokio::spawn(async move {
            let _ = server.accept_one().await;
        });

        // Give server time to start listening
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Connect client
        let mut client = RpcClient::connect(&server_addr.to_string())
            .await
            .expect("failed to connect client");

        // Send RunProgram request
        let request = Request::RunProgram {
            instruction_limit: Some(1000),
            enable_trace: false,
        };
        let response = client
            .send_request(request)
            .await
            .expect("failed to get response");

        // Verify response
        match response {
            Response::Error { .. } => {
                // Expected (not implemented)
            }
            _ => panic!("expected Error variant"),
        }

        // Wait for server task to finish
        let _ = tokio::time::timeout(tokio::time::Duration::from_secs(1), server_handle).await;
    }

    #[test]
    fn rpc_error_display() {
        let err = RpcError::Other("test error".to_string());
        assert_eq!(format!("{}", err), "test error");
    }
}
