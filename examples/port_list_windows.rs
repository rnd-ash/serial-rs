use std::io::{Read, BufReader, BufRead};

use pyserial_rs::{windows::{port_lister, COMPort}, PortScanner, SerialPortState};


fn main() {
    let mut scanner = port_lister::COMPortLister{};
    for port in scanner.list_devices().unwrap() {
        println!("Found port:");
        println!("\tPort: {}", port.get_port());
        println!("\tDescription: {}", port.get_desc());
        println!("\tManufacturer: {}", port.get_manufacturer());
    }

    match COMPort::new("COM4".into(), Some(
        SerialPortState::default()
            .baud(115200)
            .read_timeout(Some(100))
            .dsr_dtr(false)
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