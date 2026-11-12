//! Client for the `noctalia` command-line tool: send a message, read the reply.

use std::process::Command;

/// `noctalia msg <args>`: its output, or why it failed.
pub(crate) fn noctalia_msg(args: &[&str]) -> Result<String, String> {
    let output = Command::new("noctalia")
        .arg("msg")
        .args(args)
        .output()
        .map_err(|e| format!("Failed to run noctalia: {}", e))?;
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    noctalia_reply(output.status.success(), stdout, &stderr)
}

/// Noctalia prints `error: ...` and exits 1 when it refuses a request.
fn noctalia_reply(success: bool, stdout: String, stderr: &str) -> Result<String, String> {
    if success && !stdout.starts_with("error:") {
        return Ok(stdout);
    }
    let detail = [stdout.as_str(), stderr]
        .into_iter()
        .find(|d| !d.is_empty())
        .unwrap_or("noctalia msg failed");
    Err(detail.trim_start_matches("error:").trim().to_string())
}

/// `config-get` prints TOML values: `true`, `'crop'`.
pub(crate) fn toml_scalar(value: &str) -> String {
    value.trim().trim_matches(['\'', '"']).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_noctalia_replies() {
        assert_eq!(
            noctalia_reply(true, "/home/u/a.jpg".into(), ""),
            Ok("/home/u/a.jpg".into())
        );
        assert_eq!(
            noctalia_reply(
                false,
                "error: unknown output \"X\"; known: eDP-1".into(),
                ""
            ),
            Err("unknown output \"X\"; known: eDP-1".into())
        );
        assert_eq!(
            noctalia_reply(false, String::new(), "no instance running"),
            Err("no instance running".into())
        );
        assert_eq!(toml_scalar("'crop'\n"), "crop");
        assert_eq!(toml_scalar("true"), "true");
    }
}
