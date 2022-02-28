//! Serial port layer for POSIX systems

use std::path::PathBuf;

use glob::glob;

use crate::PortInfo;

// Posix port scanner
pub struct PosixPortScanner {}

impl super::PortScanner for PosixPortScanner {
    fn list_devices(&mut self) -> Vec<crate::PortInfo> {
        glob("/dev/ttyS*")
            .unwrap() // Built in serial ports
            .chain(glob("/dev/ttyUSB*").unwrap()) // USB-Serial with its own driver
            .chain(glob("/dev/ttyXRUSB*").unwrap()) // XR-USB serial port
            .chain(glob("/dev/ttyACM*").unwrap()) // USB-Serial with CDC-ACM profile
            .chain(glob("/dev/ttyAMA*").unwrap()) // ARM internal port
            .chain(glob("/dev/rfcomm*").unwrap()) // BT Serial
            .chain(glob("/dev/ttyAP*").unwrap()) // Advantech multi-port serial
            .chain(glob("/dev/ttyGS*").unwrap()) // Gadget serial
            .map(|port_name| {
                if let Ok(path) = port_name {
                    let mut port_info = PortInfo::default();
                    port_info.name = path.to_str().unwrap().to_string();
                    let name = path.iter().last().unwrap();

                    if PathBuf::from(format!("/sys/class/tty/{:?}/device", name)).exists() {}

                    Some(port_info)
                } else {
                    None
                }
            })
            .filter(|x| x.is_some())
            .map(|x| x.unwrap())
            .collect()
    }
}

#[cfg(test)]
pub mod unix_tests {
    use crate::PortScanner;

    use super::PosixPortScanner;

    #[test]
    pub fn list_ports() {
        let mut scanner = PosixPortScanner {};
        println!("{:#?}", scanner.list_devices());
    }
}
