use visioflow_core::error::{Result, VisioFlowError};

use crate::notifications::{
    copy_payload_from_toast_staging, run_notify_test, run_notify_test_backend, NativeNotification,
    TOAST_SUPPRESSION_HINT,
};

pub fn notify_test(
    title: Option<&str>,
    body: Option<&str>,
    backend: Option<&str>,
    verbose: bool,
) -> Result<()> {
    let body_text = body
        .map(str::to_owned)
        .unwrap_or_else(|| "Toast delivery smoke test".to_owned());
    let note = NativeNotification {
        title: title
            .map(str::to_owned)
            .unwrap_or_else(|| "VisioFlow".to_owned()),
        body: body_text.clone(),
        copy_payload: Some(body_text.clone()),
        already_copied: false,
    };

    let result = if let Some(backend) = backend {
        run_notify_test_backend(&note, backend)
    } else {
        run_notify_test(&note)
    };

    match result {
        Ok(channel) => {
            if verbose {
                eprintln!("visioflow: toast sent via {channel}");
                eprintln!("visioflow: {TOAST_SUPPRESSION_HINT}");
            }
            println!("toast sent ({channel}): {}", note.title);
            Ok(())
        }
        Err(err) => Err(VisioFlowError::Capture(format!(
            "toast delivery failed: {err}"
        ))),
    }
}

/// Handle foreground toast activation: copy staged payload to clipboard.
pub fn notify_copy_from_toast(from_toast: &std::path::Path, silent: bool) -> Result<()> {
    copy_payload_from_toast_staging(from_toast).map_err(|err| {
        VisioFlowError::Capture(format!("toast copy activation failed: {err}"))
    })?;
    if !silent {
        println!("copied toast payload to clipboard");
    }
    Ok(())
}
