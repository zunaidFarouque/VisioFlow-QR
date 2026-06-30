use std::collections::BTreeMap;
use std::io::Cursor;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::Duration;

use tempfile::TempDir;

use super::{read_line, write_line, SocketIpcClient, SocketIpcServer};
use crate::ipc::{ClientMessage, DaemonHandler, IpcClient, IpcServer, ServerMessage};
use crate::rules::{FileRuleStore, Rule, RuleStore};

#[test]
fn read_line_reads_until_newline() {
    let mut reader = Cursor::new(b"{\"type\":\"ping\",\"id\":1}\n");
    let line = read_line(&mut reader).expect("read");
    assert_eq!(line.trim_end(), "{\"type\":\"ping\",\"id\":1}");
}

#[test]
fn write_line_writes_bytes_and_flushes() {
    let mut buf = Vec::new();
    write_line(&mut buf, "{\"ok\":true}\n").expect("write");
    assert_eq!(buf, b"{\"ok\":true}\n");
}

/// Unique IPC socket path per test run (filesystem UDS on Linux, named pipe on Windows).
fn socket_path_for_test(temp_dir: &TempDir) -> String {
    let unique = format!(
        "visioflow-ipc-e2e-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    );

    #[cfg(windows)]
    {
        let _ = temp_dir;
        format!(r"\\.\pipe\{unique}")
    }
    #[cfg(not(windows))]
    {
        temp_dir
            .path()
            .join(format!("{unique}.sock"))
            .to_string_lossy()
            .into_owned()
    }
}

fn temp_daemon_handler() -> (TempDir, DaemonHandler) {
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join("rules.json");
    let store = FileRuleStore::new(path);
    let mut rules = BTreeMap::new();
    let mut rule = Rule::new("asset");
    rule.regex = Some(r"ASSET:(?P<asset>\d+)".to_owned());
    rules.insert("asset".to_owned(), rule);
    store.save_all(&rules).expect("save rules");
    let handler = DaemonHandler::new(store).expect("handler");
    (dir, handler)
}

fn spawn_daemon_server(
    socket_path: String,
    handler: DaemonHandler,
) -> (mpsc::Receiver<()>, thread::JoinHandle<()>) {
    let (ready_tx, ready_rx) = mpsc::channel();
    let handler = Arc::new(Mutex::new(handler));

    let join = thread::spawn(move || {
        let handler = Arc::clone(&handler);
        let mut server = SocketIpcServer::bind(&socket_path, move |msg| {
            handler.lock().expect("handler lock").handle(msg, false)
        })
        .expect("bind");
        ready_tx.send(()).expect("ready");
        server.accept().expect("accept");
        server.handle_one_message().expect("ping");
        server.handle_one_message().expect("execute_rule");
    });

    (ready_rx, join)
}

fn wait_for_server(ready_rx: mpsc::Receiver<()>) {
    ready_rx
        .recv_timeout(Duration::from_secs(5))
        .expect("server ready within 5s");
}

/// Real local-socket round-trip: Ping/Pong and ExecuteRule/RuleResult via [`DaemonHandler`].
#[test]
fn socket_ipc_roundtrip_ping_and_execute_rule() {
    let (temp_dir, handler) = temp_daemon_handler();
    let socket_path = socket_path_for_test(&temp_dir);
    let (ready_rx, server_thread) = spawn_daemon_server(socket_path.clone(), handler);
    wait_for_server(ready_rx);

    let mut client = SocketIpcClient::connect(&socket_path).expect("connect");

    client
        .send_request(ClientMessage::Ping { id: 1 })
        .expect("send ping");
    let pong = client.recv_response().expect("recv pong");
    assert_eq!(pong, ServerMessage::Pong { id: 1 });

    client
        .send_request(ClientMessage::ExecuteRule {
            id: 2,
            name: "asset".into(),
            payload: "ASSET:42".into(),
        })
        .expect("send execute_rule");
    let result = client.recv_response().expect("recv rule_result");

    match result {
        ServerMessage::RuleResult {
            id,
            vars,
            exit_code,
        } => {
            assert_eq!(id, 2);
            assert_eq!(vars.get("QR_RAW").map(String::as_str), Some("ASSET:42"));
            assert_eq!(vars.get("QR_VAR_ASSET").map(String::as_str), Some("42"));
            assert!(exit_code.is_none());
        }
        other => panic!("expected RuleResult, got {other:?}"),
    }

    server_thread.join().expect("server thread");
}
