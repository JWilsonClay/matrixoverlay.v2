// src/core/productivity/ai.rs
use anyhow::{Result, Context, bail};
use git2::Repository;

/// [HARDENED] Generates a commit message via LiteRT + Gemma 2 (4b) with prompt injection protection.
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
        "<start_of_turn>user\nGenerate a concise one-line git commit message. Output ONLY the message. Diff follows:\n\n{}<end_of_turn>\n<start_of_turn>model\n",
        sanitized
    );

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(20)) // [HARDENING] Strict timeout
        .build()?;

    let body = serde_json::json!({
        "prompt": prompt,
        "max_tokens": 100
    });

    let res = client.post("http://localhost:8000/generate")
        .json(&body)
        .send()
        .context("LiteRT service unreachable at port 8000")?
        .json::<serde_json::Value>()?;

    if let Some(msg) = res["generated_text"].as_str() {
        let clean = msg.trim().trim_matches('"').to_string();
        if clean.len() > 150 { bail!("AI generated suspicious message length"); }
        Ok(clean)
    } else {
        bail!("Failed to get LiteRT response")
    }
}
