//! Windows port lister and enumerator

use std::{ffi::CString, ptr};

use regex::{RegexBuilder};
use winapi::{um::{setupapi::{SetupDiClassGuidsFromNameA, SetupDiGetClassDevsA, DIGCF_PRESENT, SP_DEVINFO_DATA, SetupDiEnumDeviceInfo, SetupDiOpenDevRegKey, DICS_FLAG_GLOBAL, DIREG_DEV, SetupDiGetDeviceInstanceIdA, SetupDiGetDeviceRegistryPropertyA, SPDRP_HARDWAREID, SPDRP_FRIENDLYNAME, SPDRP_MFG, SetupDiDestroyDeviceInfoList}, cguid::GUID_NULL, winnt::KEY_READ, winreg::{RegQueryValueExA, RegCloseKey}}, shared::{minwindef::DWORD, guiddef::GUID, ntdef::ULONG}};

use crate::{return_win_op, windows::error::get_win_error, SerialResult, PortInfo};

#[derive(Debug, Copy, Clone)]
/// Windows COM Port lister
pub struct COMPortLister {

}

const PORT_NAME_LEN: usize = 500;

impl crate::PortScanner for COMPortLister {
    fn list_devices(&mut self) -> SerialResult<Vec<crate::PortInfo>> {
        let mut port_name_class = CString::new("Ports").unwrap();
        let mut num_guids: DWORD = 0;
        let mut guids: Vec<GUID> = Vec::new();
        guids.push(GUID_NULL);
        return_win_op!(SetupDiClassGuidsFromNameA(port_name_class.as_ptr(), guids.as_mut_ptr(), guids.len() as DWORD, &mut num_guids))?;

        if num_guids == 0 {
            guids.pop();
        }

        // Now add any modems
        port_name_class = CString::new("Modem").unwrap();
        let mut modem_guids: Vec<GUID> = Vec::new();
        modem_guids.push(GUID_NULL);
        return_win_op!(SetupDiClassGuidsFromNameA(port_name_class.as_ptr(), modem_guids.as_mut_ptr(), modem_guids.len() as DWORD, &mut num_guids))?;

        if num_guids == 0 {
            modem_guids.pop();
        }

        // Append modems to list of GUIDS
        guids.append(&mut modem_guids);
        let mut devices: Vec<PortInfo> = Vec::new();
        for mut guid in guids {
            //let mut b_interface_num: Option<u32> = None;
            let g_hdi = unsafe {
                SetupDiGetClassDevsA(&mut guid, ptr::null_mut(), ptr::null_mut(), DIGCF_PRESENT)
            };
            let mut dev_info: SP_DEVINFO_DATA = unsafe { std::mem::zeroed() };
            dev_info.cbSize = std::mem::size_of::<SP_DEVINFO_DATA>() as u32;
            let mut idx = 0;
            while unsafe { SetupDiEnumDeviceInfo(g_hdi, idx, &mut dev_info) } != 0 {
                idx += 1;

                let hkey = unsafe {
                    SetupDiOpenDevRegKey(g_hdi, &mut dev_info, DICS_FLAG_GLOBAL, 0, DIREG_DEV, KEY_READ)
                };
                let mut port_name_buffer: [u8; PORT_NAME_LEN] = [0; PORT_NAME_LEN];
                let mut port_name_len = PORT_NAME_LEN as ULONG;

                let port_name_key = CString::new("PortName").unwrap();
                unsafe { RegQueryValueExA(hkey, port_name_key.as_ptr(), ptr::null_mut(), ptr::null_mut(), port_name_buffer.as_mut_ptr(), &mut port_name_len) };
                unsafe { RegCloseKey(hkey) };

                let port_name = String::from_utf8(port_name_buffer[..port_name_len as usize].to_vec()).unwrap();

                // Discard LPT Parallel ports
                if port_name.starts_with("LPT") { continue; }
                let mut hw_id_buffer: [u8; 500] = [0; 500];
                let hw_id_len = 500 as ULONG;

                if unsafe {
                    SetupDiGetDeviceInstanceIdA(g_hdi, &mut dev_info, hw_id_buffer.as_mut_ptr() as *mut i8, hw_id_len-1, ptr::null_mut())
                } == 0 {
                    if unsafe {
                        SetupDiGetDeviceRegistryPropertyA(g_hdi, &mut dev_info, SPDRP_HARDWAREID, ptr::null_mut(), hw_id_buffer.as_mut_ptr(), hw_id_len-1, ptr::null_mut())
                    } == 0 {
                        return Err(get_win_error())
                    }
                }

                let mut tmp = String::from_utf8(hw_id_buffer.to_vec()).unwrap();
                let hw_string = tmp.trim_matches(char::from(0x00));
                let mut info = crate::PortInfo::default();
                info.port = port_name;
                if hw_string.starts_with("USB") {
                    let regex = RegexBuilder::new(r"VID_([0-9a-f]{4})(&PID_([0-9a-f]{4}))?(&MI_(\d{2}))?(\\(.*))?").case_insensitive(true).build().unwrap();
                    if let Some(captures) = regex.captures(&hw_string) {
                        info.vid = u16::from_str_radix(captures.get(1).unwrap().as_str(), 16).unwrap();
                        if let Some(m) = captures.get(3) {
                            info.pid = u16::from_str_radix(m.as_str(), 16).unwrap();
                        }
                    }
                } else if hw_string.starts_with("FTDIBUS") {
                    
                } else {
                    info.hwid = hw_string.to_string();
                }

                let mut friendly_name_buffer: [u8; 500] = [0; 500];
                let friendly_name_buffer_len = 500 as ULONG;
                if unsafe {
                    SetupDiGetDeviceRegistryPropertyA(g_hdi, &mut dev_info, SPDRP_FRIENDLYNAME, std::ptr::null_mut(), friendly_name_buffer.as_mut_ptr(), friendly_name_buffer_len-1, std::ptr::null_mut())
                } != 0 {
                    tmp = String::from_utf8(friendly_name_buffer.to_vec()).unwrap();
                    info.description = tmp.trim_matches(char::from(0x00)).to_string();
                }

                friendly_name_buffer = [0x00; 500];
                if unsafe {
                    SetupDiGetDeviceRegistryPropertyA(g_hdi, &mut dev_info, SPDRP_MFG, std::ptr::null_mut(), friendly_name_buffer.as_mut_ptr(), friendly_name_buffer_len-1, std::ptr::null_mut())
                } != 0 {
                    tmp = String::from_utf8(friendly_name_buffer.to_vec()).unwrap();
                    info.manufacturer = tmp.trim_matches(char::from(0x00)).to_string();
                }
                devices.push(info);
            }
            unsafe { SetupDiDestroyDeviceInfoList(g_hdi) };
        }
        return Ok(devices)
    }
}