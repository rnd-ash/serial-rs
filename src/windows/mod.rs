//! Windows COM Port handler

use std::fmt::Debug;
use std::{cmp::max, io::ErrorKind};

use crate::{return_win_op, SerialPort, SerialPortState, SerialResult};
use winapi::um::fileapi::CreateFileW;
use winapi::um::ioapiset::GetOverlappedResult;
use winapi::um::synchapi::CreateEventW;
use winapi::{
    shared::{
        minwindef::{DWORD, LPVOID},
        winerror::{
            ERROR_INVALID_USER_BUFFER, ERROR_IO_PENDING, ERROR_NOT_ENOUGH_MEMORY,
            ERROR_OPERATION_ABORTED, ERROR_SUCCESS,
        },
    },
    um::{
        commapi::{
            ClearCommBreak, ClearCommError, EscapeCommFunction, GetCommModemStatus, GetCommState,
            PurgeComm, SetCommBreak, SetCommMask, SetCommState, SetCommTimeouts, SetupComm,
        },
        errhandlingapi::GetLastError,
        fileapi::{ReadFile, WriteFile, OPEN_EXISTING},
        handleapi::{CloseHandle, INVALID_HANDLE_VALUE},
        minwinbase::OVERLAPPED,
        synchapi::{ResetEvent},
        winbase::{
            CLRDTR, CLRRTS, COMMTIMEOUTS, COMSTAT, DCB, DTR_CONTROL_DISABLE, DTR_CONTROL_ENABLE,
            DTR_CONTROL_HANDSHAKE, EVENPARITY, FILE_FLAG_OVERLAPPED, MARKPARITY, MS_CTS_ON,
            MS_DSR_ON, MS_RING_ON, MS_RLSD_ON, NOPARITY, ODDPARITY, ONE5STOPBITS, ONESTOPBIT,
            PURGE_RXABORT, PURGE_RXCLEAR, PURGE_TXABORT, PURGE_TXCLEAR, RTS_CONTROL_DISABLE,
            RTS_CONTROL_ENABLE, RTS_CONTROL_HANDSHAKE, SETDTR, SETRTS, SETXOFF, SETXON,
            SPACEPARITY, TWOSTOPBITS,
        },
        winnt::{FILE_ATTRIBUTE_NORMAL, GENERIC_READ, GENERIC_WRITE, HANDLE, MAXDWORD},
    },
};

use self::error::get_win_error;

pub (crate) mod error;
pub mod port_lister;

/// Windows COM Port
pub struct COMPort {
    serial_state: super::SerialPortState,
    handle: HANDLE,
    overlapped_read: OVERLAPPED,
    overlapped_write: OVERLAPPED,
    path: String,
}

impl Debug for COMPort {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("COMPort").field("serial_state", &self.serial_state).field("path", &self.path).finish()
    }
}

unsafe impl Send for COMPort {}
unsafe impl Sync for COMPort {}

impl COMPort {
    /// Creates a new COM Port and opens it
    #[allow(unused)]
    pub fn new(path: String, initial_state: Option<SerialPortState>) -> SerialResult<Self> {
        let mut name = Vec::<u16>::with_capacity(4 + path.len() + 1);

        name.extend(r"\\.\".encode_utf16());
        name.extend(path.encode_utf16());
        name.push(0);

        let handle = unsafe {
            CreateFileW(
                name.as_ptr(),
                GENERIC_READ | GENERIC_WRITE,
                0,
                std::ptr::null_mut(),
                OPEN_EXISTING,
                FILE_ATTRIBUTE_NORMAL | FILE_FLAG_OVERLAPPED,
                std::ptr::null_mut(),
            )
        };

        if handle == INVALID_HANDLE_VALUE {
            return Err(get_win_error());
        }
        let mut overlapped_read: OVERLAPPED = unsafe { std::mem::zeroed() };
        let mut overlapped_write: OVERLAPPED = unsafe { std::mem::zeroed() };
        overlapped_read.hEvent =
            unsafe { CreateEventW(std::ptr::null_mut(), 1, 0, std::ptr::null_mut()) };
        overlapped_write.hEvent =
            unsafe { CreateEventW(std::ptr::null_mut(), 0, 0, std::ptr::null_mut()) };

        if overlapped_read.hEvent == INVALID_HANDLE_VALUE {
            return Err(get_win_error());
        }
        if overlapped_write.hEvent == INVALID_HANDLE_VALUE {
            return Err(get_win_error());
        }

        return_win_op!(SetupComm(handle, 4096, 4096))?;

        let mut ret = Self {
            serial_state: initial_state.unwrap_or_default(),
            handle,
            path,
            overlapped_read,
            overlapped_write,
        };

        ret.reconfigure_port()?;

        return_win_op!(PurgeComm(
            ret.handle,
            PURGE_TXCLEAR | PURGE_TXABORT | PURGE_RXCLEAR | PURGE_RXABORT
        ))?;
        Ok(ret)
    }

    fn get_comm_modem_status(&self) -> DWORD {
        let mut stat: DWORD = 0;
        unsafe { GetCommModemStatus(self.handle, &mut stat) };
        return stat;
    }
}

impl super::SerialPort for COMPort {
    fn reconfigure_port(&mut self) -> SerialResult<()> {
        // First set timeouts
        let mut timeouts: COMMTIMEOUTS = unsafe { std::mem::zeroed() };
        if let Some(timeout) = self.serial_state.timeout {
            if timeout == 0 {
                timeouts.ReadIntervalTimeout = MAXDWORD;
            } else {
                timeouts.ReadTotalTimeoutConstant = max(timeout as u32, 1);
            }
            if timeout != 0 && self.serial_state.inter_byte_timeout.is_some() {
                timeouts.ReadIntervalTimeout = max(
                    self.serial_state.inter_byte_timeout.unwrap() as u32,
                    1,
                );
            }
        }

        if let Some(timeout) = self.serial_state.write_timeout {
            if timeout == 0 {
                timeouts.WriteTotalTimeoutConstant = MAXDWORD;
            } else {
                timeouts.WriteTotalTimeoutConstant = max(timeout as u32, 1);
            }
        }
        return_win_op!(SetCommTimeouts(self.handle, &mut timeouts))?;
        return_win_op!(SetCommMask(self.handle, 0x0080))?;

        // Setup DCB
        let mut dcb: DCB = unsafe { std::mem::zeroed() };
        return_win_op!(GetCommState(self.handle, &mut dcb))?;
        dcb.BaudRate = self.serial_state.baud_rate;

        dcb.ByteSize = match self.serial_state.byte_size {
            crate::ByteSize::Five => 5,
            crate::ByteSize::Six => 6,
            crate::ByteSize::Seven => 7,
            crate::ByteSize::Eight => 8,
        };

        match self.serial_state.parity {
            crate::Parity::None => {
                dcb.Parity = NOPARITY;
                dcb.set_fParity(0);
            }
            crate::Parity::Even => {
                dcb.Parity = EVENPARITY;
                dcb.set_fParity(1);
            }
            crate::Parity::Odd => {
                dcb.Parity = ODDPARITY;
                dcb.set_fParity(1);
            }
            crate::Parity::Mark => {
                dcb.Parity = MARKPARITY;
                dcb.set_fParity(1);
            }
            crate::Parity::Space => {
                dcb.Parity = SPACEPARITY;
                dcb.set_fParity(1);
            }
        }

        dcb.StopBits = match self.serial_state.stop_bits {
            crate::StopBits::One => ONESTOPBIT,
            crate::StopBits::OnePointFive => ONE5STOPBITS,
            crate::StopBits::Two => TWOSTOPBITS,
        };

        dcb.set_fBinary(1);

        if !self.serial_state.rs485_mode {
            if self.serial_state.rts_cts {
                dcb.set_fRtsControl(RTS_CONTROL_HANDSHAKE)
            } else {
                dcb.set_fRtsControl(if self.serial_state.rts_state {
                    RTS_CONTROL_ENABLE
                } else {
                    RTS_CONTROL_DISABLE
                });
            }
            dcb.set_fOutxCtsFlow(self.serial_state.rts_cts as u32);
        }

        if self.serial_state.dsr_dtr {
            dcb.set_fDtrControl(DTR_CONTROL_HANDSHAKE);
        } else {
            dcb.set_fDtrControl(if self.serial_state.dtr_state {
                DTR_CONTROL_ENABLE
            } else {
                DTR_CONTROL_DISABLE
            })
        }

        dcb.set_fOutxDsrFlow(self.serial_state.dsr_dtr as u32);
        dcb.set_fOutX(self.serial_state.xon_xoff as u32);
        dcb.set_fInX(self.serial_state.xon_xoff as u32);
        dcb.set_fNull(0);
        dcb.set_fErrorChar(0);
        dcb.set_fAbortOnError(0);
        dcb.XonChar = super::XON;
        dcb.XoffChar = super::XOFF;

        return_win_op!(SetCommState(self.handle, &mut dcb))?;
        Ok(())
    }

    fn close(self) -> SerialResult<()> {
        unsafe {
            CloseHandle(self.overlapped_read.hEvent);
            CloseHandle(self.overlapped_write.hEvent);
            CloseHandle(self.handle);
        }
        Ok(())
    }

    fn set_buffer_size(&mut self, rx_size: usize, tx_size: usize) -> SerialResult<()> {
        return_win_op!(SetupComm(self.handle, rx_size as DWORD, tx_size as DWORD))
    }

    fn set_output_flow_control(&self, enable: bool) -> SerialResult<()> {
        return_win_op!(match enable {
            true => EscapeCommFunction(self.handle, SETXON),
            false => EscapeCommFunction(self.handle, SETXOFF),
        })
    }

    fn set_data_terminal_ready(&mut self, enable: bool) -> SerialResult<()> {
        self.serial_state.dtr_state = enable;
        return_win_op!(match enable {
            true => EscapeCommFunction(self.handle, SETDTR),
            false => EscapeCommFunction(self.handle, CLRDTR),
        })
    }

    fn set_request_to_send(&mut self, enable: bool) -> SerialResult<()> {
        self.serial_state.rts_state = enable;
        return_win_op!(match enable {
            true => EscapeCommFunction(self.handle, SETRTS),
            false => EscapeCommFunction(self.handle, CLRRTS),
        })
    }

    fn set_break_state(&mut self, enable: bool) -> SerialResult<()> {
        self.serial_state.break_state = enable;
        return_win_op!(match enable {
            true => SetCommBreak(self.handle),
            false => ClearCommBreak(self.handle),
        })
    }

    fn read_clear_to_send(&self) -> SerialResult<bool> {
        Ok(MS_CTS_ON & self.get_comm_modem_status() != 0)
    }

    fn read_data_set_ready(&self) -> SerialResult<bool> {
        Ok(MS_DSR_ON & self.get_comm_modem_status() != 0)
    }

    fn read_ring_indicator(&self) -> SerialResult<bool> {
        Ok(MS_RING_ON & self.get_comm_modem_status() != 0)
    }

    fn read_carrier_detect(&self) -> SerialResult<bool> {
        Ok(MS_RLSD_ON & self.get_comm_modem_status() != 0)
    }

    fn bytes_to_read(&self) -> SerialResult<usize> {
        let mut flags: DWORD = 0;
        let mut comstat: COMSTAT = unsafe { std::mem::zeroed() };

        return_win_op!(ClearCommError(self.handle, &mut flags, &mut comstat))?;
        Ok(comstat.cbInQue as usize)
    }

    fn bytes_to_write(&self) -> SerialResult<usize> {
        let mut flags: DWORD = 0;
        let mut comstat: COMSTAT = unsafe { std::mem::zeroed() };

        return_win_op!(ClearCommError(self.handle, &mut flags, &mut comstat))?;
        Ok(comstat.cbOutQue as usize)
    }

    fn get_path(&self) -> String {
        self.path.clone()
    }
}

const VALID_PENDING_ERRORS: [DWORD; 2] = [ERROR_SUCCESS, ERROR_IO_PENDING];

impl std::io::Write for COMPort {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if buf.len() == 0 {
            return Ok(0);
        }
        let len = buf.len() as DWORD;
        let mut written: DWORD = 0;
        let success = unsafe {
            WriteFile(
                self.handle,
                buf.as_ptr() as *const winapi::ctypes::c_void,
                len,
                &mut written,
                &mut self.overlapped_write,
            )
        };
        if self.serial_state.write_timeout.is_some() {
            if success == 0 && !VALID_PENDING_ERRORS.contains(&unsafe { GetLastError() }) {
                if unsafe { GetLastError() } == ERROR_OPERATION_ABORTED {
                    return Ok(written as usize);
                } else if written != len {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::Interrupted,
                        get_win_error(),
                    ));
                }
            }
            return Ok(written as usize);
        } else {
            let error = if success != 0 {
                ERROR_SUCCESS
            } else {
                unsafe { GetLastError() }
            };
            if error == ERROR_SUCCESS || error == ERROR_IO_PENDING {
                return Ok(written as usize);
            } else {
                let e_type: std::io::ErrorKind = match error {
                    ERROR_INVALID_USER_BUFFER => ErrorKind::InvalidData,
                    ERROR_NOT_ENOUGH_MEMORY => ErrorKind::OutOfMemory,
                    _ => ErrorKind::Interrupted,
                };
                return Err(std::io::Error::new(e_type, get_win_error()));
            }
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        todo!()
    }
}

impl std::io::Read for COMPort {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if buf.len() == 0 {
            return Ok(0);
        }

        unsafe { ResetEvent(self.overlapped_read.hEvent) };

        let mut flags: DWORD = 0;
        let mut comstat: COMSTAT = unsafe { std::mem::zeroed() };
        return_win_op!(ClearCommError(self.handle, &mut flags, &mut comstat))?;

        let to_read = if self.serial_state.timeout.is_none() {
            std::cmp::min(comstat.cbInQue as usize, buf.len())
        } else {
            buf.len()
        };

        if to_read == 0 {
            // No bytes to read
            return Err(get_win_error().into());
        }
        let mut read_count: DWORD = 0;
        let read_status = unsafe {
            ReadFile(
                self.handle,
                buf.as_mut_ptr() as LPVOID,
                to_read as u32,
                &mut read_count,
                &mut self.overlapped_read,
            )
        };

        if read_count == to_read as u32 {
            return Ok(to_read);
        }

        if read_status == 0 && !VALID_PENDING_ERRORS.contains(&unsafe { GetLastError() }) {
            return Err(get_win_error().into());
        }
        let result_ok = unsafe {
            GetOverlappedResult(self.handle, &mut self.overlapped_read, &mut read_count, 1)
        };
        if result_ok == 0 {
            if unsafe { GetLastError() } != ERROR_OPERATION_ABORTED {
                return Err(get_win_error().into());
            } else {
                return Ok(read_count as usize);
            }
        }
        Ok(read_count as usize)
    }
}

impl Drop for COMPort {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.overlapped_read.hEvent);
            CloseHandle(self.overlapped_write.hEvent);
            CloseHandle(self.handle);
        }
    }
}
