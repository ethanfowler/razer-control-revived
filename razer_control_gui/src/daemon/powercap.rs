//! CPU package power limits (RAPL PL1/PL2) via the powercap sysfs interface.
//!
//! Firmware can program the MSR and MMIO RAPL interfaces differently and the
//! lower limit is the one that takes effect, so both are written. Writing
//! needs root: the daemon goes through razer-helper, which includes this file.

use std::fs;
use std::io;
use std::path::Path;

const POWERCAP_DIR: &str = "/sys/class/powercap";
const ZONES: [&str; 2] = ["intel-rapl:0", "intel-rapl-mmio:0"];

/// Set PL1/PL2 in watts on every package zone present. Succeeds if any zone
/// took the new limits.
pub fn write(pl1: u32, pl2: u32) -> io::Result<()> {
    write_in(Path::new(POWERCAP_DIR), pl1, pl2)
}

fn write_in(root: &Path, pl1: u32, pl2: u32) -> io::Result<()> {
    let mut result = Err(io::Error::new(
        io::ErrorKind::NotFound,
        "no RAPL package zone",
    ));
    for zone in ZONES {
        let dir = root.join(zone);
        if !dir.is_dir() {
            continue;
        }
        for (constraint, watts) in [("long_term", pl1), ("short_term", pl2)] {
            match write_constraint(&dir, constraint, watts) {
                Ok(()) => result = Ok(()),
                Err(e) if result.is_err() => {
                    result = Err(io::Error::new(
                        e.kind(),
                        format!("{}/{}: {}", zone, constraint, e),
                    ))
                }
                Err(_) => {}
            }
        }
    }
    result
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
    use std::sync::atomic::{AtomicU64, Ordering};

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
        write_in(&root, 25, 45).unwrap();
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
        write_in(&root, 55, 115).unwrap();
        assert_eq!(read_uw(&root, "intel-rapl:0", 1), "55000000");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn nothing_applied_without_rapl() {
        let root = fake_powercap(&[]);
        fs::create_dir_all(&root).unwrap();
        assert!(write_in(&root, 55, 115).is_err());
        fs::remove_dir_all(root).unwrap();
    }
}
