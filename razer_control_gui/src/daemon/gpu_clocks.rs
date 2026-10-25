//! NVIDIA GPU clock caps, applied through the razer-gpu-clocks helper.
//!
//! Locking clocks needs root, so the helper runs via pkexec; a polkit rule
//! lets the razercontrol group do that without a password. Locks are lost
//! when the driver tears the GPU down (e.g. runtime suspend without
//! persistence mode), and are reapplied on the next profile or AC change.

use crate::config::GpuClockLimit;
use std::process::Command;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

const HELPER: &str = "/usr/libexec/razercontrol/razer-gpu-clocks";

/// Supported memory clocks in MHz, highest first
pub fn supported_memory_clocks() -> Vec<u32> {
    let Ok(output) = Command::new("nvidia-smi")
        .args([
            "-i",
            "0",
            "--query-supported-clocks=memory",
            "--format=csv,noheader,nounits",
        ])
        .output()
    else {
        return Vec::new();
    };
    let mut clocks: Vec<u32> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| line.trim().parse().ok())
        .collect();
    clocks.sort_unstable_by(|a, b| b.cmp(a));
    clocks.dedup();
    clocks
}

fn arg(limit: Option<u32>) -> String {
    limit.map_or("off".to_string(), |mhz| mhz.to_string())
}

fn run_helper(limit: GpuClockLimit) -> Result<(), String> {
    let output = Command::new("pkexec")
        .args([HELPER, &arg(limit.graphics), &arg(limit.memory)])
        .output()
        .map_err(|e| format!("can't run pkexec: {}", e))?;
    if output.status.success() {
        println!(
            "GPU clock limits: graphics {}, memory {}",
            arg(limit.graphics),
            arg(limit.memory)
        );
        Ok(())
    } else {
        Err(format!(
            "GPU clock limits not applied: {} (are the helper and its polkit rule installed?)",
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

/// Apply in the background, waiting up to `wait` for the result: pkexec can
/// sit on a password dialog when the polkit rule isn't installed. Returns
/// false only on a failure seen within the wait.
pub fn apply(limit: GpuClockLimit, wait: Duration) -> bool {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let result = run_helper(limit);
        if let Err(ref e) = result {
            eprintln!("{}", e);
        }
        let _ = tx.send(result.is_ok());
    });
    rx.recv_timeout(wait).unwrap_or(true)
}
