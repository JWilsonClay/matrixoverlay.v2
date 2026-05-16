// src/core/productivity/ai.rs
use anyhow::{Result, Context, bail};
use git2::Repository;

/// [HARDENED] Generates a commit message via Ollama with prompt injection protection.
pub fn generate_ai_commit_message(repo: &Repository) -> Result<String> {
    let diff = repo.diff_index_to_workdir(None, None)?;
    let mut diff_text = Vec::new();
    diff.print(git2::DiffFormat::Patch, |_, _, line| {
        diff_text.extend_from_slice(line.content());
        true
    })?;

    let diff_str = String::from_utf8_lossy(&diff_text);
    // [HARDENING] Strict truncation and basic sanitization to prevent prompt injection
    let truncated = if diff_str.len() > 3000 { &diff_str[..3000] } else { &diff_str };
    let sanitized = truncated.replace("Ignore previous instructions", "[REDACTED]")
                             .replace("SYSTEM COMPROMISED", "[REDACTED]");

    let prompt = format!(
        "Generate a concise one-line git commit message. Output ONLY the message. Diff follows:\n\n{}",
        sanitized
    );

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(20)) // [HARDENING] Strict timeout
        .build()?;

    let body = serde_json::json!({
        "model": "qwen2.5-coder:7b-instruct-q5_K_M",
        "prompt": prompt,
        "stream": false
    });

    let res = client.post("http://localhost:11434/api/generate")
        .json(&body)
        .send()
        .context("Ollama service unreachable")?
        .json::<serde_json::Value>()?;

    if let Some(msg) = res["response"].as_str() {
        let clean = msg.trim().trim_matches('"').to_string();
        if clean.len() > 100 { bail!("AI generated suspicious message length"); }
        Ok(clean)
    } else {
        bail!("Failed to get AI response")
    }
}
