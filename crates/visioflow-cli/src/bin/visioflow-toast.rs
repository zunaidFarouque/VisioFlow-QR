//! Headless Windows-subsystem entry point for toast Copy protocol activation.
//! Launched by the `visioflow:` URL handler — no console window.

#![cfg_attr(all(windows, not(test)), windows_subsystem = "windows")]

fn main() {
    match visioflow_cli::notifications::try_dispatch_toast_protocol_activation() {
        Some(Ok(())) => {}
        Some(Err(_)) => std::process::exit(1),
        None => std::process::exit(1),
    }
}
