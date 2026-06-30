//! Newline-delimited JSON IPC protocol between CLI and daemon.
//!
//! See `DOCs/IPC_PROTOCOL.md` for the wire format.

mod handler;
mod paths;
mod transport;

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::error::Result;

pub use handler::{route_with_native, DaemonHandler, MemoryRuleStore};
pub use paths::{default_socket_path, parse_socket_name, DEFAULT_SOCKET_NAME};
pub use transport::{read_line, write_line, SocketIpcClient, SocketIpcServer};

/// Correlates a client request with its server response.
pub type RequestId = u64;

/// Client → daemon message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    Ping {
        id: RequestId,
    },
    ExecuteRule {
        id: RequestId,
        name: String,
        payload: String,
    },
    ListRules {
        id: RequestId,
    },
    Reload {
        id: RequestId,
    },
}

/// Daemon → client message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    Pong {
        id: RequestId,
    },
    RuleResult {
        id: RequestId,
        vars: HashMap<String, String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        exit_code: Option<i32>,
    },
    Error {
        id: RequestId,
        message: String,
    },
    RulesList {
        id: RequestId,
        names: Vec<String>,
    },
}

/// Serialize a client message as one newline-terminated JSON line.
pub fn serialize_client_line(
    msg: &ClientMessage,
) -> std::result::Result<String, serde_json::Error> {
    let mut line = serde_json::to_string(msg)?;
    line.push('\n');
    Ok(line)
}

/// Parse one client message from a JSON line (trailing newline optional).
pub fn deserialize_client_line(
    line: &str,
) -> std::result::Result<ClientMessage, serde_json::Error> {
    serde_json::from_str(line.trim_end_matches('\n'))
}

/// Serialize a server message as one newline-terminated JSON line.
pub fn serialize_server_line(
    msg: &ServerMessage,
) -> std::result::Result<String, serde_json::Error> {
    let mut line = serde_json::to_string(msg)?;
    line.push('\n');
    Ok(line)
}

/// Parse one server message from a JSON line (trailing newline optional).
pub fn deserialize_server_line(
    line: &str,
) -> std::result::Result<ServerMessage, serde_json::Error> {
    serde_json::from_str(line.trim_end_matches('\n'))
}

/// CLI-side IPC transport (send request, receive response).
#[cfg_attr(test, mockall::automock)]
pub trait IpcClient: Send + Sync {
    fn send_request(&mut self, request: ClientMessage) -> Result<()>;
    fn recv_response(&mut self) -> Result<ServerMessage>;
}

/// Daemon-side IPC transport (accept connection, handle one request/response cycle).
#[cfg_attr(test, mockall::automock)]
pub trait IpcServer: Send + Sync {
    fn accept(&mut self) -> Result<()>;
    fn handle_one_message(&mut self) -> Result<ServerMessage>;
}

impl ClientMessage {
    /// Returns the request id for correlation with the server response.
    #[must_use]
    pub fn id(&self) -> RequestId {
        match self {
            Self::Ping { id }
            | Self::ExecuteRule { id, .. }
            | Self::ListRules { id }
            | Self::Reload { id } => *id,
        }
    }
}

impl ServerMessage {
    /// Returns the correlated request id for any server message variant.
    #[must_use]
    pub fn id(&self) -> RequestId {
        match self {
            Self::Pong { id }
            | Self::RuleResult { id, .. }
            | Self::Error { id, .. }
            | Self::RulesList { id, .. } => *id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn client_message_ping_roundtrip() {
        let msg = ClientMessage::Ping { id: 1 };
        let line = serialize_client_line(&msg).unwrap();
        let parsed = deserialize_client_line(&line).unwrap();
        assert_eq!(parsed, msg);
        assert!(line.ends_with('\n'));
    }

    #[test]
    fn client_message_execute_rule_roundtrip() {
        let msg = ClientMessage::ExecuteRule {
            id: 42,
            name: "wifi".into(),
            payload: "WIFI:T:MyNet;P:secret;;".into(),
        };
        let line = serialize_client_line(&msg).unwrap();
        assert_eq!(deserialize_client_line(&line).unwrap(), msg);
    }

    #[test]
    fn client_message_list_rules_roundtrip() {
        let msg = ClientMessage::ListRules { id: 7 };
        let line = serialize_client_line(&msg).unwrap();
        assert_eq!(deserialize_client_line(&line).unwrap(), msg);
    }

    #[test]
    fn client_message_reload_roundtrip() {
        let msg = ClientMessage::Reload { id: 99 };
        let line = serialize_client_line(&msg).unwrap();
        assert_eq!(deserialize_client_line(&line).unwrap(), msg);
    }

    #[test]
    fn server_message_pong_roundtrip() {
        let msg = ServerMessage::Pong { id: 1 };
        let line = serialize_server_line(&msg).unwrap();
        assert_eq!(deserialize_server_line(&line).unwrap(), msg);
        assert!(line.ends_with('\n'));
    }

    #[test]
    fn server_message_rule_result_roundtrip() {
        let mut vars = HashMap::new();
        vars.insert("QR_RAW".into(), "asset:123".into());
        vars.insert("QR_VAR_ASSET".into(), "123".into());
        let msg = ServerMessage::RuleResult {
            id: 42,
            vars,
            exit_code: Some(0),
        };
        let line = serialize_server_line(&msg).unwrap();
        assert_eq!(deserialize_server_line(&line).unwrap(), msg);
    }

    #[test]
    fn server_message_rule_result_omits_null_exit_code() {
        let msg = ServerMessage::RuleResult {
            id: 1,
            vars: HashMap::new(),
            exit_code: None,
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(!json.contains("exit_code"));
    }

    #[test]
    fn server_message_error_roundtrip() {
        let msg = ServerMessage::Error {
            id: 5,
            message: "rule not found".into(),
        };
        let line = serialize_server_line(&msg).unwrap();
        assert_eq!(deserialize_server_line(&line).unwrap(), msg);
    }

    #[test]
    fn server_message_rules_list_roundtrip() {
        let msg = ServerMessage::RulesList {
            id: 3,
            names: vec!["wifi".into(), "uri".into()],
        };
        let line = serialize_server_line(&msg).unwrap();
        assert_eq!(deserialize_server_line(&line).unwrap(), msg);
    }

    #[test]
    fn mock_client_server_ping_pong_handshake() {
        let mut server = MockIpcServer::new();
        server.expect_accept().times(1).returning(|| Ok(()));
        server
            .expect_handle_one_message()
            .times(1)
            .returning(|| Ok(ServerMessage::Pong { id: 1 }));

        let mut client = MockIpcClient::new();
        client
            .expect_send_request()
            .times(1)
            .with(mockall::predicate::eq(ClientMessage::Ping { id: 1 }))
            .returning(|_| Ok(()));
        client
            .expect_recv_response()
            .times(1)
            .returning(|| Ok(ServerMessage::Pong { id: 1 }));

        server.accept().unwrap();
        client.send_request(ClientMessage::Ping { id: 1 }).unwrap();
        let client_response = client.recv_response().unwrap();
        let server_response = server.handle_one_message().unwrap();

        assert_eq!(client_response, ServerMessage::Pong { id: 1 });
        assert_eq!(server_response, ServerMessage::Pong { id: 1 });
        assert_eq!(client_response.id(), server_response.id());
    }

    #[test]
    fn mock_execute_rule_handshake() {
        let request = ClientMessage::ExecuteRule {
            id: 10,
            name: "asset".into(),
            payload: "asset:999".into(),
        };
        let mut vars = HashMap::new();
        vars.insert("QR_VAR_ASSET".into(), "999".into());
        let server_vars = vars.clone();
        let client_vars = vars;

        let mut server = MockIpcServer::new();
        server.expect_accept().returning(|| Ok(()));
        server.expect_handle_one_message().returning(move || {
            Ok(ServerMessage::RuleResult {
                id: 10,
                vars: server_vars.clone(),
                exit_code: Some(0),
            })
        });

        let mut client = MockIpcClient::new();
        client
            .expect_send_request()
            .with(mockall::predicate::eq(request.clone()))
            .returning(|_| Ok(()));
        client.expect_recv_response().returning(move || {
            Ok(ServerMessage::RuleResult {
                id: 10,
                vars: client_vars.clone(),
                exit_code: Some(0),
            })
        });

        server.accept().unwrap();
        client.send_request(request).unwrap();
        let response = client.recv_response().unwrap();

        match response {
            ServerMessage::RuleResult {
                id,
                vars,
                exit_code,
            } => {
                assert_eq!(id, 10);
                assert_eq!(vars.get("QR_VAR_ASSET").map(String::as_str), Some("999"));
                assert_eq!(exit_code, Some(0));
            }
            other => panic!("expected RuleResult, got {other:?}"),
        }
    }
}

#[cfg(test)]
mod handler_test;

#[cfg(test)]
mod transport_test;
