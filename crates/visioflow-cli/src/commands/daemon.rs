//! Background daemon: start/stop/status/reload and IPC server loop.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use visioflow_core::{
    default_socket_path, parse_socket_name, ClientMessage, DaemonHandler, FileRuleStore,
    IpcClient, IpcServer, ServerMessage, SocketIpcClient, SocketIpcServer, VisioFlowError,
};

/// PID file alongside the rules store under the user config directory.
pub fn default_pid_path() -> PathBuf {
    FileRuleStore::with_default_path()
        .path()
        .parent()
        .map_or_else(|| PathBuf::from("daemon.pid"), |p| p.join("daemon.pid"))
}

pub fn read_pid(path: &Path) -> Result<u32, VisioFlowError> {
    let text = fs::read_to_string(path).map_err(VisioFlowError::Io)?;
    text.trim()
        .parse::<u32>()
        .map_err(|e| VisioFlowError::Ipc(format!("invalid pid file: {e}")))
}

pub fn write_pid(path: &Path, pid: u32) -> Result<(), VisioFlowError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(VisioFlowError::Io)?;
    }
    fs::write(path, format!("{pid}\n")).map_err(VisioFlowError::Io)
}

pub fn remove_pid(path: &Path) -> Result<(), VisioFlowError> {
    if path.exists() {
        fs::remove_file(path).map_err(VisioFlowError::Io)?;
    }
    Ok(())
}

pub fn is_process_alive(pid: u32) -> bool {
    #[cfg(windows)]
    {
        use std::process::Command as ProcCommand;
        let output = ProcCommand::new("tasklist")
            .args(["/FI", &format!("PID eq {pid}")])
            .output();
        match output {
            Ok(out) => {
                let text = String::from_utf8_lossy(&out.stdout);
                text.contains(&pid.to_string())
            }
            Err(_) => false,
        }
    }
    #[cfg(not(windows))]
    {
        use std::process::Command as ProcCommand;
        ProcCommand::new("kill")
            .args(["-0", &pid.to_string()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
}

pub fn daemon_status(socket: &str, pid_path: &Path) -> Result<(), VisioFlowError> {
    if pid_path.exists() {
        let pid = read_pid(pid_path)?;
        if is_process_alive(pid) {
            println!("daemon running (pid {pid}, socket {socket})");
            return Ok(());
        }
        println!("daemon not running (stale pid file)");
        return Err(VisioFlowError::Ipc("daemon not running".into()));
    }
    println!("daemon not running");
    Err(VisioFlowError::Ipc("daemon not running".into()))
}

pub fn daemon_stop(pid_path: &Path) -> Result<(), VisioFlowError> {
    if !pid_path.exists() {
        return Err(VisioFlowError::Ipc("daemon is not running".into()));
    }
    let pid = read_pid(pid_path)?;
    if !is_process_alive(pid) {
        remove_pid(pid_path)?;
        return Err(VisioFlowError::Ipc("daemon is not running".into()));
    }

    #[cfg(windows)]
    {
        Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/F"])
            .status()
            .map_err(VisioFlowError::Io)?;
    }
    #[cfg(not(windows))]
    {
        Command::new("kill")
            .arg(pid.to_string())
            .status()
            .map_err(VisioFlowError::Io)?;
    }

    for _ in 0..20 {
        if !is_process_alive(pid) {
            remove_pid(pid_path)?;
            return Ok(());
        }
        thread::sleep(Duration::from_millis(100));
    }
    Err(VisioFlowError::Ipc(format!(
        "daemon pid {pid} did not exit in time"
    )))
}

pub fn daemon_reload_ipc(socket: &str) -> Result<(), VisioFlowError> {
    ipc_ping_or_message(socket, ClientMessage::Reload { id: 1 })?;
    Ok(())
}

pub fn ipc_ping(socket: &str) -> Result<(), VisioFlowError> {
    match ipc_ping_or_message(socket, ClientMessage::Ping { id: 0 })? {
        ServerMessage::Pong { .. } => Ok(()),
        other => Err(VisioFlowError::Ipc(format!(
            "unexpected ping response: {other:?}"
        ))),
    }
}

fn ipc_ping_or_message(
    socket: &str,
    request: ClientMessage,
) -> Result<ServerMessage, VisioFlowError> {
    let mut client = SocketIpcClient::connect(socket)?;
    client.send_request(request)?;
    client.recv_response()
}

pub fn run_daemon_server_loop(
    socket: &str,
    store: FileRuleStore,
    pid_path: &Path,
    verbose: bool,
) -> Result<(), VisioFlowError> {
    // Validate socket name early.
    let _ = parse_socket_name(socket)?;

    let handler = DaemonHandler::new(store).map_err(|e| VisioFlowError::Ipc(e.to_string()))?;
    let handler = Arc::new(Mutex::new(handler));
    let handler_for_server = Arc::clone(&handler);

    let mut server = SocketIpcServer::bind(socket, move |msg| {
        let mut guard = match handler_for_server.lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };
        guard.handle(msg, verbose)
    })?;

    write_pid(pid_path, std::process::id())?;
    if verbose {
        eprintln!("daemon listening on {socket} (pid {})", std::process::id());
    }

    loop {
        if server.accept().is_err() {
            break;
        }
        if server.handle_one_message().is_err() {
            continue;
        }
    }

    let _ = remove_pid(pid_path);
    Ok(())
}

pub fn daemon_start(
    socket: Option<&str>,
    hidden: bool,
    store: Option<PathBuf>,
    verbose: bool,
) -> Result<(), VisioFlowError> {
    let socket_path = socket
        .map(str::to_owned)
        .unwrap_or_else(default_socket_path);
    let pid_path = default_pid_path();

    if pid_path.exists() {
        if let Ok(pid) = read_pid(&pid_path) {
            if is_process_alive(pid) {
                return Err(VisioFlowError::Ipc(format!(
                    "daemon already running (pid {pid})"
                )));
            }
        }
        remove_pid(&pid_path)?;
    }

    let file_store = match store {
        Some(path) => FileRuleStore::new(path),
        None => FileRuleStore::with_default_path(),
    };

    if hidden {
        spawn_detached_daemon(&socket_path, file_store.path().to_path_buf(), verbose)?;
        thread::sleep(Duration::from_millis(300));
        if pid_path.exists() {
            println!("daemon started on {socket_path}");
            Ok(())
        } else {
            Err(VisioFlowError::Ipc(
                "daemon failed to start (no pid file)".into(),
            ))
        }
    } else {
        run_daemon_server_loop(&socket_path, file_store, &pid_path, verbose)
    }
}

fn spawn_detached_daemon(
    socket: &str,
    store: PathBuf,
    verbose: bool,
) -> Result<(), VisioFlowError> {
    let exe = std::env::current_exe().map_err(VisioFlowError::Io)?;
    let mut cmd = Command::new(exe);
    cmd.arg("daemon")
        .arg("start")
        .arg("--socket")
        .arg(socket)
        .arg("--store")
        .arg(store);
    if verbose {
        cmd.arg("--verbose");
    }

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        const DETACHED_PROCESS: u32 = 0x0000_0008;
        cmd.creation_flags(CREATE_NO_WINDOW | DETACHED_PROCESS);
    }

    cmd.stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(VisioFlowError::Io)?;
    Ok(())
}

/// Execute a rule via the daemon IPC socket (exec action runs in the daemon).
pub fn rule_execute_via_ipc(
    socket: &str,
    name: &str,
    payload: &str,
) -> Result<visioflow_core::ResolvedVars, VisioFlowError> {
    let mut client = SocketIpcClient::connect(socket)?;
    client.send_request(ClientMessage::ExecuteRule {
        id: 1,
        name: name.to_owned(),
        payload: payload.to_owned(),
    })?;
    match client.recv_response()? {
        ServerMessage::RuleResult { vars, .. } => {
            let mut resolved = visioflow_core::ResolvedVars::new();
            for (key, value) in vars {
                resolved.insert(key, value);
            }
            Ok(resolved)
        }
        ServerMessage::Error { message, .. } => Err(VisioFlowError::Capture(message)),
        other => Err(VisioFlowError::Ipc(format!(
            "unexpected execute response: {other:?}"
        ))),
    }
}

/// Route the first captured payload through the daemon (exec runs server-side).
pub fn route_capture_trigger_via_ipc(
    socket: &str,
    rule_name: &str,
    payloads: &[String],
) -> Result<visioflow_core::ResolvedVars, VisioFlowError> {
    let payload = payloads.first().ok_or_else(|| {
        VisioFlowError::Capture("no payloads decoded for trigger".into())
    })?;
    rule_execute_via_ipc(socket, rule_name, payload)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn read_write_pid_roundtrip() {
        let dir = TempDir::new().expect("tempdir");
        let path = dir.path().join("daemon.pid");
        write_pid(&path, 12345).expect("write");
        assert_eq!(read_pid(&path).expect("read"), 12345);
    }

    #[test]
    fn is_process_alive_current_pid() {
        assert!(is_process_alive(std::process::id()));
    }

    #[test]
    fn is_process_alive_dead_pid() {
        assert!(!is_process_alive(4_000_000));
    }
}
