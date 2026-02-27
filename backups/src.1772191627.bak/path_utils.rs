use std::path::{Path, PathBuf};
use std::env;

/// Checks if a path is safe to read.
/// Rules:
/// 1. Must be within the user's HOME directory.
/// 2. Must not contain ".." after canonicalization.
/// 3. Must not be a sensitive directory (e.g., .ssh, .gnupg).
pub fn is_safe_path(path: &Path) -> bool {
    // 1. Get HOME directory
    let home = match env::var("HOME") {
        Ok(h) => PathBuf::from(h),
        Err(_) => return false,
    };

    // 2. Canonicalize path to resolve ".." and symlinks
    // Note: canonicalize() requires the path to exist. For non-existent paths,
    // we do a basic check for ".." components.
    let full_path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        home.join(path)
    };

    // Basic sanity check for ".." before canonicalization (pre-emptive)
    if full_path.components().any(|c| matches!(c, std::path::Component::ParentDir)) {
        return false;
    }

    // Try canonicalization if it exists
    if let Ok(canonical) = full_path.canonicalize() {
        // Must start with home
        if !canonical.starts_with(&home) {
            return false;
        }

        // Check for sensitive sub-directories
        let sensitive_patterns = [".ssh", ".gnupg", ".aws", ".config/gh", "secrets"];
        for pattern in &sensitive_patterns {
            if canonical.to_string_lossy().contains(pattern) {
                return false;
            }
        }
        
        true
    } else {
        // If file doesn't exist, we permit it for now if it's within home
        // (e.g. for checking existence later)
        full_path.starts_with(&home)
    }
}

/// Sanitize path for logging (make relative to HOME if possible)
pub fn sanitize_path_for_log(path: &Path) -> String {
    if let Ok(home) = env::var("HOME") {
        let home_path = Path::new(&home);
        if let Ok(rel) = path.strip_prefix(home_path) {
            return format!("~/{:?}", rel);
        }
    }
    format!("{:?}", path)
}
