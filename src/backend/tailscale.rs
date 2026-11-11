//! Tailscale: whether the daemon is connected, and turning it on and off with the `tailscale`
//! CLI. `up` and `down` need the user to be Tailscale's operator
//! (`sudo tailscale set --operator=$USER`); otherwise they go through `pkexec`, which asks for a
//! password through the desktop's polkit agent.

use std::io::Read;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde_json::Value;

use super::os::tool_exists;

#[derive(Debug, Clone, PartialEq)]
pub struct TailscaleStatus {
    /// The daemon's `BackendState`: `Running`, `Stopped`, `NeedsLogin`, `Starting`, …
    pub backend_state: String,
    /// This machine's MagicDNS name, without the trailing dot.
    pub dns_name: Option<String>,
    /// The other devices on the tailnet: online first, then by name.
    pub peers: Vec<TailscalePeer>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TailscalePeer {
    /// The machine name (the first label of its MagicDNS name), else its hostname.
    pub name: String,
    /// `linux`, `android`, `iOS`, `windows`, `macOS`, …
    pub os: String,
    pub ips: Vec<String>,
    pub online: bool,
    /// Traffic to the internet goes through it.
    pub exit_node: bool,
    /// When it was last online, as "3 days ago"; `None` while online or never seen.
    pub last_seen: Option<String>,
}

impl TailscalePeer {
    pub fn mobile(&self) -> bool {
        matches!(self.os.to_ascii_lowercase().as_str(), "android" | "ios")
    }
}

impl TailscaleStatus {
    pub fn running(&self) -> bool {
        self.backend_state == "Running"
    }

    /// Whether `up` or `down` can switch it without a login: it's connected or stopped.
    pub fn can_toggle(&self) -> bool {
        matches!(self.backend_state.as_str(), "Running" | "Stopped")
    }
}

/// Whether `name` is a Tailscale tunnel (`tailscale0`, or another `tailscale*`).
pub fn is_interface(name: &str) -> bool {
    name.starts_with("tailscale")
}

/// The daemon's state; `None` where the `tailscale` CLI isn't installed.
pub fn status() -> Result<Option<TailscaleStatus>, String> {
    if !tool_exists("tailscale") {
        return Ok(None);
    }
    let output = Command::new("tailscale")
        .args(["status", "--json"])
        .output()
        .map_err(|e| format!("Couldn't run tailscale: {e}"))?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64);
    // It exits non-zero while stopped but still answers.
    match parse_status(&String::from_utf8_lossy(&output.stdout), now) {
        Some(s) => Ok(Some(s)),
        None => Err(format!(
            "Couldn't read Tailscale's status: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )),
    }
}

/// Reads `tailscale status --json`; `now` is Unix time, for how long ago peers were seen.
pub fn parse_status(json: &str, now: i64) -> Option<TailscaleStatus> {
    let v: Value = serde_json::from_str(json).ok()?;
    let backend_state = v.get("BackendState")?.as_str()?.to_owned();
    let dns_name = v
        .pointer("/Self/DNSName")
        .and_then(Value::as_str)
        .map(|n| n.trim_end_matches('.').to_owned())
        .filter(|n| !n.is_empty());
    let mut peers: Vec<TailscalePeer> = v
        .get("Peer")
        .and_then(Value::as_object)
        .map(|m| m.values().filter_map(|p| parse_peer(p, now)).collect())
        .unwrap_or_default();
    peers.sort_by(|a, b| {
        b.online
            .cmp(&a.online)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    Some(TailscaleStatus {
        backend_state,
        dns_name,
        peers,
    })
}

fn parse_peer(p: &Value, now: i64) -> Option<TailscalePeer> {
    let str_of = |key: &str| p.get(key).and_then(Value::as_str).unwrap_or_default();
    let name = str_of("DNSName")
        .split('.')
        .next()
        .filter(|n| !n.is_empty())
        .unwrap_or(str_of("HostName"))
        .to_owned();
    if name.is_empty() {
        return None;
    }
    let online = p.get("Online").and_then(Value::as_bool).unwrap_or(false);
    let last_seen = (!online)
        .then(|| unix_time(str_of("LastSeen")))
        .flatten()
        .map(|t| ago(now - t));
    Some(TailscalePeer {
        name,
        os: str_of("OS").to_owned(),
        ips: p
            .get("TailscaleIPs")
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(|ip| Some(ip.as_str()?.to_owned()))
                    .collect()
            })
            .unwrap_or_default(),
        online,
        exit_node: p.get("ExitNode").and_then(Value::as_bool).unwrap_or(false),
        last_seen,
    })
}

/// Unix seconds of an RFC 3339 UTC time (`2026-11-03T06:17:38.1Z`); `None` for Go's zero
/// time (never seen) and anything unreadable.
fn unix_time(t: &str) -> Option<i64> {
    let num = |r: std::ops::Range<usize>| t.get(r)?.parse::<i64>().ok();
    let (y, m, d) = (num(0..4)?, num(5..7)?, num(8..10)?);
    let (hh, mm, ss) = (num(11..13)?, num(14..16)?, num(17..19)?);
    if y < 2000 || !(1..=12).contains(&m) {
        return None;
    }
    // Days since 1970-01-01 (Howard Hinnant's days_from_civil).
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * ((m + 9) % 12) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    Some(days * 86_400 + hh * 3600 + mm * 60 + ss)
}

fn ago(secs: i64) -> String {
    let plural = |n: i64, unit: &str| format!("{n} {unit}{} ago", if n == 1 { "" } else { "s" });
    match secs {
        ..60 => "just now".into(),
        60..3600 => plural(secs / 60, "minute"),
        3600..86_400 => plural(secs / 3600, "hour"),
        86_400..2_592_000 => plural(secs / 86_400, "day"),
        _ => plural(secs / 2_592_000, "month"),
    }
}

/// How long `up` or `down` may take, including a password prompt.
const TIMEOUT: Duration = Duration::from_secs(60);

/// Connects (`tailscale up`) or disconnects (`tailscale down`). Blocks; run it off the UI
/// thread.
pub fn set_running(on: bool) -> Result<(), String> {
    let verb = if on { "up" } else { "down" };
    let (ok, err) = run(Command::new("tailscale").arg(verb))?;
    if ok {
        return Ok(());
    }
    if !needs_privilege(&err) {
        return Err(err);
    }
    if !tool_exists("pkexec") {
        return Err(format!("{err}\n{OPERATOR_HINT}"));
    }
    let (ok, err) = run(Command::new("pkexec").args(["tailscale", verb]))?;
    if ok {
        Ok(())
    } else if err.is_empty() || err.contains("Not authorized") || err.contains("dismissed") {
        Err(format!("Authorization was cancelled. {OPERATOR_HINT}"))
    } else {
        Err(err)
    }
}

const OPERATOR_HINT: &str =
    "To switch Tailscale without a password, run `sudo tailscale set --operator=$USER` once.";

/// Whether `tailscale` refused for want of permission (the user isn't its operator).
fn needs_privilege(stderr: &str) -> bool {
    let e = stderr.to_ascii_lowercase();
    e.contains("access denied") || e.contains("permission denied") || e.contains("sudo")
}

/// Runs `cmd` to the end or `TIMEOUT`: whether it succeeded, and its trimmed stderr.
fn run(cmd: &mut Command) -> Result<(bool, String), String> {
    let mut child = cmd
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Couldn't run tailscale: {e}"))?;
    let mut stderr = child.stderr.take();
    // Read stderr on the side, so a chatty command can't fill the pipe and stall.
    let reader = std::thread::spawn(move || {
        let mut s = String::new();
        if let Some(e) = stderr.as_mut() {
            let _ = e.read_to_string(&mut s);
        }
        s
    });
    let started = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
            break status;
        }
        if started.elapsed() > TIMEOUT {
            let _ = child.kill();
            let _ = child.wait();
            return Err("Tailscale didn't answer in time".into());
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    let err = reader.join().unwrap_or_default().trim().to_owned();
    Ok((status.success(), err))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_state_and_name() {
        let s = parse_status(
            r#"{"BackendState":"Running","Self":{"DNSName":"home.tail1234.ts.net."}}"#,
            0,
        )
        .unwrap();
        assert!(s.running());
        assert!(s.can_toggle());
        assert_eq!(s.dns_name.as_deref(), Some("home.tail1234.ts.net"));

        let s = parse_status(r#"{"BackendState":"NeedsLogin","Self":null}"#, 0).unwrap();
        assert!(!s.running());
        assert!(!s.can_toggle());
        assert_eq!(s.dns_name, None);

        assert_eq!(parse_status("Tailscale is stopped.", 0), None);
    }

    #[test]
    fn reads_the_peers() {
        let json = r#"{"BackendState":"Running","Peer":{
            "k1":{"HostName":"Alex S","DNSName":"phone.tail1234.ts.net.","OS":"android",
                  "TailscaleIPs":["100.79.40.34","fd7a::1"],"Online":false,
                  "LastSeen":"2026-11-03T06:17:38.1Z","ExitNode":false},
            "k2":{"HostName":"nas","DNSName":"","OS":"linux","TailscaleIPs":["100.64.0.2"],
                  "Online":true,"LastSeen":"0001-01-01T00:00:00Z","ExitNode":true}}}"#;
        // 2026-11-11T06:17:38Z
        let now = unix_time("2026-11-11T06:17:38Z").unwrap();
        let s = parse_status(json, now).unwrap();
        let names: Vec<_> = s.peers.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(
            names,
            ["nas", "phone"],
            "online first; hostname without a DNS name"
        );
        assert!(s.peers[0].exit_node && s.peers[0].last_seen.is_none());
        assert_eq!(s.peers[1].last_seen.as_deref(), Some("8 days ago"));
        assert!(s.peers[1].mobile());
        assert_eq!(s.peers[1].ips[0], "100.79.40.34");
    }

    #[test]
    fn reads_times() {
        assert_eq!(
            unix_time("1970-01-01T00:00:00Z"),
            None,
            "before 2000 is Go's zero"
        );
        assert_eq!(unix_time("2000-03-01T00:00:01Z"), Some(951_868_801));
        assert_eq!(unix_time("2026-11-03T06:17:38.1Z"), Some(1_793_686_658));
        assert_eq!(ago(30), "just now");
        assert_eq!(ago(61), "1 minute ago");
        assert_eq!(ago(7200), "2 hours ago");
    }

    #[test]
    fn spots_a_refusal_for_permission() {
        assert!(needs_privilege(
            "Access denied: prefs write access denied\n\nUse 'sudo tailscale down'."
        ));
        assert!(!needs_privilege("failed to connect to local tailscaled"));
    }
}
