use std::io::{BufReader, BufRead};

use serial_rs::{posix::{port_lister, TTYPort}, PortScanner, SerialPortSettings};

fn main() {
    let mut scanner = port_lister::TTYPortScanner{};
    for port in scanner.list_devices().unwrap() {
        println!("Found port:");
        println!("\tPort: {}", port.get_port());
        println!("\tDescription: {}", port.get_desc());
        println!("\tManufacturer: {}", port.get_manufacturer());
    }

    match TTYPort::new("/dev/cu.Bluetooth-Incoming-Port".into(), Some(
        SerialPortSettings::default()
            .baud(115200)
            .read_timeout(Some(100))
    )) {
        Ok(mut port) => {
            println!("Port open OK!");
            let mut buf_reader = BufReader::new(&mut port);
            let mut b = String::new();
            loop {
                if buf_reader.read_line(&mut b).is_ok() {
                    print!("{}", b);
                    b.clear();
                }
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
        },
        Err(e) => {
            eprintln!("Cannot open com port {}", e)
        }
    }
}
