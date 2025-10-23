use regex::Regex;
use std::error::Error;
use std::fs::{self, OpenOptions};
use std::io::{self, Read, Write};
use std::path::Path;
use std::time::Duration;

const GPU_HWMON_BASE: &str = "/sys/class/drm/card1/device/hwmon/hwmon2";

enum Command {
    SetPowerLimit(u32),
    GetPowerLimit,
    GetPowerMax,
    GetPowerMin,
    Test(String),
    Unknown(String),
}

struct CommandPatterns {
    set_power_limit: Regex,
    get_power_limit: Regex,
    get_power_max: Regex,
    get_power_min: Regex,
    test_cmd: Regex,
}

fn main() {
    let port_name = "/dev/ttyUSB0";
    let baud_rate = 115200;

    let patterns = CommandPatterns {
        set_power_limit: Regex::new(r"^setPowerLimit:\s*([0-9]+)\s*$").unwrap(),
        get_power_limit: Regex::new(r"^getPowerLimit$").unwrap(),
        get_power_max: Regex::new(r"^getPowerMax$").unwrap(),
        get_power_min: Regex::new(r"^getPowerMin$").unwrap(),
        test_cmd: Regex::new(r"^test(?:\s+(.*))?$").unwrap(),
    };

    let port = serialport::new(port_name, baud_rate)
        .timeout(Duration::from_millis(10))
        .open();

    match port {
        Ok(mut port) => {
            let mut serial_buf: Vec<u8> = vec![0; 1000];
            println!("Receiving data on {} at {} baud:", &port_name, &baud_rate);
            loop {
                //match port.read_to_string(&mut buffer) {
                match port.read(serial_buf.as_mut_slice()) {
                    Ok(t) => {
                        let recieved = String::from_utf8_lossy(&serial_buf[..t]);
                        // println!("DEBUG: Recieved raw: {:?}", recieved);

                        let cmd = parse_command(&recieved, &patterns);
                        execute_command(cmd, &mut port);
                    }
                    Err(ref e) if e.kind() == io::ErrorKind::TimedOut => (),
                    Err(e) => eprintln!("{:?}", e),
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to open \"{}\". Error: {}", port_name, e);
            ::std::process::exit(1);
        }
    }
}

fn execute_command(cmd: Command, port: &mut dyn Write) {
    match cmd {
        Command::SetPowerLimit(microwatts) => {
            // let microwatts = watts * 1_000_000;
            println!("Setting GPU power limit to {} µW...", microwatts);
            match set_gpu_power_limit(microwatts) {
                Ok(_) => println!("Successfully set GPU power limit."),
                Err(e) => eprintln!("Failed to set GPU power limit: {}", e),
            }
        }
        Command::GetPowerLimit => match get_gpu_power_limit() {
            Ok(s) => {
                write!(port, "{}\r\n", s).unwrap();
            }
            Err(e) => {
                write!(port, "Failed to read power_cap ({})", e).unwrap();
            }
        },
        Command::GetPowerMax => match get_gpu_power_max() {
            Ok(s) => {
                write!(port, "{}\r\n", s).unwrap();
            }
            Err(e) => {
                write!(port, "Failed to read power_cap ({})", e).unwrap();
            }
        },
        Command::GetPowerMin => match get_gpu_power_min() {
            Ok(s) => {
                write!(port, "{}\r\n", s).unwrap();
            }
            Err(e) => {
                write!(port, "Failed to read power_cap ({})", e).unwrap();
            }
        },
        Command::Test(arg) => {
            println!("Recieved test argument: {}", arg);
        }
        Command::Unknown(s) => {
            io::stdout().write_all(s.as_bytes()).unwrap();
            io::stdout().flush().unwrap();
        }
    }
}
fn parse_command(line: &str, pat: &CommandPatterns) -> Command {
    let trimmed = line.trim();
    // :println!("DEBUG: Trimmed raw: {:?}", trimmed);
    if let Some(caps) = pat.set_power_limit.captures(trimmed) {
        if let Some(watts_str) = caps.get(1) {
            if let Ok(watts) = watts_str.as_str().parse::<u32>() {
                return Command::SetPowerLimit(watts);
            }
        }
    } else if let Some(caps) = pat.get_power_limit.captures(trimmed) {
        _ = caps;
        return Command::GetPowerLimit;
    } else if let Some(caps) = pat.get_power_max.captures(trimmed) {
        _ = caps;
        return Command::GetPowerMax;
    } else if let Some(caps) = pat.get_power_min.captures(trimmed) {
        _ = caps;
        return Command::GetPowerMin;
    } else if let Some(caps) = pat.test_cmd.captures(trimmed) {
        let arg = caps
            .get(1)
            .map(|m| m.as_str().to_string())
            .unwrap_or_default();
        return Command::Test(arg);
    }

    Command::Unknown(line.to_string())
}
pub fn get_gpu_power_limit() -> Result<String, Box<dyn Error>> {
    let path = Path::new(GPU_HWMON_BASE).join("power1_cap");
    if !path.exists() {
        return Err(format!("Path not found: {}", path.display()).into());
    }

    let power_cap = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;

    // Trim any trailing newline and return
    Ok(power_cap.trim().to_string())
}

pub fn get_gpu_power_max() -> Result<String, Box<dyn Error>> {
    let path = Path::new(GPU_HWMON_BASE).join("power1_cap_max");
    if !path.exists() {
        return Err(format!("Path not found: {}", path.display()).into());
    }

    let power_max = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;

    // Trim any trailing newline and return
    Ok(power_max.trim().to_string())
}

pub fn get_gpu_power_min() -> Result<String, Box<dyn Error>> {
    let path = Path::new(GPU_HWMON_BASE).join("power1_cap_max");
    if !path.exists() {
        return Err(format!("Path not found: {}", path.display()).into());
    }

    let power_min = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;

    // Trim any trailing newline and return
    Ok(power_min.trim().to_string())
}

pub fn set_gpu_power_limit(value: u32) -> Result<(), Box<dyn Error>> {
    // println!("DEBUG: Entered set_gpu_power_limit");
    // You may need to adjust this path if your GPU uses card0 or hwmon0
    // let path = Path::new("/sys/class/drm/card1/device/hwmon/hwmon2/power1_cap");
    let path = Path::new(GPU_HWMON_BASE).join("power1_cap");

    // Check that the file exists
    if !path.exists() {
        return Err(format!("Path not found: {}", path.display()).into());
    }

    // Attempt to open the file for writing
    let mut file = match OpenOptions::new().write(true).open(&path) {
        Ok(f) => f,
        Err(e) => {
            return Err(format!(
                "Failed to open {}: {} (try running as root with sudo)",
                path.display(),
                e
            )
            .into());
        }
    };

    // Write the value
    _ = writeln!(file, "{}", value);

    Ok(())
}
