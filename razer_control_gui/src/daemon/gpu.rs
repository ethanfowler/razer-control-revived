use std::fs;
use std::path::Path;
use std::process::Command;
use std::sync::OnceLock;

use super::comms::GpuInfo;

/// Known GPU vendor IDs
const VENDOR_NVIDIA: &str = "0x10de";
const VENDOR_AMD: &str = "0x1002";
const VENDOR_INTEL: &str = "0x8086";

/// Scan PCI devices and return all detected GPUs
pub fn discover_gpus() -> Vec<GpuInfo> {
    let mut gpus = Vec::new();
    let pci_dir = Path::new("/sys/bus/pci/devices");

    let entries = match fs::read_dir(pci_dir) {
        Ok(e) => e,
        Err(_) => return gpus,
    };

    for entry in entries.flatten() {
        let dev_path = entry.path();
        let class = read_sysfs_trimmed(&dev_path.join("class"));

        // Check if this is a GPU (VGA controller or 3D controller)
        let is_gpu = match class.as_deref() {
            Some(c) => c.starts_with("0x0300") || c.starts_with("0x0302"),
            None => false,
        };
        if !is_gpu {
            continue;
        }

        let vendor = read_sysfs_trimmed(&dev_path.join("vendor"));
        let pci_slot = entry.file_name().to_string_lossy().to_string();

        // Determine driver from symlink
        let driver = match fs::read_link(dev_path.join("driver")) {
            Ok(link) => link
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default(),
            Err(_) => String::new(),
        };

        // Determine GPU type based on vendor
        let gpu_type = match vendor.as_deref() {
            Some(VENDOR_NVIDIA) => "dgpu".to_string(),
            Some(VENDOR_AMD) => {
                if driver == "amdgpu" {
                    // On hybrid laptops, AMD is typically the iGPU
                    "igpu".to_string()
                } else {
                    "dgpu".to_string()
                }
            }
            Some(VENDOR_INTEL) => "igpu".to_string(),
            _ => "unknown".to_string(),
        };

        // Get runtime PM status
        let runtime_status = read_sysfs_trimmed(&dev_path.join("power/runtime_status"))
            .unwrap_or_else(|| "unsupported".to_string());

        // Build a human-readable name
        let name = resolve_gpu_name(vendor.as_deref(), &driver);

        gpus.push(GpuInfo {
            name,
            pci_slot,
            driver,
            gpu_type,
            runtime_status,
        });
    }

    // Sort: iGPU first, then dGPU
    gpus.sort_by(|a, b| {
        a.gpu_type
            .cmp(&b.gpu_type)
            .then_with(|| a.pci_slot.cmp(&b.pci_slot))
    });
    gpus
}

/// Find the first dGPU PCI slot
fn find_dgpu_path() -> Option<std::path::PathBuf> {
    let gpus = discover_gpus();
    gpus.into_iter()
        .find(|g| g.gpu_type == "dgpu")
        .map(|g| Path::new("/sys/bus/pci/devices").join(&g.pci_slot))
}

/// Check if dGPU runtime PM is set to "auto" (power saving enabled)
pub fn get_dgpu_runtime_pm() -> bool {
    if let Some(dgpu_path) = find_dgpu_path() {
        let control = read_sysfs_trimmed(&dgpu_path.join("power/control"));
        matches!(control.as_deref(), Some("auto"))
    } else {
        false
    }
}

/// Set dGPU runtime PM: true = "auto" (allow suspend), false = "on" (always active)
pub fn set_dgpu_runtime_pm(enabled: bool) -> bool {
    if let Some(dgpu_path) = find_dgpu_path() {
        let value = if enabled { "auto" } else { "on" };
        let control_path = dgpu_path.join("power/control");
        match fs::write(&control_path, value) {
            Ok(_) => {
                println!("Set dGPU runtime PM to '{}'", value);
                true
            }
            Err(e) => {
                eprintln!("Failed to write to {:?}: {}", control_path, e);
                false
            }
        }
    } else {
        eprintln!("No dGPU found for runtime PM control");
        false
    }
}

/// The installed GPU mode switcher: Ubuntu's prime-select, else envycontrol
pub fn gpu_mode_tool() -> Option<&'static str> {
    if Path::new("/usr/bin/prime-select").exists() {
        Some("prime-select")
    } else if ["/usr/bin/envycontrol", "/usr/local/bin/envycontrol"]
        .iter()
        .any(|path| Path::new(path).exists())
    {
        Some("envycontrol")
    } else {
        None
    }
}

/// The selected GPU mode (hybrid, integrated or nvidia), which applies from
/// the next login or boot, or "unknown". Querying needs no root.
pub fn get_gpu_mode(tool: &str) -> String {
    if tool == "prime-select" {
        // What `prime-select query` reads, without starting Python on every
        // poll: on, on-demand, or anything else for intel
        let mode = match fs::read_to_string("/etc/prime-discrete") {
            Ok(text) => match text.trim() {
                "on" => "nvidia",
                "on-demand" => "hybrid",
                _ => "integrated",
            },
            Err(_) => "unknown",
        };
        return mode.to_string();
    }
    let output = match Command::new(tool).arg("--query").output() {
        Ok(output) if output.status.success() => output,
        Ok(output) => {
            eprintln!(
                "{} query failed: {}",
                tool,
                String::from_utf8_lossy(&output.stderr).trim()
            );
            return "unknown".to_string();
        }
        Err(e) => {
            eprintln!("Failed to run {}: {}", tool, e);
            return "unknown".to_string();
        }
    };
    let stdout = String::from_utf8_lossy(&output.stdout).to_lowercase();
    // envycontrol prints e.g. "Current mode: hybrid"
    let mode = stdout.rsplit("mode:").next().unwrap_or("").trim();
    match mode {
        "hybrid" => "hybrid",
        "integrated" => "integrated",
        "nvidia" => "nvidia",
        _ => "unknown",
    }
    .to_string()
}

static SESSION_GPU_MODE: OnceLock<String> = OnceLock::new();

/// Remember the GPU mode selected when the daemon starts, which is when the
/// session does, as the mode the session runs
pub fn record_session_gpu_mode() {
    if let Some(tool) = gpu_mode_tool() {
        let _ = SESSION_GPU_MODE.set(get_gpu_mode(tool));
    }
}

/// The GPU mode the session runs, which differs from the selected one until
/// the next login or boot, or "unknown". A lingering user manager (e.g. an
/// SSH session spanning a logout) can leave the recorded mode stale.
pub fn session_gpu_mode() -> String {
    // Without the NVIDIA driver loaded, only the integrated GPU is in use
    if !Path::new("/sys/module/nvidia").exists() {
        return "integrated".to_string();
    }
    match SESSION_GPU_MODE.get().map(String::as_str) {
        // Selected before this boot's driver load, so not in effect yet
        Some("integrated") | None => "unknown".to_string(),
        Some(mode) => mode.to_string(),
    }
}

/// What switching to a mode needs before it takes effect
pub fn gpu_mode_applies(mode: &str) -> &'static str {
    // Integrated unloads the NVIDIA driver, which is in use until a reboot
    if mode == "integrated" {
        "Restart to apply."
    } else {
        "Log out and back in to apply."
    }
}

/// Read a sysfs file, returning trimmed content
fn read_sysfs_trimmed(path: &Path) -> Option<String> {
    fs::read_to_string(path).ok().map(|s| s.trim().to_string())
}

fn resolve_gpu_name(vendor: Option<&str>, driver: &str) -> String {
    let vendor_name = match vendor {
        Some(VENDOR_NVIDIA) => "NVIDIA",
        Some(VENDOR_AMD) => "AMD",
        Some(VENDOR_INTEL) => "Intel",
        _ => "Unknown",
    };
    if driver.is_empty() {
        format!("{} GPU", vendor_name)
    } else {
        format!("{} GPU ({})", vendor_name, driver)
    }
}
