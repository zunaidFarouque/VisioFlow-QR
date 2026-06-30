//! Platform socket path conventions for VisioFlow IPC.

use interprocess::local_socket::{GenericNamespaced, Name, ToFsName, ToNsName};

use crate::error::{Result, VisioFlowError};

/// Default socket identifier (namespaced on all platforms).
pub const DEFAULT_SOCKET_NAME: &str = "visioflow.sock";

/// Default display path for operators (`--ipc-socket` when omitted).
#[must_use]
pub fn default_socket_path() -> String {
    #[cfg(windows)]
    {
        format!(r"\\.\pipe\{DEFAULT_SOCKET_NAME}")
    }
    #[cfg(not(windows))]
    {
        format!("/tmp/{DEFAULT_SOCKET_NAME}")
    }
}

/// Convert a user-supplied socket path into an [`interprocess`] local socket name.
pub fn parse_socket_name(path: &str) -> Result<Name<'static>> {
    if path.is_empty() {
        return Err(VisioFlowError::Ipc("socket path must not be empty".into()));
    }

    #[cfg(windows)]
    {
        if path.starts_with(r"\\.\pipe\") {
            return path
                .to_fs_name::<interprocess::os::windows::local_socket::NamedPipe>()
                .map(|n| n.into_owned())
                .map_err(|e| VisioFlowError::Ipc(format!("invalid pipe path: {e}")));
        }
        path
            .to_ns_name::<GenericNamespaced>()
            .map(|n| n.into_owned())
            .map_err(|e| VisioFlowError::Ipc(format!("invalid socket name: {e}")))
    }

    #[cfg(not(windows))]
    {
        if path.starts_with('/') {
            return path
                .to_fs_name::<interprocess::os::unix::local_socket::FilesystemUdSocket>()
                .map(|n| n.into_owned())
                .map_err(|e| VisioFlowError::Ipc(format!("invalid socket path: {e}")));
        }
        path.to_ns_name::<GenericNamespaced>()
            .map(|n| n.into_owned())
            .map_err(|e| VisioFlowError::Ipc(format!("invalid socket name: {e}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_socket_path_is_platform_specific() {
        let path = default_socket_path();
        #[cfg(windows)]
        assert!(path.starts_with(r"\\.\pipe\"));
        #[cfg(not(windows))]
        assert!(path.starts_with("/tmp/"));
    }

    #[test]
    fn parse_default_name_succeeds() {
        let name = parse_socket_name(DEFAULT_SOCKET_NAME).expect("parse");
        assert!(name.is_namespaced());
    }

    #[test]
    fn parse_full_default_path_succeeds() {
        let path = default_socket_path();
        parse_socket_name(&path).expect("parse full path");
    }
}
