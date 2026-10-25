//! razer-gpu-clocks: privileged helper that caps or unlocks the NVIDIA GPU's
//! graphics and memory clocks.
//!
//! Clock locks need root, but the daemon runs as the session user, so it runs
//! this through pkexec. The helper only ever sets an upper clock bound (the
//! lower bound is the lowest clock the GPU supports) or removes it, so being
//! able to run it grants nothing beyond that.
//!
//! Usage: razer-gpu-clocks <graphics MHz|off> <memory MHz|off>

use std::process::{Command, ExitCode};

const NVIDIA_SMI: &str = "/usr/bin/nvidia-smi";
const MAX_MHZ: u32 = 20_000;

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
    let [graphics, memory] = args else {
        return Err("usage: razer-gpu-clocks <graphics MHz|off> <memory MHz|off>".to_string());
    };
    let graphics = parse_limit(graphics)?;
    let memory = parse_limit(memory)?;
    apply("graphics", "-lgc", "-rgc", graphics)?;
    apply("memory", "-lmc", "-rmc", memory)
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("razer-gpu-clocks: {}", e);
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
    fn rejects_wrong_argument_count() {
        assert!(run(&["1200".to_string()]).is_err());
    }
}
