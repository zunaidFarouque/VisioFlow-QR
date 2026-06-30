#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeNotification {
    pub title: String,
    pub body: String,
}

#[cfg(windows)]
pub fn send_native_notification(note: &NativeNotification) -> std::result::Result<(), String> {
    use windows::core::HSTRING;
    use windows::Data::Xml::Dom::XmlDocument;
    use windows::UI::Notifications::{ToastNotification, ToastNotificationManager};

    fn escape_xml(raw: &str) -> String {
        raw.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&apos;")
    }

    let xml = format!(
        "<toast><visual><binding template=\"ToastGeneric\"><text>{}</text><text>{}</text></binding></visual></toast>",
        escape_xml(&note.title),
        escape_xml(&note.body)
    );

    let doc = XmlDocument::new().map_err(|e| e.to_string())?;
    doc.LoadXml(&HSTRING::from(xml))
        .map_err(|e| e.to_string())?;
    let toast = ToastNotification::CreateToastNotification(&doc).map_err(|e| e.to_string())?;
    let notifier =
        ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from("VisioFlow-QR"))
            .map_err(|e| e.to_string())?;
    notifier.Show(&toast).map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(not(windows))]
pub fn send_native_notification(_note: &NativeNotification) -> std::result::Result<(), String> {
    Ok(())
}
