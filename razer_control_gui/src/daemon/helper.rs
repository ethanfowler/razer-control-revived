//! Power settings that need root, applied through the razer-helper binary.
//!
//! The daemon runs as the session user, so it runs the helper via pkexec; the
//! helper's polkit policy lets the active local user do that without a
//! password. GPU clock locks are lost when the driver tears the GPU down
//! (e.g. runtime suspend without persistence mode), and are reapplied on the
//! next profile or AC change.

use crate::config::{CpuPowerLimit, GpuClockLimit};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, mpsc};
use std::thread;
use std::time::Duration;

const HELPER: &str = "/usr/libexec/razercontrol/razer-helper";

/// Settings the helper changes, each with the generation of its latest request
#[derive(Clone, Copy)]
enum Setting {
    CpuLimit,
    Turbo,
    GpuClocks,
}

static GENERATIONS: [AtomicU64; 3] = [AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0)];
static RUNNING: Mutex<()> = Mutex::new(());

pub fn set_cpu_power_limit(limit: CpuPowerLimit, wait: Duration) -> bool {
    apply(
        Setting::CpuLimit,
        vec![
            "cpu-limit".to_string(),
            limit.pl1.to_string(),
            limit.pl2.to_string(),
        ],
        format!("CPU power limits: PL1 {} W, PL2 {} W", limit.pl1, limit.pl2),
        wait,
    )
}

pub fn set_turbo(enabled: bool, wait: Duration) -> bool {
    let state = if enabled { "on" } else { "off" };
    apply(
        Setting::Turbo,
        vec!["turbo".to_string(), state.to_string()],
        format!("CPU turbo {}", state),
        wait,
    )
}

pub fn set_gpu_clocks(limit: GpuClockLimit, wait: Duration) -> bool {
    let (graphics, memory) = (arg(limit.graphics), arg(limit.memory));
    apply(
        Setting::GpuClocks,
        vec!["gpu-clocks".to_string(), graphics.clone(), memory.clone()],
        format!("GPU clock limits: graphics {}, memory {}", graphics, memory),
        wait,
    )
}

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

fn run_helper(args: &[String]) -> Result<(), String> {
    let output = Command::new("pkexec")
        .arg(HELPER)
        .args(args)
        .output()
        .map_err(|e| format!("can't run pkexec: {}", e))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "{} (are razer-helper and its polkit policy installed?)",
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

/// Run the helper in the background, waiting up to `wait` for the result:
/// pkexec can sit on a password dialog when the polkit policy isn't
/// installed. Runs are serialised, and a request superseded by a newer one
/// for the same setting is skipped, so the last request wins. Returns false
/// only on a failure seen within the wait.
fn apply(setting: Setting, args: Vec<String>, done: String, wait: Duration) -> bool {
    let generation = &GENERATIONS[setting as usize];
    let mine = generation.fetch_add(1, Ordering::SeqCst) + 1;
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let _running = RUNNING.lock().unwrap_or_else(|e| e.into_inner());
        if generation.load(Ordering::SeqCst) != mine {
            let _ = tx.send(true);
            return;
        }
        let result = run_helper(&args);
        match result {
            Ok(()) => println!("{}", done),
            Err(ref e) => eprintln!("Not applied: {}", e),
        }
        let _ = tx.send(result.is_ok());
    });
    rx.recv_timeout(wait).unwrap_or(true)
}
