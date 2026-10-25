use serde::{Deserialize, Serialize};
use std::io::prelude::*;
use std::{env, fs, fs::File, io};

const SETTINGS_FILE: &str = "/.local/share/razercontrol/daemon.json";
const EFFECTS_FILE: &str = "/.local/share/razercontrol/effects.json";

#[derive(Serialize, Deserialize, Copy, Clone)]
pub struct PowerConfig {
    pub power_mode: u8,
    pub cpu_boost: u8,
    pub gpu_boost: u8,
    pub fan_rpm: i32,
    pub brightness: u8,
    pub logo_state: u8,
    pub screensaver: bool, // turno of keyboard light if screen is blank
    pub idle: u32,
    /// Fixed CPU power limits, overriding the ones that follow the power mode
    #[serde(default)]
    pub cpu_power_limit: Option<CpuPowerLimit>,
}

impl PowerConfig {
    pub fn new() -> PowerConfig {
        PowerConfig {
            power_mode: 0,
            cpu_boost: 1,
            gpu_boost: 0,
            fan_rpm: 0,
            brightness: 128,
            logo_state: 0,
            screensaver: false,
            idle: 0,
            cpu_power_limit: None,
        }
    }
}

/// CPU package power limits in watts.
#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq)]
pub struct CpuPowerLimit {
    pub pl1: u32, // sustained (long term)
    pub pl2: u32, // burst (short term)
}

/// CPU package power limits applied alongside each power mode. The EC modes
/// don't touch RAPL on Linux, so without these the firmware's limits apply in
/// every mode. Disabling leaves whatever limits are currently set.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct CpuPowerLimits {
    pub enabled: bool,
    pub balanced: CpuPowerLimit,
    pub gaming: CpuPowerLimit,
    pub creator: CpuPowerLimit,
    pub silent: CpuPowerLimit,
    /// Custom mode, indexed by CPU boost level (Low, Medium, High, Boost)
    pub custom: [CpuPowerLimit; 4],
}

impl Default for CpuPowerLimits {
    fn default() -> CpuPowerLimits {
        let limit = |pl1, pl2| CpuPowerLimit { pl1, pl2 };
        CpuPowerLimits {
            enabled: true,
            balanced: limit(55, 115),
            gaming: limit(120, 160),
            creator: limit(80, 157),
            silent: limit(25, 45),
            custom: [
                limit(35, 65),
                limit(55, 115),
                limit(90, 157),
                limit(120, 160),
            ],
        }
    }
}

impl CpuPowerLimit {
    /// Sanity bounds for user-set limits: below 5 W the system crawls, and
    /// PL2 is a burst allowance on top of PL1.
    pub fn is_valid(&self) -> bool {
        (5..=250).contains(&self.pl1) && (self.pl1..=250).contains(&self.pl2)
    }
}

impl CpuPowerLimits {
    /// Limits for a power mode (0 Balanced, 1 Gaming, 2 Creator, 3 Silent,
    /// 4 Custom), or None when disabled or the mode is unknown.
    pub fn for_mode(&self, power_mode: u8, cpu_boost: u8) -> Option<CpuPowerLimit> {
        if !self.enabled {
            return None;
        }
        match power_mode {
            0 => Some(self.balanced),
            1 => Some(self.gaming),
            2 => Some(self.creator),
            3 => Some(self.silent),
            4 => self.custom.get(usize::from(cpu_boost)).copied(),
            _ => None,
        }
    }

    /// Limits for a profile: its fixed limits if set, else the mode's.
    pub fn for_profile(&self, power: &PowerConfig) -> Option<CpuPowerLimit> {
        power
            .cpu_power_limit
            .or_else(|| self.for_mode(power.power_mode, power.cpu_boost))
    }
}

#[derive(Serialize, Deserialize)]
pub struct Configuration {
    pub power: [PowerConfig; 2],
    pub sync: bool,    // sync light settings between ac and battery
    pub no_light: f64, // no light bellow this percentage of battery
    pub standard_effect: u8,
    pub standard_effect_params: Vec<u8>,
    #[serde(default)]
    pub bho_on: bool,
    #[serde(default = "default_bho_threshold")]
    pub bho_threshold: u8,
    #[serde(default)]
    pub gui_effect: u8, // GUI custom effect index (0=Static, 1=StaticGradient, 2=WaveGradient, 3=Breathing)
    #[serde(default)]
    pub gui_effect_params: Vec<u8>, // GUI effect color params (RGB bytes)
    #[serde(default)]
    pub cpu_power_limits: CpuPowerLimits,
    /// Run nvidia-powerd (Dynamic Boost); None leaves the service alone
    #[serde(default)]
    pub nvidia_powerd: Option<bool>,
}

fn default_bho_threshold() -> u8 {
    80
}

impl Configuration {
    pub fn new() -> Configuration {
        Configuration {
            power: [PowerConfig::new(), PowerConfig::new()],
            sync: false,
            no_light: 0.0,
            standard_effect: 0x04, // spectrum cycling
            standard_effect_params: vec![],
            bho_on: false,
            bho_threshold: 80,
            gui_effect: 0,
            gui_effect_params: vec![],
            cpu_power_limits: CpuPowerLimits::default(),
            nvidia_powerd: None,
        }
    }

    pub fn write_to_file(&mut self) -> io::Result<()> {
        ensure_config_dir()?;
        let j: String = serde_json::to_string_pretty(&self)?;
        File::create(get_home_directory() + SETTINGS_FILE)?.write_all(j.as_bytes())?;
        Ok(())
    }

    pub fn read_from_config() -> io::Result<Configuration> {
        let str = fs::read_to_string(get_home_directory() + SETTINGS_FILE)?;
        let res: Configuration = serde_json::from_str(str.as_str())?;
        Ok(res)
    }

    pub fn write_effects_save(json: serde_json::Value) -> io::Result<()> {
        ensure_config_dir()?;
        let j: String = serde_json::to_string_pretty(&json)?;
        File::create(get_home_directory() + EFFECTS_FILE)?.write_all(j.as_bytes())?;
        Ok(())
    }

    pub fn read_effects_file() -> io::Result<serde_json::Value> {
        let str = fs::read_to_string(get_home_directory() + EFFECTS_FILE)?;
        let res: serde_json::Value = serde_json::from_str(str.as_str())?;
        Ok(res)
    }
}

fn get_home_directory() -> String {
    env::var("HOME").unwrap_or_else(|_| {
        eprintln!("WARNING: HOME environment variable not set, falling back to /tmp");
        "/tmp".to_string()
    })
}

fn ensure_config_dir() -> io::Result<()> {
    let dir = get_home_directory() + "/.local/share/razercontrol";
    fs::create_dir_all(dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cpu_power_limits_follow_mode_and_custom_boost() {
        let limits = CpuPowerLimits::default();
        assert_eq!(limits.for_mode(3, 0), Some(limits.silent));
        assert_eq!(limits.for_mode(1, 0), Some(limits.gaming));
        assert_eq!(limits.for_mode(4, 0), Some(limits.custom[0]));
        assert_eq!(limits.for_mode(4, 3), Some(limits.custom[3]));
        assert_eq!(limits.for_mode(4, 4), None);
        assert_eq!(limits.for_mode(5, 0), None);
    }

    #[test]
    fn disabled_cpu_power_limits_apply_nothing() {
        let limits = CpuPowerLimits {
            enabled: false,
            ..CpuPowerLimits::default()
        };
        assert_eq!(limits.for_mode(0, 0), None);
    }

    #[test]
    fn configs_without_cpu_power_limits_get_defaults() {
        let mut json = serde_json::to_value(Configuration::new()).unwrap();
        json.as_object_mut().unwrap().remove("cpu_power_limits");
        let config: Configuration = serde_json::from_value(json).unwrap();
        assert_eq!(config.cpu_power_limits, CpuPowerLimits::default());
    }

    #[test]
    fn fixed_cpu_power_limit_overrides_the_mode() {
        let limits = CpuPowerLimits::default();
        let mut power = PowerConfig::new();
        power.power_mode = 3;
        assert_eq!(limits.for_profile(&power), Some(limits.silent));
        let fixed = CpuPowerLimit { pl1: 15, pl2: 25 };
        power.cpu_power_limit = Some(fixed);
        assert_eq!(limits.for_profile(&power), Some(fixed));
    }

    #[test]
    fn cpu_power_limit_bounds() {
        assert!(CpuPowerLimit { pl1: 15, pl2: 25 }.is_valid());
        assert!(CpuPowerLimit { pl1: 55, pl2: 55 }.is_valid());
        assert!(!CpuPowerLimit { pl1: 2, pl2: 25 }.is_valid());
        assert!(!CpuPowerLimit { pl1: 55, pl2: 45 }.is_valid());
        assert!(!CpuPowerLimit { pl1: 55, pl2: 1000 }.is_valid());
    }

    #[test]
    fn profiles_without_fixed_cpu_power_limit_follow_the_mode() {
        let mut json = serde_json::to_value(PowerConfig::new()).unwrap();
        json.as_object_mut().unwrap().remove("cpu_power_limit");
        let power: PowerConfig = serde_json::from_value(json).unwrap();
        assert_eq!(power.cpu_power_limit, None);
    }
}
