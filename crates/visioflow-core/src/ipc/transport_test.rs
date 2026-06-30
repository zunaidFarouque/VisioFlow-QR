use std::io::Cursor;

use super::{read_line, write_line};

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

#[test]
#[ignore = "manual: real socket integration; run with --ignored"]
fn socket_ping_pong_roundtrip() {
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration;

    use super::{SocketIpcClient, SocketIpcServer};
    use crate::ipc::{ClientMessage, IpcClient, IpcServer, ServerMessage};

    let socket_name = format!("visioflow-test-ping-{}", std::process::id());
    let server_name = socket_name.clone();
    let (ready_tx, ready_rx) = mpsc::channel();

    let server_thread = thread::spawn(move || {
        let mut server = SocketIpcServer::bind(&server_name, |msg| match msg {
            ClientMessage::Ping { id } => ServerMessage::Pong { id },
            other => ServerMessage::Error {
                id: other.id(),
                message: "unexpected".into(),
            },
        })
        .expect("bind");
        ready_tx.send(()).expect("ready");
        server.accept().expect("accept");
        server.handle_one_message().expect("handle");
    });

    ready_rx
        .recv_timeout(Duration::from_secs(5))
        .expect("server ready");

    let mut client = SocketIpcClient::connect(&socket_name).expect("connect");
    client
        .send_request(ClientMessage::Ping { id: 99 })
        .expect("send");
    let response = client.recv_response().expect("recv");
    assert_eq!(response, ServerMessage::Pong { id: 99 });

    server_thread.join().expect("server thread");
}
