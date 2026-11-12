//! The About page's system facts: OS, kernel, uptime, GPU, memory, disk, CPU.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SystemInfo {
    pub os: String,
    pub hostname: String,
    pub kernel: String,
    pub uptime: String,
    pub shell: String,
    pub hyprland_version: String,
    pub gpus: Vec<String>,
    pub ram_used: String,
    pub ram_total: String,
    pub disk_used: String,
    pub disk_total: String,
    pub cpu: String,
}

pub fn get_system_info() -> Result<SystemInfo, String> {
    Ok(SystemInfo {
        os: get_os_info(),
        hostname: get_hostname(),
        kernel: get_kernel_version(),
        uptime: get_uptime(),
        shell: get_shell_info(),
        hyprland_version: get_hyprland_version(),
        gpus: get_gpu_info(),
        ram_used: get_ram_used(),
        ram_total: get_ram_total(),
        disk_used: get_disk_used(),
        disk_total: get_disk_total(),
        cpu: get_cpu_info(),
    })
}

fn get_os_info() -> String {
    // Try to read /etc/os-release
    if let Ok(contents) = fs::read_to_string("/etc/os-release") {
        for line in contents.lines() {
            if line.starts_with("PRETTY_NAME=") {
                let name = line.strip_prefix("PRETTY_NAME=").unwrap_or("");
                return name.trim_matches('"').to_string();
            }
        }
    }
    "Unknown".to_string()
}

fn get_hostname() -> String {
    Command::new("hostname")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "Unknown".to_string())
}

fn get_kernel_version() -> String {
    Command::new("uname")
        .arg("-r")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "Unknown".to_string())
}

fn get_uptime() -> String {
    if let Ok(contents) = fs::read_to_string("/proc/uptime")
        && let Some(uptime_seconds) = contents.split_whitespace().next()
        && let Ok(seconds) = uptime_seconds.parse::<f64>()
    {
        let days = (seconds / 86400.0).floor() as u64;
        let hours = ((seconds % 86400.0) / 3600.0).floor() as u64;
        let minutes = ((seconds % 3600.0) / 60.0).floor() as u64;

        if days > 0 {
            return format!("{}d {}h {}m", days, hours, minutes);
        } else if hours > 0 {
            return format!("{}h {}m", hours, minutes);
        } else {
            return format!("{}m", minutes);
        }
    }
    "Unknown".to_string()
}

fn get_shell_info() -> String {
    let shell_path = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
    let shell_name = Path::new(&shell_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("sh");

    // Try to get shell version
    let version_output = Command::new(&shell_path).arg("--version").output();

    if let Ok(output) = version_output
        && output.status.success()
    {
        let version_str = String::from_utf8_lossy(&output.stdout);
        // Get first line which usually contains version info
        if let Some(first_line) = version_str.lines().next() {
            return first_line.trim().to_string();
        }
    }

    shell_name.to_string()
}

fn get_hyprland_version() -> String {
    Command::new("hyprctl")
        .arg("version")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|s| s.lines().next().map(|l| l.to_string()))
        .unwrap_or_else(|| "Unknown".to_string())
}

fn get_gpu_info() -> Vec<String> {
    let output = Command::new("lspci").output();

    if let Ok(output) = output
        && output.status.success()
    {
        let output_str = String::from_utf8_lossy(&output.stdout);
        let mut gpus = Vec::new();

        for line in output_str.lines() {
            if line.contains("VGA compatible controller") || line.contains("3D controller") {
                // Extract GPU name after the controller type
                if let Some(gpu_name) = line.split(':').nth(2) {
                    gpus.push(gpu_name.trim().to_string());
                }
            }
        }

        if !gpus.is_empty() {
            return gpus;
        }
    }

    vec!["Unknown".to_string()]
}

fn get_ram_used() -> String {
    if let Ok(contents) = fs::read_to_string("/proc/meminfo") {
        let mut total = 0u64;
        let mut available = 0u64;

        for line in contents.lines() {
            if line.starts_with("MemTotal:") {
                if let Some(value) = line.split_whitespace().nth(1) {
                    total = value.parse::<u64>().unwrap_or(0);
                }
            } else if line.starts_with("MemAvailable:")
                && let Some(value) = line.split_whitespace().nth(1)
            {
                available = value.parse::<u64>().unwrap_or(0);
            }
        }

        if total > 0 && available > 0 {
            let used = total - available;
            return format!("{:.2} GB", used as f64 / 1024.0 / 1024.0);
        }
    }

    "Unknown".to_string()
}

fn get_ram_total() -> String {
    if let Ok(contents) = fs::read_to_string("/proc/meminfo") {
        for line in contents.lines() {
            if line.starts_with("MemTotal:")
                && let Some(value) = line.split_whitespace().nth(1)
                && let Ok(kb) = value.parse::<u64>()
            {
                return format!("{:.2} GB", kb as f64 / 1024.0 / 1024.0);
            }
        }
    }

    "Unknown".to_string()
}

fn get_disk_used() -> String {
    let output = Command::new("df").args(["-h", "/"]).output();

    if let Ok(output) = output
        && output.status.success()
    {
        let output_str = String::from_utf8_lossy(&output.stdout);
        if let Some(line) = output_str.lines().nth(1)
            && let Some(used) = line.split_whitespace().nth(2)
        {
            return used.to_string();
        }
    }

    "Unknown".to_string()
}

fn get_disk_total() -> String {
    let output = Command::new("df").args(["-h", "/"]).output();

    if let Ok(output) = output
        && output.status.success()
    {
        let output_str = String::from_utf8_lossy(&output.stdout);
        if let Some(line) = output_str.lines().nth(1)
            && let Some(total) = line.split_whitespace().nth(1)
        {
            return total.to_string();
        }
    }

    "Unknown".to_string()
}

fn get_cpu_info() -> String {
    if let Ok(contents) = fs::read_to_string("/proc/cpuinfo") {
        for line in contents.lines() {
            if line.starts_with("model name")
                && let Some(name) = line.split(':').nth(1)
            {
                return name.trim().to_string();
            }
        }
    }

    "Unknown".to_string()
}
