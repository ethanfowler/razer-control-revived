//! razer-helper: privileged helper for the power settings that need root.
//!
//! The daemon runs as the session user, so it runs this through pkexec; the
//! polkit policy lets the active local user do so without a password. Each
//! command takes bounded values and sets exactly one thing, so being able to
//! run it grants nothing beyond what the GUI offers.
//!
//! Usage:
//!   razer-helper cpu-limit <PL1 W> <PL2 W>
//!   razer-helper turbo <on|off>
//!   razer-helper gpu-clocks <graphics MHz|off> <memory MHz|off>
//!
//! GPU clock caps only set an upper bound (the lower bound is the lowest
//! clock the GPU supports) or remove it.

use std::process::{Command, ExitCode};

#[path = "../daemon/powercap.rs"]
mod powercap;
#[allow(dead_code)] // reading is the daemon's job
#[path = "../daemon/turbo.rs"]
mod turbo;

const NVIDIA_SMI: &str = "/usr/bin/nvidia-smi";
const MAX_MHZ: u32 = 20_000;
const USAGE: &str = "usage: razer-helper cpu-limit <PL1 W> <PL2 W> | turbo <on|off> | gpu-clocks <graphics MHz|off> <memory MHz|off>";

/// PL1/PL2 in watts, bounded as the daemon's config is
fn parse_cpu_limit(pl1: &str, pl2: &str) -> Result<(u32, u32), String> {
    let watts = |arg: &str| {
        arg.parse::<u32>()
            .map_err(|_| format!("invalid power limit '{}': expected watts", arg))
    };
    let (pl1, pl2) = (watts(pl1)?, watts(pl2)?);
    if (5..=250).contains(&pl1) && (pl1..=250).contains(&pl2) {
        Ok((pl1, pl2))
    } else {
        Err(format!(
            "power limits out of range: PL1 {} W, PL2 {} W (need 5 <= PL1 <= PL2 <= 250)",
            pl1, pl2
        ))
    }
}

fn parse_switch(arg: &str) -> Result<bool, String> {
    match arg {
        "on" => Ok(true),
        "off" => Ok(false),
        _ => Err(format!("invalid switch '{}': expected on or off", arg)),
    }
}

fn parse_limit(arg: &str) -> Result<Option<u32>, String> {
    if arg == "off" {
        return Ok(None);
    }
    match arg.parse::<u32>() {
        Ok(mhz) if (1..=MAX_MHZ).contains(&mhz) => Ok(Some(mhz)),
        _ => Err(format!("invalid clock '{}': expected MHz or 'off'", arg)),
    }
}

/// Lowest clock the GPU supports in a domain ("graphics" or "memory")
fn lowest_supported(domain: &str) -> Result<u32, String> {
    let output = Command::new(NVIDIA_SMI)
        .args([
            "-i",
            "0",
            &format!("--query-supported-clocks={}", domain),
            "--format=csv,noheader,nounits",
        ])
        .output()
        .map_err(|e| format!("can't run nvidia-smi: {}", e))?;
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| line.trim().parse::<u32>().ok())
        .min()
        .ok_or_else(|| format!("no supported {} clocks reported", domain))
}

fn nvidia_smi(args: &[&str]) -> Result<(), String> {
    let output = Command::new(NVIDIA_SMI)
        .args(["-i", "0"])
        .args(args)
        .output()
        .map_err(|e| format!("can't run nvidia-smi: {}", e))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "nvidia-smi {}: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stdout).trim()
        ))
    }
}

/// Cap a domain's clock, or remove the cap
fn apply(domain: &str, lock: &str, reset: &str, limit: Option<u32>) -> Result<(), String> {
    match limit {
        Some(max) => {
            let min = lowest_supported(domain)?.min(max);
            nvidia_smi(&[lock, &format!("{},{}", min, max)])
        }
        None => nvidia_smi(&[reset]),
    }
}

fn run(args: &[String]) -> Result<(), String> {
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    match args[..] {
        ["cpu-limit", pl1, pl2] => {
            let (pl1, pl2) = parse_cpu_limit(pl1, pl2)?;
            powercap::write(pl1, pl2).map_err(|e| format!("can't set CPU power limit {}", e))
        }
        ["turbo", enabled] => {
            turbo::write(parse_switch(enabled)?).map_err(|e| format!("can't set CPU turbo: {}", e))
        }
        ["gpu-clocks", graphics, memory] => {
            let graphics = parse_limit(graphics)?;
            let memory = parse_limit(memory)?;
            apply("graphics", "-lgc", "-rgc", graphics)?;
            apply("memory", "-lmc", "-rmc", memory)
        }
        _ => Err(USAGE.to_string()),
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("razer-helper: {}", e);
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_limits() {
        assert_eq!(parse_limit("off"), Ok(None));
        assert_eq!(parse_limit("1200"), Ok(Some(1200)));
        assert!(parse_limit("0").is_err());
        assert!(parse_limit("99999").is_err());
        assert!(parse_limit("-1").is_err());
        assert!(parse_limit("1200; reboot").is_err());
    }

    #[test]
    fn parses_cpu_limits() {
        assert_eq!(parse_cpu_limit("25", "45"), Ok((25, 45)));
        assert_eq!(parse_cpu_limit("55", "55"), Ok((55, 55)));
        assert!(parse_cpu_limit("4", "45").is_err());
        assert!(parse_cpu_limit("45", "25").is_err());
        assert!(parse_cpu_limit("55", "251").is_err());
        assert!(parse_cpu_limit("55", "x").is_err());
    }

    #[test]
    fn parses_switches() {
        assert_eq!(parse_switch("on"), Ok(true));
        assert_eq!(parse_switch("off"), Ok(false));
        assert!(parse_switch("1").is_err());
    }

    #[test]
    fn rejects_unknown_commands_and_argument_counts() {
        let args = |list: &[&str]| list.iter().map(|a| a.to_string()).collect::<Vec<_>>();
        assert!(run(&args(&[])).is_err());
        assert!(run(&args(&["reboot"])).is_err());
        assert!(run(&args(&["gpu-clocks", "1200"])).is_err());
        assert!(run(&args(&["turbo", "on", "off"])).is_err());
        assert!(run(&args(&["cpu-limit", "55"])).is_err());
    }
}
