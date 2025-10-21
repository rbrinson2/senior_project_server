use regex::Regex;
use std::error::Error;
use std::fs::OpenOptions;
use std::io::{self, Write};
use std::path::Path;
use std::time::Duration;

enum Command {
    PowerLimit(u32),
    Test(String),
    Unknown(String),
}

struct CommandPatterns {
    power_limit: Regex,
    test_cmd: Regex,
}

fn main() {
    let port_name = "/dev/ttyUSB0";
    let baud_rate = 115200;

    let patterns = CommandPatterns {
        power_limit: Regex::new(r"^powerLimit:\s*(\d)\s*$").unwrap(),
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

                        let cmd = parse_command(&recieved, &patterns);
                        execute_command(cmd);
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

fn execute_command(cmd: Command) {
    match cmd {
        Command::PowerLimit(watts) => {
            let microwatts = watts * 1_000_000;
            println!("Setting GPU power limit to {} µW...", microwatts);
            match set_gpu_power_limit(&microwatts.to_string()) {
                Ok(_) => println!("Successfully set GPU power limit."),
                Err(e) => eprintln!("Failed to set GPU power limit: {}", e),
            }
        }
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
    if let Some(caps) = pat.power_limit.captures(trimmed) {
        if let Some(watts_str) = caps.get(1) {
            if let Ok(watts) = watts_str.as_str().parse::<u32>() {
                return Command::PowerLimit(watts);
            }
        }
    } else if let Some(caps) = pat.test_cmd.captures(trimmed) {
        let arg = caps
            .get(1)
            .map(|m| m.as_str().to_string())
            .unwrap_or_default();
        return Command::Test(arg);
    }

    Command::Unknown(line.to_string())
}

pub fn set_gpu_power_limit(value: &str) -> Result<(), Box<dyn Error>> {
    // You may need to adjust this path if your GPU uses card0 or hwmon0
    let path = Path::new("/sys/class/drm/card1/device/hwmon/hwmon2/power1_cap");

    // Check that the file exists
    if !path.exists() {
        return Err(format!("Path not found: {}", path.display()).into());
    }

    // Attempt to open the file for writing
    let mut file = match OpenOptions::new().write(true).open(path) {
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
    if let Err(e) = writeln!(file, "{}", value.trim()) {
        return Err(format!("Failed to write power limit: {}", e).into());
    }

    Ok(())
}
