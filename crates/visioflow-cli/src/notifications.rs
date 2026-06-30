#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeNotification {
    pub title: String,
    pub body: String,
    /// Full text copied when the toast Copy button is activated (may differ from truncated body).
    pub copy_payload: Option<String>,
    /// When true, routing already placed the payload on the clipboard; toast shows "Copy again".
    pub already_copied: bool,
}

/// Label shown on the toast Copy action button.
pub const TOAST_COPY_ACTION_LABEL: &str = "Copy";

/// Label when the payload was already copied during routing.
pub const TOAST_COPY_AGAIN_ACTION_LABEL: &str = "Copy again";

/// Copy button label for a routing toast (Copy vs Copy again).
#[must_use]
pub fn toast_copy_action_label(already_copied: bool) -> &'static str {
    if already_copied {
        TOAST_COPY_AGAIN_ACTION_LABEL
    } else {
        TOAST_COPY_ACTION_LABEL
    }
}

/// Filename prefix for temp files staging toast copy payloads.
const TOAST_COPY_STAGING_PREFIX: &str = "visioflow-toast-copy-";

/// Windows App User Model ID used for toast identity (must match Start Menu shortcut).
pub const TOAST_APP_ID: &str = "VisioFlow.VisioFlowQR";

/// Stub COM CLSID on the Start Menu shortcut (required for unpackaged toast activation).
pub const TOAST_ACTIVATOR_CLSID: &str = "{A7F3C2E1-5B4D-4A9E-8F6C-1D2E3F4A5B6C}";

/// Custom URL scheme for toast Copy activation (unpackaged apps require protocol activation).
pub const TOAST_PROTOCOL_SCHEME: &str = "visioflow";

const TOAST_SHORTCUT_NAME: &str = "VisioFlow.lnk";

/// Hint printed when toast APIs succeed but Windows may suppress the popup.
pub const TOAST_SUPPRESSION_HINT: &str = "if no toast appears, open Windows Settings > System > Notifications and enable notifications for VisioFlow (also check Focus Assist)";

/// Start Menu folder for the toast registration shortcut.
pub fn toast_shortcut_dir() -> std::result::Result<std::path::PathBuf, String> {
    std::env::var("APPDATA")
        .map(|appdata| {
            std::path::PathBuf::from(format!(
                "{appdata}\\Microsoft\\Windows\\Start Menu\\Programs\\VisioFlow"
            ))
        })
        .map_err(|e| e.to_string())
}

/// Full path to the Start Menu shortcut that registers the toast AppUserModelID.
pub fn toast_shortcut_path() -> std::result::Result<std::path::PathBuf, String> {
    toast_shortcut_dir().map(|dir| dir.join(TOAST_SHORTCUT_NAME))
}

/// Returns true when an existing shortcut targets a different executable.
pub fn shortcut_target_stale(stored_target: &str, current_exe: &std::path::Path) -> bool {
    let stored = std::path::Path::new(stored_target);
    match (stored.canonicalize(), current_exe.canonicalize()) {
        (Ok(a), Ok(b)) => a != b,
        _ => stored != current_exe,
    }
}

pub fn escape_xml(raw: &str) -> String {
    raw.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// Maximum characters shown in a toast body before truncation with an ellipsis.
pub const TOAST_BODY_MAX_CHARS: usize = 256;

/// Truncate `text` for toast display, appending `…` when longer than `max_chars`.
#[must_use]
pub fn truncate_for_toast(text: &str, max_chars: usize) -> String {
    if max_chars == 0 {
        return String::new();
    }
    if text.chars().count() <= max_chars {
        return text.to_owned();
    }
    let truncated: String = text.chars().take(max_chars).collect();
    format!("{truncated}…")
}

/// Protocol URI Windows launches when the toast Copy button is clicked.
#[must_use]
pub fn toast_copy_protocol_uri(staging_path: &std::path::Path) -> String {
    format!(
        "{TOAST_PROTOCOL_SCHEME}:notify-copy?path={}",
        percent_encode(staging_path.as_os_str())
    )
}

/// Parse a `visioflow:notify-copy?path=...` activation argument into a staging file path.
#[must_use]
pub fn parse_toast_protocol_activation(arg: &str) -> Option<std::path::PathBuf> {
    let scheme_prefix = format!("{TOAST_PROTOCOL_SCHEME}:");
    let rest = arg.strip_prefix(&scheme_prefix)?;
    let query = rest.strip_prefix("notify-copy?")?;
    for pair in query.split('&') {
        let (key, value) = pair.split_once('=')?;
        if key == "path" {
            let decoded = percent_decode(value).ok()?;
            return Some(std::path::PathBuf::from(decoded));
        }
    }
    None
}

/// If argv is a toast protocol activation, copy the staged payload and return `Some`.
pub fn try_dispatch_toast_protocol_activation() -> Option<std::result::Result<(), String>> {
    let mut args = std::env::args();
    args.next();
    let protocol = args.next()?;
    if args.next().is_some() {
        return None;
    }
    let path = parse_toast_protocol_activation(&protocol)?;
    Some(copy_payload_from_toast_staging(&path))
}

fn percent_encode(raw: &std::ffi::OsStr) -> String {
    let text = raw.to_string_lossy();
    let mut out = String::with_capacity(text.len());
    for byte in text.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b'/' | b':' | b'\\'
            => out.push(byte as char),
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

fn percent_decode(input: &str) -> std::result::Result<String, String> {
    let mut out = Vec::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hex = &input[i + 1..i + 3];
            let byte = u8::from_str_radix(hex, 16).map_err(|e| e.to_string())?;
            out.push(byte);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).map_err(|e| e.to_string())
}

/// Windows GUI-subsystem helper next to `visioflow.exe` for toast Copy activation.
pub const TOAST_ACTIVATOR_EXE_NAME: &str = "visioflow-toast.exe";

/// Path to the headless toast activator shipped beside the main CLI binary.
#[must_use]
pub fn toast_activator_exe_path(main_exe: &std::path::Path) -> std::path::PathBuf {
    main_exe
        .parent()
        .map(|parent| parent.join(TOAST_ACTIVATOR_EXE_NAME))
        .unwrap_or_else(|| std::path::PathBuf::from(TOAST_ACTIVATOR_EXE_NAME))
}

/// CLI arguments Windows passes when the toast Copy button is clicked (foreground activation).
#[must_use]
pub fn toast_copy_activation_args(staging_path: &std::path::Path) -> String {
    toast_copy_protocol_uri(staging_path)
}

#[cfg(windows)]
fn ensure_toast_activator_binary(main_exe: &std::path::Path) -> std::result::Result<(), String> {
    let activator = toast_activator_exe_path(main_exe);
    if activator.is_file() {
        Ok(())
    } else {
        Err(format!(
            "missing toast activator {} — rebuild with: cargo build -p visioflow-cli",
            activator.display()
        ))
    }
}

/// Write `payload` to a temp file for toast Copy activation; returns the file path.
pub fn stage_toast_copy_payload(payload: &str) -> std::result::Result<std::path::PathBuf, String> {
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let id = COUNTER.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    let name = format!("{TOAST_COPY_STAGING_PREFIX}{pid}-{id}.txt");
    let path = std::env::temp_dir().join(name);
    std::fs::write(&path, payload).map_err(|e| e.to_string())?;
    Ok(path)
}

/// Returns true when `path` is a VisioFlow toast copy staging file under the temp directory.
pub fn is_toast_copy_staging_path(path: &std::path::Path) -> bool {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return false;
    };
    if !name.starts_with(TOAST_COPY_STAGING_PREFIX) || !name.ends_with(".txt") {
        return false;
    }
    let Some(parent) = path.parent() else {
        return false;
    };
    same_temp_directory(parent, &std::env::temp_dir())
}

#[cfg(windows)]
fn same_temp_directory(left: &std::path::Path, right: &std::path::Path) -> bool {
    fn normalize(path: &std::path::Path) -> std::path::PathBuf {
        path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
    }
    let left_norm = normalize(left);
    let right_norm = normalize(right);
    left_norm
        .to_string_lossy()
        .eq_ignore_ascii_case(&right_norm.to_string_lossy())
}

#[cfg(not(windows))]
fn same_temp_directory(left: &std::path::Path, right: &std::path::Path) -> bool {
    left == right
}

/// Read a staged toast copy file, place its contents on the clipboard, then delete the file.
pub fn copy_payload_from_toast_staging(path: &std::path::Path) -> std::result::Result<(), String> {
    if !is_toast_copy_staging_path(path) {
        return Err("refusing to read non-staging toast copy path".to_owned());
    }
    let payload = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    copy_text_to_clipboard(&payload)?;
    let _ = std::fs::remove_file(path);
    Ok(())
}

pub fn copy_text_to_clipboard(text: &str) -> std::result::Result<(), String> {
    let mut clipboard =
        arboard::Clipboard::new().map_err(|e| format!("clipboard unavailable: {e}"))?;
    clipboard
        .set_text(text.to_owned())
        .map_err(|e| format!("clipboard write failed: {e}"))
}

pub fn toast_xml(note: &NativeNotification, copy_staging: Option<&std::path::Path>) -> String {
    let visual = format!(
        "<visual><binding template=\"ToastGeneric\"><text>{}</text><text>{}</text></binding></visual>",
        escape_xml(&note.title),
        escape_xml(&note.body)
    );
    let actions = copy_staging.map(|path| {
        format!(
            "<actions><action content=\"{}\" arguments=\"{}\" activationType=\"protocol\"/></actions>",
            escape_xml(toast_copy_action_label(note.already_copied)),
            escape_xml(&toast_copy_protocol_uri(path))
        )
    });
    match actions {
        Some(actions) => format!("<toast>{visual}{actions}</toast>"),
        None => format!("<toast>{visual}</toast>"),
    }
}

type NotificationSender = fn(&NativeNotification) -> std::result::Result<(), String>;

pub fn send_with_backends(
    note: &NativeNotification,
    backends: &[NotificationSender],
) -> std::result::Result<(), String> {
    let mut errors = Vec::new();
    for backend in backends {
        match backend(note) {
            Ok(()) => return Ok(()),
            Err(err) => errors.push(err),
        }
    }
    Err(errors.join("; "))
}

type LabeledSender = (&'static str, NotificationSender);

fn send_with_backends_labeled(
    note: &NativeNotification,
    backends: &[LabeledSender],
) -> std::result::Result<&'static str, String> {
    let mut errors = Vec::new();
    for (label, backend) in backends {
        match backend(note) {
            Ok(()) => return Ok(label),
            Err(err) => errors.push(format!("{label}: {err}")),
        }
    }
    Err(errors.join("; "))
}

/// Run the toast smoke path and return the backend channel name on success.
pub fn run_notify_test(note: &NativeNotification) -> std::result::Result<&'static str, String> {
    send_with_backends_labeled(note, default_labeled_backends())
}

/// Run a single toast backend (for smoke tests and diagnostics).
pub fn run_notify_test_backend(
    note: &NativeNotification,
    backend: &str,
) -> std::result::Result<&'static str, String> {
    let backends = default_labeled_backends();
    let (label, sender) = backends
        .iter()
        .find(|(name, _)| *name == backend)
        .ok_or_else(|| {
            let available = backends
                .iter()
                .map(|(name, _)| *name)
                .collect::<Vec<_>>()
                .join(", ");
            format!("unknown toast backend {backend:?}; available: {available}")
        })?;
    sender(note).map(|()| *label)
}

pub fn send_native_notification(note: &NativeNotification) -> std::result::Result<(), String> {
    send_with_backends(note, default_backends())
}

#[cfg(windows)]
fn default_labeled_backends() -> &'static [LabeledSender] {
    &[
        ("winrt", send_native_winrt),
        ("powershell", send_powershell_winrt),
        ("burnttoast", send_powershell_burnt_toast),
    ]
}

#[cfg(not(windows))]
fn default_labeled_backends() -> &'static [LabeledSender] {
    &[("noop", |_| Ok(()))]
}

#[cfg(windows)]
fn default_backends() -> &'static [NotificationSender] {
    &[
        send_native_winrt,
        send_powershell_winrt,
        send_powershell_burnt_toast,
    ]
}

#[cfg(not(windows))]
fn default_backends() -> &'static [NotificationSender] {
    &[|_| Ok(())]
}

#[cfg(windows)]
fn prepare_toast_send(note: &NativeNotification) -> std::result::Result<(String, Option<std::path::PathBuf>), String> {
    let copy_staging = match &note.copy_payload {
        Some(payload) => Some(stage_toast_copy_payload(payload)?),
        None => None,
    };
    let xml = toast_xml(note, copy_staging.as_deref());
    Ok((xml, copy_staging))
}

#[cfg(windows)]
fn send_native_winrt(note: &NativeNotification) -> std::result::Result<(), String> {
    use windows::core::HSTRING;
    use windows::Data::Xml::Dom::XmlDocument;
    use windows::UI::Notifications::{ToastNotification, ToastNotificationManager};
    use windows::Win32::UI::Shell::SetCurrentProcessExplicitAppUserModelID;

    ensure_toast_shortcut()?;

    unsafe {
        SetCurrentProcessExplicitAppUserModelID(&HSTRING::from(TOAST_APP_ID))
            .map_err(|e| e.to_string())?;
    }

    let (xml, _copy_staging) = prepare_toast_send(note)?;
    let doc = XmlDocument::new().map_err(|e| e.to_string())?;
    doc.LoadXml(&HSTRING::from(xml))
        .map_err(|e| e.to_string())?;
    let toast = ToastNotification::CreateToastNotification(&doc).map_err(|e| e.to_string())?;
    let notifier = ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from(TOAST_APP_ID))
        .map_err(|e| e.to_string())?;
    notifier.Show(&toast).map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(windows)]
fn ensure_toast_shortcut() -> std::result::Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let lnk = toast_shortcut_path()?;
    if let Some(parent) = lnk.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    // Always rewrite: WScript-only shortcuts lack System.AppUserModel.ID, which
    // makes WinRT Show() succeed while Windows drops the toast silently.
    install_toast_shortcut(&lnk, &exe, TOAST_APP_ID)?;
    register_toast_activation_support(&exe)
}

#[cfg(windows)]
fn install_toast_shortcut(
    lnk_path: &std::path::Path,
    exe_path: &std::path::Path,
    app_id: &str,
) -> std::result::Result<(), String> {
    use windows::core::{Interface, HSTRING, PROPVARIANT, w};
    use windows::Win32::Storage::EnhancedStorage::{
        PKEY_AppUserModel_ID, PKEY_AppUserModel_ToastActivatorCLSID,
    };
    use windows::Win32::System::Com::{
        CLSCTX_INPROC_SERVER, CoCreateInstance, CoInitializeEx, COINIT_APARTMENTTHREADED,
        IPersistFile, StructuredStorage::InitPropVariantFromStringAsVector,
    };
    use windows::Win32::UI::Shell::PropertiesSystem::IPropertyStore;
    use windows::Win32::UI::Shell::{IShellLinkW, ShellLink};

    let working_dir = exe_path
        .parent()
        .ok_or_else(|| "executable has no parent directory".to_owned())?;

    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);

        let link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER)
            .map_err(|e| e.to_string())?;
        link.SetPath(&HSTRING::from(exe_path.as_os_str()))
            .map_err(|e| e.to_string())?;
        link.SetWorkingDirectory(&HSTRING::from(working_dir.as_os_str()))
            .map_err(|e| e.to_string())?;
        link.SetDescription(&HSTRING::from("VisioFlow QR scanner"))
            .map_err(|e| e.to_string())?;
        link.SetArguments(w!(""))
            .map_err(|e| e.to_string())?;

        let property_store: IPropertyStore = link.cast().map_err(|e| e.to_string())?;
        let app_id_prop: PROPVARIANT =
            InitPropVariantFromStringAsVector(&HSTRING::from(app_id)).map_err(|e| e.to_string())?;
        property_store
            .SetValue(&PKEY_AppUserModel_ID, &app_id_prop)
            .map_err(|e| e.to_string())?;
        let clsid_prop: PROPVARIANT = InitPropVariantFromStringAsVector(&HSTRING::from(
            TOAST_ACTIVATOR_CLSID,
        ))
        .map_err(|e| e.to_string())?;
        property_store
            .SetValue(&PKEY_AppUserModel_ToastActivatorCLSID, &clsid_prop)
            .map_err(|e| e.to_string())?;
        property_store.Commit().map_err(|e| e.to_string())?;

        let persist_file: IPersistFile = link.cast().map_err(|e| e.to_string())?;
        persist_file
            .Save(&HSTRING::from(lnk_path.as_os_str()), true)
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

#[cfg(windows)]
fn register_toast_activation_support(exe_path: &std::path::Path) -> std::result::Result<(), String> {
    ensure_toast_activator_binary(exe_path)?;
    register_toast_protocol_handler(exe_path)?;
    register_toast_aumid_registry()
}

/// Registry `open\command` value for `visioflow:` protocol activation without a console flash.
#[cfg(windows)]
#[must_use]
pub fn toast_protocol_registry_command(exe_path: &std::path::Path) -> String {
    let activator = toast_activator_exe_path(exe_path);
    format!(r#""{}" "%1""#, activator.display())
}

#[cfg(not(windows))]
#[must_use]
pub fn toast_protocol_registry_command(_exe_path: &std::path::Path) -> String {
    String::new()
}

#[cfg(windows)]
fn register_toast_protocol_handler(exe_path: &std::path::Path) -> std::result::Result<(), String> {
    let scheme = TOAST_PROTOCOL_SCHEME;
    let classes = format!(r"Software\Classes\{scheme}");
    let command_key = format!(r"{classes}\shell\open\command");
    let command = toast_protocol_registry_command(exe_path);

    reg_set_hkcu_string(&classes, None, &format!("URL:{scheme} Protocol"))?;
    reg_set_hkcu_string(&classes, Some("URL Protocol"), "")?;
    reg_set_hkcu_string(&command_key, None, &command)?;
    Ok(())
}

#[cfg(windows)]
fn register_toast_aumid_registry() -> std::result::Result<(), String> {
    let key = format!(r"Software\Classes\AppUserModelId\{TOAST_APP_ID}");
    reg_set_hkcu_string(&key, Some("DisplayName"), "VisioFlow")?;
    Ok(())
}

#[cfg(windows)]
fn reg_set_hkcu_string(
    subkey: &str,
    value_name: Option<&str>,
    data: &str,
) -> std::result::Result<(), String> {
    use std::os::windows::process::CommandExt;
    use std::process::Command;

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    let hive = r"HKEY_CURRENT_USER";
    let key = format!(r"{hive}\{subkey}");
    let mut cmd = Command::new("reg");
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd.arg("add").arg(&key).arg("/f");
    match value_name {
        Some(name) => {
            cmd.args(["/v", name, "/t", "REG_SZ", "/d", data]);
        }
        None => {
            cmd.args(["/ve", "/t", "REG_SZ", "/d", data]);
        }
    }
    let output = cmd.output().map_err(|e| format!("reg add failed: {e}"))?;
    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!(
            "reg add {subkey} failed ({}): {}",
            output.status,
            stderr.trim()
        ))
    }
}

#[cfg(windows)]
fn send_powershell_winrt(note: &NativeNotification) -> std::result::Result<(), String> {
    ensure_toast_shortcut()?;
    let (xml, _copy_staging) = prepare_toast_send(note)?;
    let xml_escaped = escape_powershell_single_quoted(&xml);

    let script = format!(
        r#"
$ErrorActionPreference = 'Stop'
$appId = '{app_id}'
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class ToastAppId {{
  [DllImport("shell32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
  public static extern int SetCurrentProcessExplicitAppUserModelID(string AppId);
}}
'@
[void][ToastAppId]::SetCurrentProcessExplicitAppUserModelID($appId)
[Windows.UI.Notifications.ToastNotificationManager, Windows.UI.Notifications, ContentType = WindowsRuntime] | Out-Null
[Windows.Data.Xml.Dom.XmlDocument, Windows.Data.Xml.Dom.XmlDocument, ContentType = WindowsRuntime] | Out-Null
$xml = '{xml_escaped}'
$doc = New-Object Windows.Data.Xml.Dom.XmlDocument
$doc.LoadXml($xml)
$toast = [Windows.UI.Notifications.ToastNotification]::new($doc)
[Windows.UI.Notifications.ToastNotificationManager]::CreateToastNotifier($appId).Show($toast)
"#,
        app_id = TOAST_APP_ID,
        xml_escaped = xml_escaped,
    );

    run_powershell(&script)
}

#[cfg(windows)]
fn send_powershell_burnt_toast(note: &NativeNotification) -> std::result::Result<(), String> {
    ensure_toast_shortcut()?;
    let title = escape_powershell_single_quoted(&note.title);
    let body = escape_powershell_single_quoted(&note.body);
    let copy_staging = match &note.copy_payload {
        Some(payload) => Some(stage_toast_copy_payload(payload)?),
        None => None,
    };
    let action_block = copy_staging
        .as_ref()
        .map(|path| {
            let args = escape_powershell_single_quoted(&toast_copy_protocol_uri(path));
            format!(
                r#"
$copyBtn = New-BTButton -Content '{label}' -Arguments '{args}'
"#,
                label = toast_copy_action_label(note.already_copied),
                args = args,
            )
        })
        .unwrap_or_default();
    let action_param = if copy_staging.is_some() {
        " -Button $copyBtn"
    } else {
        ""
    };
    let script = format!(
        r#"
$ErrorActionPreference = 'Stop'
Import-Module BurntToast -ErrorAction Stop
{action_block}New-BurntToastNotification -AppId '{app_id}' -Text '{title}', '{body}' -Urgent{action_param}
"#,
        action_block = action_block,
        app_id = TOAST_APP_ID,
        title = title,
        body = body,
        action_param = action_param,
    );
    run_powershell(&script)
}

#[cfg(windows)]
fn escape_powershell_single_quoted(raw: &str) -> String {
    raw.replace('\'', "''")
}

#[cfg(windows)]
fn run_powershell(script: &str) -> std::result::Result<(), String> {
    use std::io::Write;
    use std::process::{Command, Stdio};

    let mut child = Command::new("powershell")
        .args([
            "-NoProfile",
            "-STA",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            "-",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("failed to spawn powershell: {e}"))?;

    child
        .stdin
        .as_mut()
        .ok_or_else(|| "powershell stdin unavailable".to_owned())?
        .write_all(script.as_bytes())
        .map_err(|e| e.to_string())?;

    let output = child.wait_with_output().map_err(|e| e.to_string())?;
    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        Err(format!(
            "powershell exited {}: {}{}",
            output.status,
            stderr.trim(),
            if stdout.trim().is_empty() {
                String::new()
            } else {
                format!(" ({})", stdout.trim())
            }
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_note() -> NativeNotification {
        NativeNotification {
            title: "VisioFlow".to_owned(),
            body: "Hello & <world>".to_owned(),
            copy_payload: None,
            already_copied: false,
        }
    }

    #[test]
    fn escape_xml_escapes_special_chars() {
        assert_eq!(
            escape_xml(r#"Tom & Jerry say "hi""#),
            "Tom &amp; Jerry say &quot;hi&quot;"
        );
    }

    #[test]
    fn toast_xml_includes_escaped_title_and_body() {
        let xml = toast_xml(&sample_note(), None);
        assert!(xml.contains("VisioFlow"));
        assert!(xml.contains("Hello &amp; &lt;world&gt;"));
        assert!(xml.contains("ToastGeneric"));
        assert!(!xml.contains("<actions>"));
    }

    #[test]
    fn toast_xml_includes_copy_action_when_staging_path_set() {
        let path = std::env::temp_dir().join("visioflow-toast-copy-test.xml-path.txt");
        let xml = toast_xml(&sample_note(), Some(&path));
        assert!(xml.contains(r#"<action content="Copy""#));
        assert!(xml.contains("activationType=\"protocol\""));
        assert!(xml.contains("visioflow:notify-copy?path="));
        assert!(xml.contains("visioflow-toast-copy-test.xml-path.txt"));
    }

    #[test]
    fn toast_copy_protocol_uri_roundtrips_temp_path() {
        let path = std::env::temp_dir().join("visioflow-toast-copy-roundtrip.txt");
        let uri = toast_copy_protocol_uri(&path);
        assert!(uri.starts_with("visioflow:notify-copy?path="));
        let parsed = parse_toast_protocol_activation(&uri).expect("parsed");
        assert_eq!(parsed, path);
    }

    #[test]
    fn toast_copy_protocol_uri_encodes_spaces_in_path() {
        let path = std::path::Path::new(
            r"C:\Users\me\AppData\Local\Temp\visioflow toast copy\visioflow-toast-copy-1.txt",
        );
        let uri = toast_copy_protocol_uri(path);
        assert!(uri.contains("%20"));
        let parsed = parse_toast_protocol_activation(&uri).expect("parsed");
        assert_eq!(parsed, path);
    }

    #[test]
    fn try_dispatch_toast_protocol_activation_copies_payload() {
        let path = stage_toast_copy_payload("protocol-dispatch-target").expect("stage");
        let uri = toast_copy_protocol_uri(&path);
        let parsed = parse_toast_protocol_activation(&uri).expect("parse");
        assert_eq!(parsed, path);
        copy_payload_from_toast_staging(&parsed).expect("copy");
        assert!(!path.exists());
    }

    #[test]
    fn stage_toast_copy_payload_writes_temp_file() {
        let path = stage_toast_copy_payload("full raw payload").expect("stage");
        assert!(is_toast_copy_staging_path(&path));
        let contents = std::fs::read_to_string(&path).expect("read");
        assert_eq!(contents, "full raw payload");
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn copy_payload_from_toast_staging_rejects_outside_temp() {
        let err = copy_payload_from_toast_staging(std::path::Path::new(r"C:\Windows\System32\cmd.exe"))
            .expect_err("expected rejection");
        assert!(err.contains("non-staging"));
    }

    #[test]
    fn copy_payload_from_toast_staging_copies_and_deletes_file() {
        let path = stage_toast_copy_payload("clipboard-target").expect("stage");
        copy_payload_from_toast_staging(&path).expect("copy");
        assert!(!path.exists());
        let mut clipboard = arboard::Clipboard::new().expect("clipboard");
        let text = clipboard.get_text().expect("clipboard text");
        assert_eq!(text, "clipboard-target");
    }

    #[test]
    fn send_with_backends_uses_first_success() {
        let note = sample_note();
        static CALLS: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);
        fn fail(_: &NativeNotification) -> std::result::Result<(), String> {
            CALLS.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Err("fail".to_owned())
        }
        fn ok(_: &NativeNotification) -> std::result::Result<(), String> {
            CALLS.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        }
        CALLS.store(0, std::sync::atomic::Ordering::SeqCst);
        send_with_backends(&note, &[fail, ok]).expect("expected success");
        assert_eq!(CALLS.load(std::sync::atomic::Ordering::SeqCst), 2);
    }

    #[test]
    fn send_with_backends_collects_errors_when_all_fail() {
        let note = sample_note();
        let err = send_with_backends(&note, &[|_| Err("a".into()), |_| Err("b".into())])
            .expect_err("expected failure");
        assert!(err.contains("a"));
        assert!(err.contains("b"));
    }

    #[test]
    fn shortcut_target_stale_detects_different_exe() {
        assert!(shortcut_target_stale(
            r"D:\app\target\debug\visioflow.exe",
            std::path::Path::new(r"D:\app\target\release\visioflow.exe"),
        ));
    }

    #[test]
    fn shortcut_target_stale_matches_same_path() {
        let path = std::path::Path::new(r"D:\app\target\release\visioflow.exe");
        assert!(!shortcut_target_stale(
            r"D:\app\target\release\visioflow.exe",
            path,
        ));
    }

    #[test]
    fn toast_shortcut_path_ends_with_visioflow_lnk() {
        let path = toast_shortcut_path().expect("shortcut path");
        assert_eq!(path.file_name().and_then(|n| n.to_str()), Some("VisioFlow.lnk"));
    }

    #[test]
    fn run_notify_test_backend_rejects_unknown_channel() {
        let note = sample_note();
        let err = run_notify_test_backend(&note, "not-a-backend").expect_err("expected error");
        assert!(err.contains("unknown toast backend"));
    }

    #[test]
    fn truncate_for_toast_keeps_short_text() {
        assert_eq!(truncate_for_toast("hello", 256), "hello");
    }

    #[test]
    fn truncate_for_toast_adds_ellipsis_for_long_text() {
        let long = "a".repeat(300);
        let out = truncate_for_toast(&long, 256);
        assert!(out.ends_with('…'));
        assert_eq!(out.chars().count(), 257);
        assert!(out.starts_with(&"a".repeat(256)));
    }

    #[test]
    fn run_notify_test_uses_labeled_backends() {
        let note = sample_note();
        let ok = |_: &NativeNotification| Ok(());
        let backends: &[LabeledSender] = &[("mock", ok)];
        let channel = send_with_backends_labeled(&note, backends).expect("channel");
        assert_eq!(channel, "mock");
    }

    #[test]
    fn toast_copy_action_label_defaults_to_copy() {
        assert_eq!(toast_copy_action_label(false), "Copy");
    }

    #[test]
    fn toast_copy_action_label_uses_copy_again_when_already_copied() {
        assert_eq!(toast_copy_action_label(true), "Copy again");
    }

    #[test]
    fn toast_xml_uses_copy_again_when_already_copied() {
        let mut note = sample_note();
        note.copy_payload = Some("payload".to_owned());
        note.already_copied = true;
        let path = std::env::temp_dir().join("visioflow-toast-copy-already.txt");
        let xml = toast_xml(&note, Some(&path));
        assert!(xml.contains(r#"<action content="Copy again""#));
        assert!(!xml.contains(r#"content="Copy" activationType"#));
    }

    #[cfg(windows)]
    #[test]
    fn toast_activator_exe_path_is_sibling_of_main_binary() {
        let main = std::path::Path::new(r"C:\app\target\release\visioflow.exe");
        let activator = toast_activator_exe_path(main);
        assert_eq!(
            activator,
            std::path::Path::new(r"C:\app\target\release\visioflow-toast.exe")
        );
    }

    #[cfg(windows)]
    #[test]
    fn toast_protocol_registry_command_uses_headless_activator_without_powershell() {
        let cmd = toast_protocol_registry_command(std::path::Path::new(
            r"C:\app\target\release\visioflow.exe",
        ));
        assert!(cmd.contains("visioflow-toast.exe"));
        assert!(cmd.contains("%1"));
        assert!(!cmd.contains("powershell"));
        assert!(!cmd.contains("CreateNoWindow"));
    }
}
