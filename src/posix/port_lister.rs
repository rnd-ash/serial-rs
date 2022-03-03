//! Linux TTY port lister and enumerator

use std::path::PathBuf;

use crate::PortInfo;

/// TTY port scanner
#[derive(Debug, Clone, Copy)]
pub struct TTYPortScanner {

}


impl TTYPortScanner {

}

fn get_paths(g: &str) -> Vec<PathBuf> {
    let mut ret: Vec<PathBuf> = vec![];
    if let Ok(res) = glob::glob(g) {
        res.for_each(|path| {
            if let Ok(p) = path {
                ret.push(p)
            }
        });
    }
    ret
}

impl crate::PortScanner for TTYPortScanner {
    fn list_devices(&mut self) -> crate::SerialResult<Vec<crate::PortInfo>> {
        let mut res: Vec<PortInfo> = vec![];
        for port in get_paths("/dev/ttyS*").into_iter()
            .chain(get_paths("/dev/ttyUSB*"))
            .chain(get_paths("/dev/ttyXRUSB*"))
            .chain(get_paths("/dev/ttyACM*"))
            .chain(get_paths("/dev/ttyAMA*"))
            .chain(get_paths("/dev/rfcomm*"))
            .chain(get_paths("/dev/ttyAP*"))
            .chain(get_paths("/dev/ttyGS*")) 
        {
            let dev_name = port.to_str().unwrap().split("/").last().unwrap();

            let mut path: Option<PathBuf> = None;
            let mut subsystem: Option<PathBuf> = None;

            if PathBuf::from(format!("/sys/class/tty/{dev_name}/device")).exists() {
                path = Some(std::fs::canonicalize(format!("/sys/class/tty/{dev_name}/device")).unwrap());
                subsystem = std::fs::canonicalize(format!("{}/subsystem", path.clone().unwrap().to_str().unwrap())).ok();
            }
            
            //let mut usb_interface_path: Option<PathBuf> = None;
            if let Some(s) = &subsystem {
                if s.to_str().unwrap().ends_with("platform") {
                    continue;
                } else if s.to_str().unwrap().ends_with("usb-serial") {
                    // TODO usb_interface_path
                } else if s.to_str().unwrap().ends_with("usb") {
                    //usb_interface_path = path;
                }
            }

            let mut port_info = PortInfo::default();
            port_info.port = port.to_string_lossy().to_string();




            println!("Dev name {} path {:?} subsystem {:?}", dev_name, path, subsystem);
            res.push(port_info);
        }
        Ok(res)
    }
}