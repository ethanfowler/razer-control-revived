//! CPU turbo boost: intel_pstate's no_turbo, or the generic cpufreq boost
//! switch on other drivers. Reading is open to everyone; writing needs root,
//! so the daemon goes through razer-helper, which includes this file.

use std::fs;
use std::io;
use std::path::Path;

const CPU_DIR: &str = "/sys/devices/system/cpu";
const NO_TURBO: &str = "intel_pstate/no_turbo";
const BOOST: &str = "cpufreq/boost";

/// Whether turbo is enabled, or None if it can't be controlled
pub fn get() -> Option<bool> {
    get_in(Path::new(CPU_DIR))
}

/// Enable or disable turbo
pub fn write(enabled: bool) -> io::Result<()> {
    set_in(Path::new(CPU_DIR), enabled)
}

fn read_flag(path: &Path) -> Option<bool> {
    match fs::read_to_string(path).ok()?.trim() {
        "0" => Some(false),
        "1" => Some(true),
        _ => None,
    }
}

fn get_in(root: &Path) -> Option<bool> {
    read_flag(&root.join(NO_TURBO))
        .map(|no_turbo| !no_turbo)
        .or_else(|| read_flag(&root.join(BOOST)))
}

fn set_in(root: &Path, enabled: bool) -> io::Result<()> {
    let flag = |on: bool| if on { "1" } else { "0" };
    let no_turbo = root.join(NO_TURBO);
    if no_turbo.exists() {
        fs::write(no_turbo, flag(!enabled))
    } else {
        fs::write(root.join(BOOST), flag(enabled))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_ID: AtomicU64 = AtomicU64::new(0);

    fn fake_cpu_dir(file: &str, value: &str) -> PathBuf {
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        let root =
            std::env::temp_dir().join(format!("razer-control-turbo-{}-{}", std::process::id(), id));
        let path = root.join(file);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, value).unwrap();
        root
    }

    #[test]
    fn intel_pstate_no_turbo_is_inverted() {
        let root = fake_cpu_dir(NO_TURBO, "1\n");
        assert_eq!(get_in(&root), Some(false));
        set_in(&root, true).unwrap();
        assert_eq!(fs::read_to_string(root.join(NO_TURBO)).unwrap(), "0");
        assert_eq!(get_in(&root), Some(true));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn cpufreq_boost_is_used_without_intel_pstate() {
        let root = fake_cpu_dir(BOOST, "1\n");
        assert_eq!(get_in(&root), Some(true));
        set_in(&root, false).unwrap();
        assert_eq!(fs::read_to_string(root.join(BOOST)).unwrap(), "0");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn no_turbo_control() {
        let root = fake_cpu_dir("online", "0-31\n");
        assert_eq!(get_in(&root), None);
        assert!(set_in(&root, true).is_err());
        fs::remove_dir_all(root).unwrap();
    }
}
