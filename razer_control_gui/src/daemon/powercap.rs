//! CPU package power limits (RAPL PL1/PL2) via the powercap sysfs interface.
//!
//! Firmware can program the MSR and MMIO RAPL interfaces differently and the
//! lower limit is the one that takes effect, so both are written. The daemon
//! runs as the session user: writing relies on the udev rule that makes just
//! these attributes writable by the razercontrol group.

use crate::config::CpuPowerLimit;
use std::fs;
use std::io;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

const POWERCAP_DIR: &str = "/sys/class/powercap";
const ZONES: [&str; 2] = ["intel-rapl:0", "intel-rapl-mmio:0"];

static WRITE_ERROR_LOGGED: AtomicBool = AtomicBool::new(false);

/// Set PL1/PL2 on every package zone present. Returns whether any zone took
/// the new limits; failures are logged once rather than on every mode change.
pub fn apply(limit: CpuPowerLimit) -> bool {
    apply_in(Path::new(POWERCAP_DIR), limit)
}

fn apply_in(root: &Path, limit: CpuPowerLimit) -> bool {
    let mut applied = false;
    for zone in ZONES {
        let dir = root.join(zone);
        if !dir.is_dir() {
            continue;
        }
        for (constraint, watts) in [("long_term", limit.pl1), ("short_term", limit.pl2)] {
            match write_constraint(&dir, constraint, watts) {
                Ok(()) => applied = true,
                Err(e) => {
                    if !WRITE_ERROR_LOGGED.swap(true, Ordering::Relaxed) {
                        eprintln!(
                            "Can't set CPU power limit {}/{}: {} (is the powercap udev rule installed and the user in the razercontrol group?)",
                            zone, constraint, e
                        );
                    }
                }
            }
        }
    }
    if applied {
        println!("CPU power limits: PL1 {} W, PL2 {} W", limit.pl1, limit.pl2);
    }
    applied
}

/// Constraint indices vary between zones, so find the one by name.
fn write_constraint(zone: &Path, name: &str, watts: u32) -> io::Result<()> {
    for i in 0..3 {
        let Ok(constraint) = fs::read_to_string(zone.join(format!("constraint_{}_name", i))) else {
            continue;
        };
        if constraint.trim() == name {
            let microwatts = u64::from(watts) * 1_000_000;
            return fs::write(
                zone.join(format!("constraint_{}_power_limit_uw", i)),
                microwatts.to_string(),
            );
        }
    }
    Err(io::Error::new(
        io::ErrorKind::NotFound,
        format!("no {} constraint", name),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::atomic::AtomicU64;

    static NEXT_ID: AtomicU64 = AtomicU64::new(0);

    fn fake_powercap(zones: &[&str]) -> PathBuf {
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "razer-control-powercap-{}-{}",
            std::process::id(),
            id
        ));
        for zone in zones {
            let dir = root.join(zone);
            fs::create_dir_all(&dir).unwrap();
            // Short term first, to check constraints are matched by name.
            for (i, name) in ["short_term", "long_term", "peak_power"].iter().enumerate() {
                fs::write(
                    dir.join(format!("constraint_{}_name", i)),
                    format!("{}\n", name),
                )
                .unwrap();
                fs::write(dir.join(format!("constraint_{}_power_limit_uw", i)), "0\n").unwrap();
            }
        }
        root
    }

    fn read_uw(root: &Path, zone: &str, index: u8) -> String {
        fs::read_to_string(
            root.join(zone)
                .join(format!("constraint_{}_power_limit_uw", index)),
        )
        .unwrap()
    }

    #[test]
    fn writes_both_zones_by_constraint_name() {
        let root = fake_powercap(&ZONES);
        assert!(apply_in(&root, CpuPowerLimit { pl1: 25, pl2: 45 }));
        for zone in ZONES {
            assert_eq!(read_uw(&root, zone, 1), "25000000");
            assert_eq!(read_uw(&root, zone, 0), "45000000");
            assert_eq!(read_uw(&root, zone, 2), "0\n");
        }
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn missing_zones_are_skipped() {
        let root = fake_powercap(&["intel-rapl:0"]);
        assert!(apply_in(&root, CpuPowerLimit { pl1: 55, pl2: 115 }));
        assert_eq!(read_uw(&root, "intel-rapl:0", 1), "55000000");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn nothing_applied_without_rapl() {
        let root = fake_powercap(&[]);
        fs::create_dir_all(&root).unwrap();
        assert!(!apply_in(&root, CpuPowerLimit { pl1: 55, pl2: 115 }));
        fs::remove_dir_all(root).unwrap();
    }
}
