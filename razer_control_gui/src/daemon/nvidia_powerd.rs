//! Start and stop nvidia-powerd (NVIDIA Dynamic Boost).
//!
//! With powerd running, the GPU power limit follows the firmware's per-mode
//! tables, which on some laptops only ever raise it above the base TGP. The
//! daemon runs as the session user, so it relies on the polkit rule that lets
//! the active local user start and stop just this unit. Without the rule,
//! GNOME may still show a password dialog, so callers run this off-thread.

use std::process::Command;

const UNIT: &str = "nvidia-powerd.service";

/// Whether the service is installed
pub fn available() -> bool {
    Command::new("systemctl")
        .args(["show", "--property=LoadState", "--value", UNIT])
        .output()
        .is_ok_and(|o| String::from_utf8_lossy(&o.stdout).trim() == "loaded")
}

pub fn is_active() -> bool {
    Command::new("systemctl")
        .args(["is-active", "--quiet", UNIT])
        .status()
        .is_ok_and(|s| s.success())
}

/// Start or stop the service. Returns (success, message).
pub fn set(enabled: bool) -> (bool, String) {
    let verb = if enabled { "start" } else { "stop" };
    match Command::new("systemctl")
        .args(["--no-ask-password", verb, UNIT])
        .output()
    {
        Ok(output) if output.status.success() => {
            let msg = format!("Dynamic Boost {}", if enabled { "on" } else { "off" });
            println!("{} ({} {})", msg, verb, UNIT);
            (true, msg)
        }
        Ok(output) => {
            let msg = format!(
                "Can't {} {}: {} (is the polkit rule installed?)",
                verb,
                UNIT,
                String::from_utf8_lossy(&output.stderr).trim()
            );
            eprintln!("{}", msg);
            (false, msg)
        }
        Err(e) => {
            let msg = format!("Failed to run systemctl: {}", e);
            eprintln!("{}", msg);
            (false, msg)
        }
    }
}

/// Bring the service to the wanted state, if it's installed
pub fn ensure(enabled: bool) {
    if available() && is_active() != enabled {
        set(enabled);
    }
}
