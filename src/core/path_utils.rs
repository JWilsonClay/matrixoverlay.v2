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

    // **[HARDENING: Pre-emptive Blocklist Check]**
    // Ensure sensitive patterns are blocked regardless of file existence.
    let sensitive_patterns = [
        ".ssh", ".gnupg", ".aws", ".config/gh", "secrets", 
        ".bash_history", ".zsh_history", ".pki", ".local/share/keyrings"
    ];
    let path_str = full_path.to_string_lossy();
    for pattern in &sensitive_patterns {
        if path_str.contains(pattern) {
            return false;
        }
    }

    // Try canonicalization if it exists
    if let Ok(canonical) = full_path.canonicalize() {
        // Must start with home
        if !canonical.starts_with(&home) {
            return false;
        }

        // **[HARDENING: Post-Canonical Blocklist Check]**
        // Re-check after resolution to catch symlink escapes.
        let canonical_str = canonical.to_string_lossy();
        for pattern in &sensitive_patterns {
            if canonical_str.contains(pattern) {
                return false;
            }
        }
        
        true
    } else {
        // If file doesn't exist, we permit it for now if it's within home
        // and doesn't contain parent directory traversal.
        // **[HARDENING: Parent Traversal Double-Check]**
        !path_str.contains("..") && full_path.starts_with(&home)
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_is_safe_path_happy() {
        let home = env::var("HOME").unwrap_or_else(|_| "/home/user".to_string());
        let safe_path = Path::new(&home).join("documents/safe.txt");
        // We can't easily test existence-based canonicalization without real files,
        // but we can test the basic boundary check.
        assert!(is_safe_path(&safe_path));
    }

    #[test]
    fn test_is_safe_path_traversal() {
        let home = env::var("HOME").unwrap_or_else(|_| "/home/user".to_string());
        let unsafe_path = Path::new(&home).join("documents/../.ssh/id_rsa");
        assert!(!is_safe_path(&unsafe_path));
    }

    #[test]
    fn test_is_safe_path_blocklist() {
        let home = env::var("HOME").unwrap_or_else(|_| "/home/user".to_string());
        let ssh_path = Path::new(&home).join(".ssh/config");
        let history_path = Path::new(&home).join(".bash_history");
        
        // Mocking behavior by ensuring strings are blocked
        assert!(!is_safe_path(&ssh_path));
        assert!(!is_safe_path(&history_path));
    }

    #[test]
    fn test_is_safe_path_outside_home() {
        let unsafe_path = Path::new("/etc/shadow");
        assert!(!is_safe_path(&unsafe_path));
    }

    #[test]
    fn test_is_safe_path_nonexistent_traversal() {
        let home = env::var("HOME").unwrap_or_else(|_| "/home/user".to_string());
        // String-based traversal check for non-existent paths
        let fake_path = Path::new(&home).join("nonexistent/../../etc/passwd");
        assert!(!is_safe_path(&fake_path));
    }
}
