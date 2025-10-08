use std::io::{self, Write};
use std::process::Command;
use std::time::Duration;

fn main() {
    let port_name = "/dev/ttyUSB0";
    let baud_rate = 115200;
    let test = "test";

    let nvidia = Command::new("/bin/bash").arg("nvidia-smi").output();

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
                        let trim = recieved.trim();
                        if test == trim {
                            io::stdout().write_all(b"hello!\n").unwrap();
                            io::stdout().write_all(&nvidia.stdout).unwrap();
                        }
                        io::stdout().write_all(&serial_buf[..t]).unwrap();
                        io::stdout().flush().unwrap();
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
