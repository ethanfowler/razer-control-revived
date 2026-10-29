use nvml_wrapper::Nvml;
use nvml_wrapper::enum_wrappers::device::{Clock, TemperatureSensor};
use std::cell::Cell;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const NVIDIA_VENDOR_ID: &str = "0x10de";

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct NvidiaTelemetry {
    pub temperature: Option<f64>,
    pub power: Option<f64>,
    pub utilization: Option<u32>,
    /// Power limit the driver is currently enforcing, in watts.
    pub power_limit: Option<f64>,
    pub graphics_clock_mhz: Option<u32>,
    pub pstate: Option<u32>,
}

/// Every NVML query counts as GPU activity, so querying on each poll keeps
/// an idle GPU from ever powering down. While it's awake it's queried at
/// most this often, leaving it time to sleep in between.
const NVML_INTERVAL: Duration = Duration::from_secs(15);

/// For about half a second after waking, the driver reports a 590.01 W
/// placeholder. Laptop GPUs draw well under this, so a reading at or above
/// it is discarded and retried at the next poll.
const MAX_PLAUSIBLE_WATTS: f64 = 500.0;

thread_local! {
    // Loaded lazily; stays None on systems without libnvidia-ml.
    static NVML: Option<Nvml> = Nvml::init().ok();
    // The last NVML reading and when it was taken
    static LAST_NVML: Cell<Option<(Instant, NvidiaTelemetry)>> = const { Cell::new(None) };
}

/// Read NVIDIA telemetry without invoking vendor tools.
///
/// Running `nvidia-smi` can load the NVIDIA kernel module or wake a runtime-
/// suspended dGPU. The GUI polls this function regularly, so only devices
/// explicitly reported as active are read: from sysfs hwmon where the driver
/// provides it (nouveau), with NVML filling in what the proprietary driver
/// only exposes there (power limit, clocks, and usually everything else).
/// NVML readings can be up to NVML_INTERVAL old.
pub fn read_nvidia_telemetry() -> NvidiaTelemetry {
    let sysfs = Path::new("/sys");
    let devices = active_nvidia_devices(sysfs);
    let mut telemetry = devices
        .iter()
        .find_map(|device| read_device_telemetry(device))
        .unwrap_or_default();
    if let Some(slot) = devices
        .first()
        .and_then(|device| device.file_name())
        .and_then(|name| name.to_str())
    {
        let nvml = match LAST_NVML.get() {
            Some((taken, reading)) if taken.elapsed() < NVML_INTERVAL => reading,
            last => {
                let reading = read_nvml_telemetry(slot);
                if reading
                    .power
                    .is_some_and(|watts| watts >= MAX_PLAUSIBLE_WATTS)
                {
                    last.map(|(_, reading)| reading).unwrap_or_default()
                } else {
                    LAST_NVML.set(Some((Instant::now(), reading)));
                    reading
                }
            }
        };
        telemetry.temperature = telemetry.temperature.or(nvml.temperature);
        telemetry.power = telemetry.power.or(nvml.power);
        telemetry.utilization = telemetry.utilization.or(nvml.utilization);
        telemetry.power_limit = nvml.power_limit;
        telemetry.graphics_clock_mhz = nvml.graphics_clock_mhz;
        telemetry.pstate = nvml.pstate;
    }
    telemetry
}

fn read_nvml_telemetry(pci_slot: &str) -> NvidiaTelemetry {
    NVML.with(|nvml| {
        let Some(device) = nvml
            .as_ref()
            .and_then(|nvml| nvml.device_by_pci_bus_id(pci_slot).ok())
        else {
            return NvidiaTelemetry::default();
        };
        NvidiaTelemetry {
            temperature: device
                .temperature(TemperatureSensor::Gpu)
                .ok()
                .map(f64::from),
            power: device.power_usage().ok().map(|mw| f64::from(mw) / 1_000.0),
            utilization: device.utilization_rates().ok().map(|u| u.gpu),
            power_limit: device
                .enforced_power_limit()
                .ok()
                .map(|mw| f64::from(mw) / 1_000.0),
            graphics_clock_mhz: device.clock_info(Clock::Graphics).ok(),
            pstate: device
                .performance_state()
                .ok()
                .map(|state| state.as_c())
                .filter(|state| *state <= 15),
        }
    })
}

#[cfg(test)]
fn read_nvidia_telemetry_from(sysfs: &Path) -> NvidiaTelemetry {
    active_nvidia_devices(sysfs)
        .into_iter()
        .find_map(|device| read_device_telemetry(&device))
        .unwrap_or_default()
}

fn active_nvidia_devices(sysfs: &Path) -> Vec<PathBuf> {
    if !sysfs.join("module/nvidia").is_dir() {
        return Vec::new();
    }

    let Ok(devices) = fs::read_dir(sysfs.join("bus/pci/devices")) else {
        return Vec::new();
    };

    let mut devices = devices
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            let vendor_is_nvidia = fs::read_to_string(path.join("vendor"))
                .is_ok_and(|vendor| vendor.trim().eq_ignore_ascii_case(NVIDIA_VENDOR_ID));
            let driver_is_nvidia = fs::read_link(path.join("driver"))
                .ok()
                .and_then(|driver| driver.file_name().map(|name| name == "nvidia"))
                .unwrap_or(false);
            let is_active = fs::read_to_string(path.join("power/runtime_status"))
                .is_ok_and(|status| status.trim() == "active");

            (vendor_is_nvidia && driver_is_nvidia && is_active).then_some(path)
        })
        .collect::<Vec<_>>();
    devices.sort();
    devices
}

fn read_device_telemetry(device: &Path) -> Option<NvidiaTelemetry> {
    let mut hwmon = fs::read_dir(device.join("hwmon"))
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .collect::<Vec<_>>();
    hwmon.sort();

    let temperature = hwmon
        .iter()
        .find_map(|path| read_number::<f64>(&path.join("temp1_input")))
        .map(|value| value / 1_000.0);
    let power = hwmon
        .iter()
        .flat_map(|path| {
            ["power1_average", "power1_input"]
                .map(|name| path.join(name))
                .into_iter()
        })
        .find_map(|path| read_number::<f64>(&path))
        .map(|value| value / 1_000_000.0);
    let utilization =
        read_number::<u32>(&device.join("gpu_busy_percent")).filter(|value| *value <= 100);

    let telemetry = NvidiaTelemetry {
        temperature,
        power,
        utilization,
        ..NvidiaTelemetry::default()
    };
    (telemetry != NvidiaTelemetry::default()).then_some(telemetry)
}

fn read_number<T: std::str::FromStr>(path: &Path) -> Option<T> {
    fs::read_to_string(path).ok()?.trim().parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::os::unix::fs::symlink;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_ID: AtomicU64 = AtomicU64::new(0);

    struct TestSysfs(PathBuf);

    impl TestSysfs {
        fn new() -> Self {
            let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "razer-control-gpu-monitor-{}-{id}",
                std::process::id()
            ));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }

        fn add_nvidia_gpu(
            &self,
            slot: &str,
            runtime_status: Option<&str>,
            bound: bool,
            temperature_millicelsius: Option<u32>,
            power_microwatts: Option<u64>,
            utilization: Option<u32>,
        ) {
            let module = self.0.join("module/nvidia");
            let device = self.0.join("bus/pci/devices").join(slot);
            fs::create_dir_all(&module).unwrap();
            fs::create_dir_all(device.join("power")).unwrap();
            fs::write(device.join("vendor"), "0x10de\n").unwrap();
            if let Some(status) = runtime_status {
                fs::write(device.join("power/runtime_status"), status).unwrap();
            }
            if bound {
                symlink(&module, device.join("driver")).unwrap();
            }

            if temperature_millicelsius.is_some() || power_microwatts.is_some() {
                let hwmon = device.join("hwmon/hwmon0");
                fs::create_dir_all(&hwmon).unwrap();
                if let Some(value) = temperature_millicelsius {
                    fs::write(hwmon.join("temp1_input"), value.to_string()).unwrap();
                }
                if let Some(value) = power_microwatts {
                    fs::write(hwmon.join("power1_average"), value.to_string()).unwrap();
                }
            }
            if let Some(value) = utilization {
                fs::write(device.join("gpu_busy_percent"), value.to_string()).unwrap();
            }
        }
    }

    impl Drop for TestSysfs {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn returns_no_telemetry_when_kernel_module_is_unloaded() {
        let sysfs = TestSysfs::new();
        assert_eq!(
            read_nvidia_telemetry_from(sysfs.path()),
            NvidiaTelemetry::default()
        );
    }

    #[test]
    fn fails_closed_for_unbound_or_indeterminate_devices() {
        let sysfs = TestSysfs::new();
        sysfs.add_nvidia_gpu(
            "0000:01:00.0",
            Some("active"),
            false,
            Some(50_000),
            None,
            None,
        );
        sysfs.add_nvidia_gpu("0000:02:00.0", None, true, Some(51_000), None, None);

        assert_eq!(
            read_nvidia_telemetry_from(sysfs.path()),
            NvidiaTelemetry::default()
        );
    }

    #[test]
    fn ignores_suspended_and_transitional_devices() {
        let sysfs = TestSysfs::new();
        for (index, status) in ["suspended", "suspending", "resuming"].iter().enumerate() {
            sysfs.add_nvidia_gpu(
                &format!("0000:0{}:00.0", index + 1),
                Some(status),
                true,
                Some(50_000),
                None,
                None,
            );
        }

        assert_eq!(
            read_nvidia_telemetry_from(sysfs.path()),
            NvidiaTelemetry::default()
        );
    }

    #[test]
    fn reads_sysfs_metrics_from_an_active_nvidia_gpu() {
        let sysfs = TestSysfs::new();
        sysfs.add_nvidia_gpu(
            "0000:01:00.0",
            Some("active"),
            true,
            Some(51_000),
            Some(12_750_000),
            Some(34),
        );

        assert_eq!(
            read_nvidia_telemetry_from(sysfs.path()),
            NvidiaTelemetry {
                temperature: Some(51.0),
                power: Some(12.75),
                utilization: Some(34),
                ..NvidiaTelemetry::default()
            }
        );
    }

    #[test]
    fn skips_a_suspended_gpu_and_reads_an_active_one() {
        let sysfs = TestSysfs::new();
        sysfs.add_nvidia_gpu(
            "0000:01:00.0",
            Some("suspended"),
            true,
            Some(90_000),
            None,
            None,
        );
        sysfs.add_nvidia_gpu(
            "0000:02:00.0",
            Some("active"),
            true,
            Some(48_000),
            None,
            None,
        );

        assert_eq!(
            read_nvidia_telemetry_from(sysfs.path()).temperature,
            Some(48.0)
        );
    }

    #[test]
    fn rejects_out_of_range_utilization() {
        let sysfs = TestSysfs::new();
        sysfs.add_nvidia_gpu("0000:01:00.0", Some("active"), true, None, None, Some(101));

        assert_eq!(
            read_nvidia_telemetry_from(sysfs.path()),
            NvidiaTelemetry::default()
        );
    }

    #[test]
    fn reads_utilization_without_hwmon() {
        let sysfs = TestSysfs::new();
        sysfs.add_nvidia_gpu("0000:01:00.0", Some("active"), true, None, None, Some(42));

        assert_eq!(
            read_nvidia_telemetry_from(sysfs.path()),
            NvidiaTelemetry {
                temperature: None,
                power: None,
                utilization: Some(42),
                ..NvidiaTelemetry::default()
            }
        );
    }

    #[test]
    fn selects_the_lowest_active_pci_slot_deterministically() {
        let sysfs = TestSysfs::new();
        sysfs.add_nvidia_gpu(
            "0000:02:00.0",
            Some("active"),
            true,
            Some(60_000),
            None,
            None,
        );
        sysfs.add_nvidia_gpu(
            "0000:01:00.0",
            Some("active"),
            true,
            Some(45_000),
            None,
            None,
        );

        assert_eq!(
            read_nvidia_telemetry_from(sysfs.path()).temperature,
            Some(45.0)
        );
    }
}
