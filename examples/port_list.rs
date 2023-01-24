use std::io::{Read, BufReader, BufRead, Write, BufWriter};

use serial_rs::{PortScanner, SerialPortSettings, FlowControl, SerialPort};

#[cfg(windows)]
use serial_rs::{windows::{port_lister, COMPort}};

#[cfg(unix)]
use serial_rs::posix::{TTYPort, port_lister};

fn main() {

    #[cfg(windows)]
    {
        let mut scanner = port_lister::COMPortLister{};
        for port in scanner.list_devices().unwrap() {
            println!("Found port:");
            println!("\tPort: {}", port.get_port());
            println!("\tDescription: {}", port.get_desc());
            println!("\tManufacturer: {}", port.get_manufacturer());
        }
    }

    #[cfg(unix)]
    {
        let mut scanner = port_lister::TTYPortScanner{};
        for port in scanner.list_devices().unwrap() {
            println!("Found port:");
            println!("\tPort: {}", port.get_port());
            println!("\tDescription: {}", port.get_desc());
            println!("\tManufacturer: {}", port.get_manufacturer());
        }
    }

    #[cfg(windows)]
    let p = COMPort::new("COM7".into(), Some(
        SerialPortSettings::default()
            .baud(115200)
            .read_timeout(Some(100))
            .write_timeout(Some(100))
            .set_flow_control(FlowControl::None)
    ));
    #[cfg(unix)]
    let p = TTYPort::new("/dev/ttyUSB0".into(), Some(
        SerialPortSettings::default()
            .baud(115200)
            .read_timeout(Some(100))
            .write_timeout(Some(100))
            .set_flow_control(FlowControl::None)
    ));
    match p {
        Ok(mut port) => {
            let clone_r = port.try_clone().unwrap();
            let mut clone_w = port.try_clone().unwrap();
            println!("Port open OK!");
            let test_msg: &[u8] = "#07E11092\n".as_bytes();
            let mut buf_reader = BufReader::new(clone_r);
            let mut b = String::new();
            loop {
                if buf_reader.read_line(&mut b).is_ok() {
                    print!("IN : {}", b);
                    b.clear();
                    println!("OUT: {:02X?}", test_msg);
                    if let Err(e) = clone_w.write(test_msg) {
                        eprintln!("Write error {}", e)
                    }
                } else {
                    std::thread::sleep(std::time::Duration::from_millis(50));
                }
            }
        },
        Err(e) => {
            eprintln!("Cannot open com port {}", e)
        }
    }
}