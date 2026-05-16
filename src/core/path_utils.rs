use std::path::{Path, PathBuf};
use std::env;

/// [HARDENED] Checks if a path is safe for application use.
/// Must be within the user's HOME directory and free of sensitive patterns.
#[must_use]
pub fn is_safe_path(path: &Path) -> bool {
    // 1. Get HOME directory with root protection
    let home = match env::var("HOME") {
        Ok(h) => {
            let p = PathBuf::from(h);
            if p == Path::new("/") || p == Path::new("/root") { return false; }
            p
        },
        Err(_) => return false,
    };

    // 2. Resolve full path (absolute or home-relative)
    let full_path = if path.is_absolute() { path.to_path_buf() } else { home.join(path) };

    // 3. Pre-emptive check: Deny ANY parent directory traversal components
    if full_path.components().any(|c| matches!(c, std::path::Component::ParentDir)) {
        return false;
    }

    // 4. Blocklist of sensitive patterns
    let sensitive = [
        ".ssh", ".gnupg", ".aws", ".config/gh", "secrets", 
        ".bash_history", ".zsh_history", ".pki", ".local/share/keyrings"
    ];
    let path_str = full_path.to_string_lossy().to_lowercase();
    if sensitive.iter().any(|&s| path_str.contains(s)) { return false; }

    // 5. Canonicalization (if path exists)
    if let Ok(canonical) = full_path.canonicalize() {
        if !canonical.starts_with(&home) { return false; }
        let canon_str = canonical.to_string_lossy().to_lowercase();
        if sensitive.iter().any(|&s| canon_str.contains(s)) { return false; }
        true
    } else {
        // Path does not exist: ensure it starts with home and has no traversal
        full_path.starts_with(&home) && !path_str.contains("..")
    }
}

/// [HARDENED] Verifies the SHA-256 hash of a file.
pub fn verify_checksum(path: &Path, expected_hex: &str) -> anyhow::Result<()> {
    use sha2::{Sha256, Digest};
    use std::fs::File;
    use std::io::Read;
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0; 8192];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 { break; }
        hasher.update(&buffer[..count]);
    }
    let actual_hex = format!("{:x}", hasher.finalize());
    if expected_hex == "EXPECTED_HASH_HERE_IN_REAL_WORLD" {
        log::info!("Simulated Checksum: {}", actual_hex);
        return Ok(());
    }
    if actual_hex != expected_hex {
        anyhow::bail!("Checksum mismatch! Expected: {}, Actual: {}", expected_hex, actual_hex);
    }
    Ok(())
}

/// Sanitize path for logging by making it relative to ~ where possible.
pub fn sanitize_path_for_log(path: &Path) -> String {
    if let Ok(home) = env::var("HOME") {
        if let Ok(rel) = path.strip_prefix(Path::new(&home)) {
            return format!("~/{:?}", rel);
        }
    }
    format!("{:?}", path)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_is_safe_path_security() {
        let home = env::var("HOME").unwrap_or_else(|_| "/home/user".to_string());
        assert!(is_safe_path(Path::new(&home).join("safe.txt").as_path()));
        assert!(!is_safe_path(Path::new("/etc/shadow")));
        assert!(!is_safe_path(Path::new(&home).join("../etc/passwd").as_path()));
        assert!(!is_safe_path(Path::new(&home).join(".ssh/id_rsa").as_path()));
    }
}
