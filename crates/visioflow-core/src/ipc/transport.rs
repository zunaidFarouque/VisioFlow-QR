use std::io::{BufRead, Write};

use interprocess::local_socket::traits::{Listener, Stream as StreamTrait};
use interprocess::local_socket::{ListenerOptions, Stream};

use crate::error::{Result, VisioFlowError};
use crate::ipc::{
    deserialize_client_line, deserialize_server_line, serialize_client_line, serialize_server_line,
    ClientMessage, IpcClient, IpcServer, ServerMessage,
};
use crate::ipc::paths::parse_socket_name;

/// Read one newline-terminated line from a byte stream.
pub fn read_line<R: BufRead>(reader: &mut R) -> Result<String> {
    let mut line = String::new();
    reader.read_line(&mut line).map_err(VisioFlowError::Io)?;
    if line.is_empty() {
        return Err(VisioFlowError::Ipc(
            "connection closed before message".into(),
        ));
    }
    Ok(line)
}

/// Write one newline-terminated JSON line to a stream.
pub fn write_line<W: Write>(writer: &mut W, line: &str) -> Result<()> {
    writer
        .write_all(line.as_bytes())
        .map_err(VisioFlowError::Io)?;
    writer.flush().map_err(VisioFlowError::Io)
}

/// CLI-side socket transport implementing [`IpcClient`].
pub struct SocketIpcClient {
    stream: Stream,
    read_buf: String,
}

impl SocketIpcClient {
    pub fn connect(path: &str) -> Result<Self> {
        let name = parse_socket_name(path)?;
        let stream = StreamTrait::connect(name).map_err(|e| {
            VisioFlowError::Ipc(format!("connect to {path} failed: {e}"))
        })?;
        Ok(Self {
            stream,
            read_buf: String::new(),
        })
    }
}

impl IpcClient for SocketIpcClient {
    fn send_request(&mut self, request: ClientMessage) -> Result<()> {
        let line = serialize_client_line(&request)
            .map_err(|e| VisioFlowError::Ipc(format!("serialize request: {e}")))?;
        write_line(&mut self.stream, &line)
    }

    fn recv_response(&mut self) -> Result<ServerMessage> {
        self.read_buf.clear();
        read_line_from_stream(&mut self.stream, &mut self.read_buf)?;
        let response = deserialize_server_line(&self.read_buf)
            .map_err(|e| VisioFlowError::Ipc(format!("deserialize response: {e}")))?;
        Ok(response)
    }
}

fn read_line_from_stream(stream: &mut Stream, buf: &mut String) -> Result<()> {
    use std::io::Read;
    buf.clear();
    loop {
        let mut byte = [0u8; 1];
        match stream.read(&mut byte) {
            Ok(0) => {
                if buf.is_empty() {
                    return Err(VisioFlowError::Ipc(
                        "connection closed before message".into(),
                    ));
                }
                break;
            }
            Ok(_) => {
                if byte[0] == b'\n' {
                    break;
                }
                buf.push(byte[0] as char);
            }
            Err(e) => return Err(VisioFlowError::Io(e)),
        }
    }
    Ok(())
}

/// Daemon-side socket listener implementing [`IpcServer`] with a message handler closure.
pub struct SocketIpcServer<F> {
    listener: interprocess::local_socket::Listener,
    stream: Option<Stream>,
    read_buf: String,
    handler: F,
}

impl<F> SocketIpcServer<F>
where
    F: FnMut(ClientMessage) -> ServerMessage + Send + Sync,
{
    pub fn bind(path: &str, handler: F) -> Result<Self> {
        let name = parse_socket_name(path)?;
        let listener = ListenerOptions::new()
            .name(name)
            .create_sync()
            .map_err(|e| VisioFlowError::Ipc(format!("bind {path} failed: {e}")))?;
        Ok(Self {
            listener,
            stream: None,
            read_buf: String::new(),
            handler,
        })
    }

    fn read_request(&mut self) -> Result<ClientMessage> {
        let stream = self
            .stream
            .as_mut()
            .ok_or_else(|| VisioFlowError::Ipc("no active connection".into()))?;
        self.read_buf.clear();
        read_line_from_stream(stream, &mut self.read_buf)?;
        deserialize_client_line(&self.read_buf)
            .map_err(|e| VisioFlowError::Ipc(format!("deserialize request: {e}")))
    }

    fn write_response(&mut self, response: &ServerMessage) -> Result<()> {
        let stream = self
            .stream
            .as_mut()
            .ok_or_else(|| VisioFlowError::Ipc("no active connection".into()))?;
        let line = serialize_server_line(response)
            .map_err(|e| VisioFlowError::Ipc(format!("serialize response: {e}")))?;
        write_line(stream, &line)
    }
}

impl<F> IpcServer for SocketIpcServer<F>
where
    F: FnMut(ClientMessage) -> ServerMessage + Send + Sync,
{
    fn accept(&mut self) -> Result<()> {
        let stream = self
            .listener
            .accept()
            .map_err(|e| VisioFlowError::Ipc(format!("accept failed: {e}")))?;
        self.stream = Some(stream);
        Ok(())
    }

    fn handle_one_message(&mut self) -> Result<ServerMessage> {
        let request = self.read_request()?;
        let response = (self.handler)(request);
        self.write_response(&response)?;
        Ok(response)
    }
}
