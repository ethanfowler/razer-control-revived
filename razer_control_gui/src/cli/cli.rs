#[path = "../comms.rs"]
mod comms;
use clap::{CommandFactory, Parser, Subcommand, ValueEnum, error::ErrorKind};

#[derive(Parser)]
#[command(
    version = "0.5.0",
    about = "razer laptop configuration for linux",
    name = "razer-cli"
)]
struct Cli {
    #[command(subcommand)]
    args: Args,
}

#[derive(Subcommand)]
enum Args {
    /// Read the current configuration of the device for some attribute
    Read {
        #[command(subcommand)]
        attr: ReadAttr,
    },
    /// Write a new configuration to the device for some attribute
    Write {
        #[command(subcommand)]
        attr: WriteAttr,
    },
    /// Write a standard effect
    StandardEffect {
        #[command(subcommand)]
        effect: StandardEffect,
    },
    /// Write a custom effect
    Effect {
        #[command(subcommand)]
        effect: Effect,
    },
    /// Print live status as one JSON object per line, for scripts and the
    /// GNOME extension, until stdout closes
    Monitor {
        /// Milliseconds between lines
        #[arg(long, default_value_t = 2000, value_parser = clap::value_parser!(u64).range(200..))]
        interval_ms: u64,
    },
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum OnOff {
    On,
    Off,
}

impl OnOff {
    pub fn is_on(&self) -> bool {
        matches!(self, Self::On)
    }
}

#[derive(Subcommand)]
enum ReadAttr {
    /// Read the current fan speed
    Fan(AcStateParam),
    /// Read the current power mode
    Power(AcStateParam),
    /// Read the current brightness
    Brightness(AcStateParam),
    /// Read the current logo mode
    Logo(AcStateParam),
    /// Read the current sync mode
    Sync,
    /// Read the current bho mode
    Bho,
    /// Read actual fan RPM from hardware
    FanRpm,
    /// Read GPU status information
    Gpu,
    /// Read the CPU power limits for a profile
    CpuLimit(AcStateParam),
    /// Read whether NVIDIA Dynamic Boost (nvidia-powerd) is running
    DynamicBoost,
    /// Read whether CPU turbo boost is on for a profile
    Turbo(AcStateParam),
    /// Read the GPU clock limits for a profile
    GpuClocks(AcStateParam),
}

#[derive(Subcommand)]
enum WriteAttr {
    /// Set the fan speed
    Fan(FanParams),
    /// Set the power mode
    Power(PowerParams),
    /// Set the brightness of the keyboard
    Brightness(BrightnessParams),
    /// Set the logo mode
    Logo(LogoParams),
    /// Set sync
    Sync(SyncParams),
    /// Set battery health optimization
    Bho(BhoParams),
    /// Set dGPU runtime power management
    RuntimePm(RuntimePmParams),
    /// Set GPU mode via prime-select or envycontrol (hybrid, integrated, nvidia)
    GpuMode(GpuModeParams),
    /// Fix the CPU power limits for a profile, or follow the power mode
    CpuLimit(CpuLimitParams),
    /// Start or stop NVIDIA Dynamic Boost (nvidia-powerd)
    DynamicBoost(DynamicBoostParams),
    /// Switch CPU turbo boost for a profile
    Turbo(TurboParams),
    /// Cap the GPU graphics and memory clocks for a profile
    GpuClocks(GpuClocksParams),
}

#[derive(Parser)]
struct PowerParams {
    /// battery/plugged in
    ac_state: AcState,
    /// power mode (0, 1, 2, 3 or 4)
    pwr: u8,
    /// cpu boost (0, 1, 2 or 3)
    cpu_mode: Option<u8>,
    /// gpu boost (0, 1 or 2)
    gpu_mode: Option<u8>,
}

#[derive(Parser)]
struct FanParams {
    /// battery/plugged in
    ac_state: AcState,
    /// fan speed in RPM
    speed: i32,
}

#[derive(Parser)]
struct BrightnessParams {
    /// battery/plugged in
    ac_state: AcState,
    /// brightness
    brightness: i32,
}

#[derive(Parser)]
struct LogoParams {
    /// battery/plugged in
    ac_state: AcState,
    /// logo mode (0, 1 or 2)
    logo_state: i32,
}

#[derive(Parser)]
struct SyncParams {
    sync_state: OnOff,
}

#[derive(Parser)]
struct BhoParams {
    state: OnOff,
    /// charging threshold
    threshold: Option<u8>,
}

#[derive(Parser)]
struct RuntimePmParams {
    /// on (suspend dGPU) or off (keep active)
    state: OnOff,
}

#[derive(Parser)]
struct GpuModeParams {
    /// GPU mode: hybrid, integrated, or nvidia
    mode: String,
}

#[derive(Parser)]
struct CpuLimitParams {
    /// battery/plugged in
    ac_state: AcState,
    /// sustained limit (PL1) in watts, or "auto" to follow the power mode
    pl1: String,
    /// burst limit (PL2) in watts; defaults to PL1
    pl2: Option<u32>,
}

#[derive(Parser)]
struct DynamicBoostParams {
    state: OnOff,
}

#[derive(Parser)]
struct TurboParams {
    /// battery/plugged in
    ac_state: AcState,
    state: OnOff,
}

#[derive(Parser)]
struct GpuClocksParams {
    /// battery/plugged in
    ac_state: AcState,
    /// graphics clock limit in MHz, or "off"
    graphics: String,
    /// memory clock limit in MHz, or "off"
    memory: String,
}

#[derive(ValueEnum, Clone)]
enum AcState {
    /// battery
    Bat,
    /// plugged in
    Ac,
}

impl AcState {
    fn as_index(&self) -> usize {
        match self {
            AcState::Bat => 0,
            AcState::Ac => 1,
        }
    }
}

#[derive(Parser, Clone)]
struct AcStateParam {
    /// battery/plugged in
    ac_state: AcState,
}

#[derive(Subcommand)]
enum StandardEffect {
    Off,
    Wave(WaveParams),
    Reactive(ReactiveParams),
    Breathing(BreathingParams),
    Spectrum,
    Static(StaticParams),
    Starlight(StarlightParams),
}

#[derive(Parser)]
struct WaveParams {
    /// direction (0 or 1)
    direction: u8,
}

#[derive(Parser)]
struct ReactiveParams {
    /// speed (0-255)
    speed: u8,
    /// red (0-255)
    red: u8,
    /// green (0-255)
    green: u8,
    /// blue (0-255)
    blue: u8,
}

#[derive(Parser)]
struct BreathingParams {
    /// kind (0-2)
    kind: u8,
    /// red1 (0-255)
    red1: u8,
    /// green1 (0-255)
    green1: u8,
    /// blue1 (0-255)
    blue1: u8,
    /// red2 (0-255)
    red2: u8,
    /// green2 (0-255)
    green2: u8,
    /// blue2 (0-255)
    blue2: u8,
}

#[derive(Parser)]
struct StarlightParams {
    /// kind (0-2)
    kind: u8,
    /// speed (0-255)
    speed: u8,
    /// red1 (0-255)
    red1: u8,
    /// green1 (0-255)
    green1: u8,
    /// blue1 (0-255)
    blue1: u8,
    /// red2 (0-255)
    red2: u8,
    /// green2 (0-255)
    green2: u8,
    /// blue2 (0-255)
    blue2: u8,
}

#[derive(Subcommand)]
enum Effect {
    Static(StaticParams),
    StaticGradient(StaticGradientParams),
    WaveGradient(WaveGradientParams),
    BreathingSingle(BreathingSingleParams),
}

#[derive(Parser)]
struct StaticParams {
    /// red (0-255)
    red: u8,
    /// green (0-255)
    green: u8,
    /// blue (0-255)
    blue: u8,
}

#[derive(Parser)]
struct StaticGradientParams {
    /// red1 (0-255)
    red1: u8,
    /// green1 (0-255)
    green1: u8,
    /// blue1 (0-255)
    blue1: u8,
    /// red2 (0-255)
    red2: u8,
    /// green2 (0-255)
    green2: u8,
    /// blue2 (0-255)
    blue2: u8,
}

#[derive(Parser)]
struct WaveGradientParams {
    /// red1 (0-255)
    red1: u8,
    /// green1 (0-255)
    green1: u8,
    /// blue1 (0-255)
    blue1: u8,
    /// red2 (0-255)
    red2: u8,
    /// green2 (0-255)
    green2: u8,
    /// blue2 (0-255)
    blue2: u8,
}

#[derive(Parser)]
struct BreathingSingleParams {
    /// red (0-255)
    red: u8,
    /// green (0-255)
    green: u8,
    /// blue (0-255)
    blue: u8,
    /// duration (0-255)
    duration: u8,
}

fn main() {
    if std::fs::metadata(comms::socket_path()).is_err() {
        eprintln!("Error. Socket doesn't exit. Is daemon running?");
        std::process::exit(1);
    }

    let cli = Cli::parse();

    match cli.args {
        Args::Monitor { interval_ms } => monitor(interval_ms),
        Args::Read { attr } => match attr {
            ReadAttr::Fan(AcStateParam { ac_state }) => read_fan_rpm(ac_state.as_index()),
            ReadAttr::Power(AcStateParam { ac_state }) => read_power_mode(ac_state.as_index()),
            ReadAttr::Brightness(AcStateParam { ac_state }) => read_brightness(ac_state.as_index()),
            ReadAttr::Logo(AcStateParam { ac_state }) => read_logo_mode(ac_state.as_index()),
            ReadAttr::Sync => read_sync(),
            ReadAttr::Bho => read_bho(),
            ReadAttr::FanRpm => read_actual_fan_rpm(),
            ReadAttr::Gpu => read_gpu_status(),
            ReadAttr::CpuLimit(AcStateParam { ac_state }) => read_cpu_limit(ac_state.as_index()),
            ReadAttr::DynamicBoost => read_dynamic_boost(),
            ReadAttr::Turbo(AcStateParam { ac_state }) => read_turbo(ac_state.as_index()),
            ReadAttr::GpuClocks(AcStateParam { ac_state }) => read_gpu_clocks(ac_state.as_index()),
        },
        Args::Write { attr } => match attr {
            WriteAttr::Fan(FanParams { ac_state, speed }) => {
                write_fan_speed(ac_state.as_index(), speed)
            }
            WriteAttr::Power(PowerParams {
                ac_state,
                pwr,
                cpu_mode,
                gpu_mode,
            }) => write_pwr_mode(ac_state.as_index(), pwr, cpu_mode, gpu_mode),
            WriteAttr::Brightness(BrightnessParams {
                ac_state,
                brightness,
            }) => write_brightness(ac_state.as_index(), brightness as u8),
            WriteAttr::Sync(SyncParams { sync_state }) => write_sync(sync_state.is_on()),
            WriteAttr::Logo(LogoParams {
                ac_state,
                logo_state,
            }) => write_logo_mode(ac_state.as_index(), logo_state as u8),
            WriteAttr::Bho(BhoParams { state, threshold }) => {
                validate_and_write_bho(threshold, state)
            }
            WriteAttr::RuntimePm(RuntimePmParams { state }) => write_runtime_pm(state.is_on()),
            WriteAttr::GpuMode(GpuModeParams { mode }) => write_gpu_mode(&mode),
            WriteAttr::CpuLimit(CpuLimitParams { ac_state, pl1, pl2 }) => {
                write_cpu_limit(ac_state.as_index(), &pl1, pl2)
            }
            WriteAttr::GpuClocks(GpuClocksParams {
                ac_state,
                graphics,
                memory,
            }) => write_gpu_clocks(ac_state.as_index(), &graphics, &memory),
            WriteAttr::Turbo(TurboParams { ac_state, state }) => {
                write_turbo(ac_state.as_index(), state.is_on())
            }
            WriteAttr::DynamicBoost(DynamicBoostParams { state }) => {
                write_dynamic_boost(state.is_on())
            }
        },
        Args::Effect { effect } => match effect {
            Effect::Static(params) => send_effect(
                "static".to_string(),
                vec![params.red, params.green, params.blue],
            ),
            Effect::StaticGradient(params) => send_effect(
                "static_gradient".to_string(),
                vec![
                    params.red1,
                    params.green1,
                    params.blue1,
                    params.red2,
                    params.green2,
                    params.blue2,
                ],
            ),
            Effect::WaveGradient(params) => send_effect(
                "wave_gradient".to_string(),
                vec![
                    params.red1,
                    params.green1,
                    params.blue1,
                    params.red2,
                    params.green2,
                    params.blue2,
                ],
            ),
            Effect::BreathingSingle(params) => send_effect(
                "breathing_single".to_string(),
                vec![params.red, params.green, params.blue, params.duration],
            ),
        },
        Args::StandardEffect { effect } => match effect {
            StandardEffect::Off => send_standard_effect("off".to_string(), vec![]),
            StandardEffect::Spectrum => send_standard_effect("spectrum".to_string(), vec![]),
            StandardEffect::Breathing(params) => send_standard_effect(
                "breathing".to_string(),
                vec![
                    params.kind,
                    params.red1,
                    params.green1,
                    params.blue1,
                    params.red2,
                    params.green2,
                    params.blue2,
                ],
            ),
            StandardEffect::Reactive(params) => send_standard_effect(
                "reactive".to_string(),
                vec![params.speed, params.red, params.green, params.blue],
            ),
            StandardEffect::Starlight(params) => send_standard_effect(
                "starlight".to_string(),
                vec![
                    params.kind,
                    params.speed,
                    params.red1,
                    params.green1,
                    params.blue1,
                    params.red2,
                    params.green2,
                    params.blue2,
                ],
            ),
            StandardEffect::Static(params) => send_standard_effect(
                "static".to_string(),
                vec![params.red, params.green, params.blue],
            ),
            StandardEffect::Wave(params) => {
                send_standard_effect("wave".to_string(), vec![params.direction])
            }
        },
    }
}

fn validate_and_write_bho(threshold: Option<u8>, state: OnOff) {
    match threshold {
        Some(threshold) => {
            if !valid_bho_threshold(threshold) {
                Cli::command()
                    .error(
                        ErrorKind::InvalidValue,
                        "Threshold must be multiple of 5 between 50 and 80",
                    )
                    .exit()
            }
            write_bho(state.is_on(), threshold)
        }
        None => {
            if state.is_on() {
                Cli::command()
                    .error(
                        ErrorKind::MissingRequiredArgument,
                        "Threshold is required when BHO is on",
                    )
                    .exit()
            }
            write_bho(state.is_on(), 80)
        }
    }
}

fn read_bho() {
    send_data(comms::DaemonCommand::GetBatteryHealthOptimizer()).map_or_else(
        || eprintln!("Unknown error occured when getting bho"),
        |result| {
            if let comms::DaemonResponse::GetBatteryHealthOptimizer { is_on, threshold } = result {
                match is_on {
                    true => {
                        println!(
                            "Battery health optimization is on with a threshold of {}",
                            threshold
                        );
                    }
                    false => {
                        eprintln!("Battery health optimization is off");
                    }
                }
            }
        },
    );
}

fn write_bho(on: bool, threshold: u8) {
    if !on {
        bho_toggle_off();
        return;
    }

    bho_toggle_on(threshold);
}

fn bho_toggle_on(threshold: u8) {
    if !valid_bho_threshold(threshold) {
        eprintln!("Threshold value must be a multiple of five between 50 and 80");
        return;
    }

    send_data(comms::DaemonCommand::SetBatteryHealthOptimizer {
        is_on: true,
        threshold,
    })
    .map_or_else(
        || eprintln!("Unknown error occured when toggling bho"),
        |result| {
            if let comms::DaemonResponse::SetBatteryHealthOptimizer { result } = result {
                match result {
                    true => {
                        println!(
                            "Battery health optimization is on with a threshold of {}",
                            threshold
                        );
                    }
                    false => {
                        eprintln!("Failed to turn on bho with threshold of {}", threshold);
                    }
                }
            }
        },
    );
}

fn valid_bho_threshold(threshold: u8) -> bool {
    if !threshold.is_multiple_of(5) {
        return false;
    }

    if !(50..=80).contains(&threshold) {
        return false;
    }

    true
}

fn bho_toggle_off() {
    send_data(comms::DaemonCommand::SetBatteryHealthOptimizer {
        is_on: false,
        threshold: 80,
    })
    .map_or_else(
        || eprintln!("Unknown error occured when toggling bho"),
        |result| {
            if let comms::DaemonResponse::SetBatteryHealthOptimizer { result } = result {
                match result {
                    true => {
                        println!("Successfully turned off bho");
                    }
                    false => {
                        eprintln!("Failed to turn off bho");
                    }
                }
            }
        },
    );
}

fn send_standard_effect(name: String, params: Vec<u8>) {
    match send_data(comms::DaemonCommand::SetStandardEffect { name, params }) {
        Some(comms::DaemonResponse::SetStandardEffect { result }) => {
            if result {
                println!("Effect set OK!");
            } else {
                eprintln!("Effect set FAIL!");
            }
        }
        Some(_) => eprintln!("Unexpected response from daemon!"),
        None => eprintln!("Unknown daemon error!"),
    }
}

fn send_effect(name: String, params: Vec<u8>) {
    match send_data(comms::DaemonCommand::SetEffect { name, params }) {
        Some(comms::DaemonResponse::SetEffect { result }) => {
            if result {
                println!("Effect set OK!");
            } else {
                eprintln!("Effect set FAIL!");
            }
        }
        Some(_) => eprintln!("Unexpected response from daemon!"),
        None => eprintln!("Unknown daemon error!"),
    }
}

fn send_data(opt: comms::DaemonCommand) -> Option<comms::DaemonResponse> {
    match comms::bind() {
        Some(socket) => comms::send_to_daemon(opt, socket),
        None => {
            eprintln!("Error. Cannot bind to socket");
            None
        }
    }
}

fn read_fan_rpm(ac: usize) {
    match send_data(comms::DaemonCommand::GetFanSpeed { ac }) {
        Some(comms::DaemonResponse::GetFanSpeed { rpm }) => {
            let rpm_desc: String = match rpm {
                f if f < 0 => String::from("Unknown"),
                0 => String::from("Auto (0)"),
                _ => format!("{} RPM", rpm),
            };
            println!("Current fan setting: {}", rpm_desc);
        }
        Some(_) => eprintln!("Daemon responded with invalid data!"),
        None => eprintln!("Unknown daemon error!"),
    }
}

fn read_actual_fan_rpm() {
    // Prefer the measured speed per fan; older ECs only report the setpoint.
    if let Some(comms::DaemonResponse::GetLiveFanRpm { rpms }) =
        send_data(comms::DaemonCommand::GetLiveFanRpm)
        && !rpms.is_empty()
    {
        let names = ["CPU", "GPU"];
        for (i, rpm) in rpms.iter().enumerate() {
            println!("{} fan: {} RPM", names.get(i).unwrap_or(&"Fan"), rpm);
        }
        return;
    }
    match send_data(comms::DaemonCommand::GetActualFanRpm) {
        Some(comms::DaemonResponse::GetActualFanRpm { rpm }) => {
            println!("{}", rpm);
        }
        Some(_) => eprintln!("Daemon responded with invalid data!"),
        None => eprintln!("Unknown daemon error!"),
    }
}

fn read_logo_mode(ac: usize) {
    match send_data(comms::DaemonCommand::GetLogoLedState { ac }) {
        Some(comms::DaemonResponse::GetLogoLedState { logo_state }) => {
            let logo_state_desc: &str = match logo_state {
                0 => "Off",
                1 => "On",
                2 => "Breathing",
                _ => "Unknown",
            };
            println!("Current logo setting: {}", logo_state_desc);
        }
        Some(_) => eprintln!("Daemon responded with invalid data!"),
        None => eprintln!("Unknown daemon error!"),
    }
}

fn read_power_mode(ac: usize) {
    if let Some(resp) = send_data(comms::DaemonCommand::GetPwrLevel { ac }) {
        if let comms::DaemonResponse::GetPwrLevel { pwr } = resp {
            let power_desc: &str = match pwr {
                0 => "Balanced",
                1 => "Gaming",
                2 => "Creator",
                3 => "Silent",
                4 => "Custom",
                _ => "Unknown",
            };
            println!("Current power setting: {}", power_desc);
            if pwr == 4 {
                if let Some(resp) = send_data(comms::DaemonCommand::GetCPUBoost { ac })
                    && let comms::DaemonResponse::GetCPUBoost { cpu } = resp
                {
                    let cpu_boost_desc: &str = match cpu {
                        0 => "Low",
                        1 => "Medium",
                        2 => "High",
                        3 => "Boost",
                        _ => "Unknown",
                    };
                    println!("Current CPU setting: {}", cpu_boost_desc);
                };
                if let Some(resp) = send_data(comms::DaemonCommand::GetGPUBoost { ac })
                    && let comms::DaemonResponse::GetGPUBoost { gpu } = resp
                {
                    let gpu_boost_desc: &str = match gpu {
                        0 => "Low",
                        1 => "Medium",
                        2 => "High",
                        _ => "Unknown",
                    };
                    println!("Current GPU setting: {}", gpu_boost_desc);
                };
            }
        } else {
            eprintln!("Daemon responded with invalid data!");
        }
    }
}

fn write_pwr_mode(ac: usize, pwr_mode: u8, cpu_mode: Option<u8>, gpu_mode: Option<u8>) {
    if pwr_mode > 4 {
        Cli::command()
            .error(
                ErrorKind::InvalidValue,
                "Power mode must be 0, 1, 2, 3 or 4",
            )
            .exit()
    }

    let cm = if pwr_mode == 4 {
        cpu_mode.expect("CPU mode must be provided when power mode is 4")
    } else {
        cpu_mode.unwrap_or(0)
    };

    if cm > 3 {
        Cli::command()
            .error(ErrorKind::InvalidValue, "CPU mode must be between 0 and 3")
            .exit()
    }

    let gm = if pwr_mode == 4 {
        gpu_mode.expect("GPU mode must be provided when power mode is 4")
    } else {
        gpu_mode.unwrap_or(0)
    };

    if gm > 2 {
        Cli::command()
            .error(ErrorKind::InvalidValue, "GPU mode must be between 0 and 2")
            .exit()
    }

    match send_data(comms::DaemonCommand::SetPowerMode {
        ac,
        pwr: pwr_mode,
        cpu: cm,
        gpu: gm,
    }) {
        Some(comms::DaemonResponse::SetPowerMode { result: false }) => {
            eprintln!("Daemon failed to apply the power mode");
            std::process::exit(1);
        }
        Some(_) => read_power_mode(ac),
        None => Cli::command()
            .error(
                ErrorKind::DisplayHelp,
                "An error occurred while sending the command to the daemon",
            )
            .exit(),
    }
}

fn read_brightness(ac: usize) {
    match send_data(comms::DaemonCommand::GetBrightness { ac }) {
        Some(comms::DaemonResponse::GetBrightness { result }) => {
            println!("Current brightness: {}", result);
        }
        Some(_) => eprintln!("Daemon responded with invalid data!"),
        None => eprintln!("Unknown daemon error!"),
    }
}

fn read_sync() {
    match send_data(comms::DaemonCommand::GetSync()) {
        Some(comms::DaemonResponse::GetSync { sync }) => {
            println!("Current sync: {:?}", sync);
        }
        Some(_) => eprintln!("Daemon responded with invalid data!"),
        None => eprintln!("Unknown daemon error!"),
    }
}

fn write_brightness(ac: usize, val: u8) {
    match send_data(comms::DaemonCommand::SetBrightness { ac, val }) {
        Some(_) => read_brightness(ac),
        None => eprintln!("Unknown error!"),
    }
}

fn write_fan_speed(ac: usize, x: i32) {
    match send_data(comms::DaemonCommand::SetFanSpeed { ac, rpm: x }) {
        Some(_) => read_fan_rpm(ac),
        None => eprintln!("Unknown error!"),
    }
}

fn write_logo_mode(ac: usize, x: u8) {
    match send_data(comms::DaemonCommand::SetLogoLedState { ac, logo_state: x }) {
        Some(_) => read_logo_mode(ac),
        None => eprintln!("Unknown error!"),
    }
}

fn write_sync(sync: bool) {
    match send_data(comms::DaemonCommand::SetSync { sync }) {
        Some(_) => read_sync(),
        None => eprintln!("Unknown error!"),
    }
}

fn read_gpu_status() {
    match send_data(comms::DaemonCommand::GetGpuStatus) {
        Some(comms::DaemonResponse::GetGpuStatus {
            gpus,
            dgpu_runtime_pm,
            gpu_mode,
            gpu_mode_running,
            gpu_mode_tool,
        }) => {
            println!("Detected GPUs:");
            for gpu in &gpus {
                let type_label = if gpu.gpu_type == "dgpu" {
                    "dGPU"
                } else {
                    "iGPU"
                };
                println!(
                    "  {} [{}] {} (driver: {}, status: {})",
                    type_label, gpu.pci_slot, gpu.name, gpu.driver, gpu.runtime_status
                );
            }
            println!(
                "dGPU Runtime PM: {}",
                if dgpu_runtime_pm {
                    "auto (power saving)"
                } else {
                    "on (always active)"
                }
            );
            match gpu_mode_tool {
                Some(tool) if gpu_mode != gpu_mode_running => println!(
                    "GPU mode: {} selected ({}), running {} until the next login or boot",
                    gpu_mode, tool, gpu_mode_running
                ),
                Some(tool) => println!("GPU mode: {} ({})", gpu_mode, tool),
                None => println!("GPU mode: no switcher (prime-select or envycontrol) installed"),
            }
        }
        Some(_) => eprintln!("Daemon responded with invalid data!"),
        None => eprintln!("Unknown daemon error!"),
    }
}

fn write_runtime_pm(enabled: bool) {
    match send_data(comms::DaemonCommand::SetDgpuRuntimePM { enabled }) {
        Some(comms::DaemonResponse::SetDgpuRuntimePM { result }) => {
            if result {
                println!(
                    "dGPU runtime PM set to {}",
                    if enabled {
                        "auto (power saving)"
                    } else {
                        "on (always active)"
                    }
                );
            } else {
                eprintln!("Failed to set dGPU runtime PM (permission denied?)");
            }
        }
        Some(_) => eprintln!("Daemon responded with invalid data!"),
        None => eprintln!("Unknown daemon error!"),
    }
}

fn read_cpu_limit(ac: usize) {
    match send_data(comms::DaemonCommand::GetCpuPowerLimit { ac }) {
        Some(comms::DaemonResponse::GetCpuPowerLimit { fixed, applied }) => {
            let source = if fixed.is_some() {
                "fixed"
            } else {
                "following the power mode"
            };
            match applied {
                Some((pl1, pl2)) => {
                    println!("CPU power limit: PL1 {pl1} W, PL2 {pl2} W ({source})")
                }
                None => println!("CPU power limit: not managed"),
            }
        }
        Some(_) => eprintln!("Daemon responded with invalid data!"),
        None => eprintln!("Unknown daemon error!"),
    }
}

fn write_cpu_limit(ac: usize, pl1: &str, pl2: Option<u32>) {
    let limit = if pl1 == "auto" {
        None
    } else {
        match pl1.parse::<u32>() {
            Ok(pl1) => Some((pl1, pl2.unwrap_or(pl1))),
            Err(_) => {
                eprintln!("PL1 must be a number of watts or \"auto\"");
                std::process::exit(1);
            }
        }
    };
    match send_data(comms::DaemonCommand::SetCpuPowerLimit { ac, limit }) {
        Some(comms::DaemonResponse::SetCpuPowerLimit { result: true }) => read_cpu_limit(ac),
        Some(comms::DaemonResponse::SetCpuPowerLimit { result: false }) => {
            eprintln!("Failed to set CPU power limit (PL1 5-250 W, PL2 at least PL1)")
        }
        Some(_) => eprintln!("Daemon responded with invalid data!"),
        None => eprintln!("Unknown daemon error!"),
    }
}

fn read_gpu_clocks(ac: usize) {
    match send_data(comms::DaemonCommand::GetGpuClockLimit { ac }) {
        Some(comms::DaemonResponse::GetGpuClockLimit {
            graphics, memory, ..
        }) => {
            let show =
                |cap: Option<u32>| cap.map_or("no limit".to_string(), |mhz| format!("{mhz} MHz"));
            println!("GPU graphics clock: {}", show(graphics));
            println!("GPU memory clock: {}", show(memory));
        }
        Some(_) => eprintln!("Daemon responded with invalid data!"),
        None => eprintln!("Unknown daemon error!"),
    }
}

fn parse_clock_cap(arg: &str) -> Option<u32> {
    if arg == "off" {
        return None;
    }
    match arg.parse::<u32>() {
        Ok(mhz) if mhz > 0 => Some(mhz),
        _ => {
            eprintln!("Clock limits must be a number of MHz or \"off\"");
            std::process::exit(1);
        }
    }
}

fn write_gpu_clocks(ac: usize, graphics: &str, memory: &str) {
    let graphics = parse_clock_cap(graphics);
    let memory = parse_clock_cap(memory);
    match send_data(comms::DaemonCommand::SetGpuClockLimit {
        ac,
        graphics,
        memory,
    }) {
        Some(comms::DaemonResponse::SetGpuClockLimit { result: true }) => read_gpu_clocks(ac),
        Some(comms::DaemonResponse::SetGpuClockLimit { result: false }) => {
            eprintln!(
                "Failed to set GPU clock limits (is the helper and its polkit rule installed?)"
            )
        }
        Some(_) => eprintln!("Daemon responded with invalid data!"),
        None => eprintln!("Unknown daemon error!"),
    }
}

fn read_turbo(ac: usize) {
    match send_data(comms::DaemonCommand::GetTurbo { ac }) {
        Some(comms::DaemonResponse::GetTurbo {
            available: false, ..
        }) => println!("CPU turbo: not controllable"),
        Some(comms::DaemonResponse::GetTurbo { enabled, .. }) => {
            println!("CPU turbo: {}", if enabled { "on" } else { "off" })
        }
        Some(_) => eprintln!("Daemon responded with invalid data!"),
        None => eprintln!("Unknown daemon error!"),
    }
}

fn write_turbo(ac: usize, enabled: bool) {
    match send_data(comms::DaemonCommand::SetTurbo { ac, enabled }) {
        Some(comms::DaemonResponse::SetTurbo { result: true }) => read_turbo(ac),
        Some(comms::DaemonResponse::SetTurbo { result: false }) => {
            eprintln!("Failed to set CPU turbo (is razer-helper installed?)")
        }
        Some(_) => eprintln!("Daemon responded with invalid data!"),
        None => eprintln!("Unknown daemon error!"),
    }
}

fn read_dynamic_boost() {
    match send_data(comms::DaemonCommand::GetNvidiaPowerd) {
        Some(comms::DaemonResponse::GetNvidiaPowerd {
            available: false, ..
        }) => {
            println!("Dynamic Boost: nvidia-powerd is not installed")
        }
        Some(comms::DaemonResponse::GetNvidiaPowerd { active, .. }) => {
            println!("Dynamic Boost: {}", if active { "on" } else { "off" })
        }
        Some(_) => eprintln!("Daemon responded with invalid data!"),
        None => eprintln!("Unknown daemon error!"),
    }
}

fn write_dynamic_boost(enabled: bool) {
    match send_data(comms::DaemonCommand::SetNvidiaPowerd { enabled }) {
        Some(comms::DaemonResponse::SetNvidiaPowerd { result, message }) => {
            if result {
                println!("{}", message);
            } else {
                eprintln!("{}", message);
            }
        }
        Some(_) => eprintln!("Daemon responded with invalid data!"),
        None => eprintln!("Unknown daemon error!"),
    }
}

fn write_gpu_mode(mode: &str) {
    match send_data(comms::DaemonCommand::SetGpuMode {
        mode: mode.to_string(),
    }) {
        Some(comms::DaemonResponse::SetGpuMode { result, message }) => {
            if result {
                println!("{}", message);
            } else {
                eprintln!("Failed: {}", message);
            }
        }
        Some(_) => eprintln!("Daemon responded with invalid data!"),
        None => eprintln!("Unknown daemon error!"),
    }
}

const POWER_MODES: [&str; 5] = ["Balanced", "Gaming", "Creator", "Silent", "Custom"];

/// Whether mains power is connected, from the first Mains power supply
fn on_ac() -> Option<bool> {
    std::fs::read_dir("/sys/class/power_supply")
        .ok()?
        .flatten()
        .map(|entry| entry.path())
        .find(|path| {
            std::fs::read_to_string(path.join("type")).is_ok_and(|kind| kind.trim() == "Mains")
        })
        .and_then(|path| std::fs::read_to_string(path.join("online")).ok())
        .map(|online| online.trim() == "1")
}

/// The active profile's state, as JSON. Fields the daemon can't report are null.
fn monitor_status() -> serde_json::Value {
    let ac = on_ac();
    // The daemon's profiles are indexed 0 battery, 1 AC
    let profile = usize::from(ac.unwrap_or(true));
    let mode = match send_data(comms::DaemonCommand::GetPwrLevel { ac: profile }) {
        Some(comms::DaemonResponse::GetPwrLevel { pwr }) => Some(pwr),
        _ => None,
    };
    let (cpu_limit, cpu_limit_fixed) =
        match send_data(comms::DaemonCommand::GetCpuPowerLimit { ac: profile }) {
            Some(comms::DaemonResponse::GetCpuPowerLimit { fixed, applied }) => {
                (applied, Some(fixed.is_some()))
            }
            _ => (None, None),
        };
    let fan_rpm = match send_data(comms::DaemonCommand::GetLiveFanRpm) {
        Some(comms::DaemonResponse::GetLiveFanRpm { rpms }) => Some(rpms),
        _ => None,
    };
    let fan_target = match send_data(comms::DaemonCommand::GetFanSpeed { ac: profile }) {
        Some(comms::DaemonResponse::GetFanSpeed { rpm }) => Some(rpm),
        _ => None,
    };
    serde_json::json!({
        "ac": ac,
        "power_mode": mode.and_then(|m| POWER_MODES.get(usize::from(m))),
        "cpu_limit": cpu_limit,
        "cpu_limit_fixed": cpu_limit_fixed,
        "fan_rpm": fan_rpm,
        // 0 is automatic
        "fan_target": fan_target,
    })
}

fn monitor(interval_ms: u64) {
    use std::io::Write;
    let mut stdout = std::io::stdout().lock();
    loop {
        if writeln!(stdout, "{}", monitor_status())
            .and_then(|()| stdout.flush())
            .is_err()
        {
            // The reader went away
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(interval_ms));
    }
}
