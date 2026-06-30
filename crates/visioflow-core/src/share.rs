//! Resolve bundled share assets (action scripts) at runtime.

use std::env;
use std::path::PathBuf;

fn action_script_extension() -> &'static str {
    if cfg!(windows) {
        ".ps1"
    } else {
        ".sh"
    }
}

/// Candidate share roots: exe directory, `VISIOFLOW_SHARE`, then repo `share/` for dev.
fn share_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();

    if let Ok(exe) = env::current_exe() {
        if let Some(dir) = exe.parent() {
            roots.push(dir.to_path_buf());
        }
    }

    if let Ok(share) = env::var("VISIOFLOW_SHARE") {
        roots.push(PathBuf::from(share));
    }

    roots.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../share"));

    roots
}

fn normalize_relative(relative: &str) -> &str {
    relative.strip_prefix("share/").unwrap_or(relative)
}

fn with_script_extension(relative: &str) -> String {
    let rel = normalize_relative(relative);
    if rel.ends_with(".ps1") || rel.ends_with(".sh") {
        rel.to_owned()
    } else {
        format!("{rel}{}", action_script_extension())
    }
}

/// Resolve a share-relative path like `share/actions/open-url` to an absolute script path.
///
/// Tries each share root in order and returns the first existing file. If none exist,
/// returns the path under the first root (exe dir when available).
#[must_use]
pub fn resolve_share_path(relative: &str) -> PathBuf {
    let rel_with_ext = with_script_extension(relative);
    let roots = share_roots();

    for root in &roots {
        let candidate = root.join(&rel_with_ext);
        if candidate.is_file() {
            return candidate;
        }
    }

    roots
        .first()
        .map(|root| root.join(&rel_with_ext))
        .unwrap_or_else(|| PathBuf::from(&rel_with_ext))
}

/// Workspace-relative path to the shipped default rules asset (dev builds).
#[must_use]
pub fn default_rules_asset_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets/default-rules.json")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn resolve_share_path_uses_visioflow_share_env() {
        let dir = TempDir::new().expect("tempdir");
        let script = dir.path().join("actions/open-url.ps1");
        fs::create_dir_all(script.parent().expect("parent")).expect("mkdir");
        fs::write(&script, "Write-Host test").expect("write");

        let previous = env::var("VISIOFLOW_SHARE").ok();
        // SAFETY: test-only; serialized by harness.
        unsafe { env::set_var("VISIOFLOW_SHARE", dir.path()) };

        let resolved = resolve_share_path("share/actions/open-url");
        assert_eq!(resolved, script);

        match previous {
            Some(value) => unsafe { env::set_var("VISIOFLOW_SHARE", value) },
            None => unsafe { env::remove_var("VISIOFLOW_SHARE") },
        }
    }

    #[test]
    fn resolve_share_path_adds_platform_extension() {
        let path = resolve_share_path("actions/open-url");
        let suffix = if cfg!(windows) {
            "actions/open-url.ps1"
        } else {
            "actions/open-url.sh"
        };
        assert!(
            path.to_string_lossy().replace('\\', "/").ends_with(suffix),
            "expected suffix {suffix}, got {}",
            path.display()
        );
    }

    #[test]
    fn resolve_share_path_finds_repo_share_scripts() {
        let resolved = resolve_share_path("share/actions/open-url");
        assert!(
            resolved.is_file(),
            "expected repo share script at {}",
            resolved.display()
        );
    }

    #[test]
    fn default_rules_asset_path_points_at_workspace_asset() {
        let path = default_rules_asset_path();
        assert!(
            path.is_file(),
            "expected default rules asset at {}",
            path.display()
        );
    }

    #[test]
    fn normalize_relative_strips_share_prefix() {
        assert_eq!(normalize_relative("share/actions/foo"), "actions/foo");
        assert_eq!(normalize_relative("actions/foo"), "actions/foo");
    }
}
