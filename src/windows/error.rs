use std::ptr;
use winapi::{
    shared::{minwindef::DWORD, ntdef::MAKELANGID},
    um::{
        errhandlingapi::GetLastError,
        winbase::{FormatMessageW, FORMAT_MESSAGE_FROM_SYSTEM, FORMAT_MESSAGE_IGNORE_INSERTS},
        winnt::{LANG_SYSTEM_DEFAULT, SUBLANG_SYS_DEFAULT, WCHAR},
    },
};

use crate::SerialError;

#[macro_export]
/// Test macro
macro_rules! return_win_op {
    ($op:expr) => {
        match unsafe { $op } {
            0 => Err(get_win_error()),
            _ => Ok(()),
        }
    };
}

impl From<SerialError> for std::io::Error {
    fn from(e: SerialError) -> Self {
        match e {
            SerialError::IoError(i) => i,
            SerialError::OsError { .. } => todo!(),
        }
    }
}

pub(crate) fn get_win_error() -> crate::SerialError {
    let e = unsafe { GetLastError() }; // Error code

    let language_id = MAKELANGID(LANG_SYSTEM_DEFAULT, SUBLANG_SYS_DEFAULT) as DWORD;
    let mut buf = [0 as WCHAR; 2048];

    unsafe {
        let res = FormatMessageW(
            FORMAT_MESSAGE_FROM_SYSTEM | FORMAT_MESSAGE_IGNORE_INSERTS,
            ptr::null_mut(),
            e as DWORD,
            language_id as DWORD,
            buf.as_mut_ptr(),
            buf.len() as DWORD,
            ptr::null_mut(),
        );
        if res == 0 {
            let fmt_error = GetLastError();
            return SerialError::OsError {
                code: e,
                desc: format!("Unknown. FormatMessageW() failed with error {}", fmt_error),
            };
        }

        let b = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
        match String::from_utf16(&buf[..b]) {
            Ok(msg) => SerialError::OsError { code: e, desc: msg },
            Err(..) => SerialError::OsError {
                code: e,
                desc: format!("Unknown, FormatMessageW() returned invalid UTF-16 string"),
            },
        }
    }
}
