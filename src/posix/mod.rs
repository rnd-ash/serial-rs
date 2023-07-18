//! TTY port

use std::{os::unix::prelude::RawFd, path::Path, slice, io};

use nix::{libc::{close, self}, fcntl::{OFlag, flock, FlockArg, fcntl, self}, sys::{termios::{tcgetattr, tcsetattr, tcflush, ControlFlags, LocalFlags, OutputFlags, InputFlags, cfsetospeed, cfsetispeed, BaudRate, SpecialCharacterIndices, tcflow, FlowArg, tcdrain}, time::TimeSpec, signal::SigSet}, poll::{PollFlags, PollFd}};
use crate::{SerialPortSettings, SerialResult, SerialPort, SerialError, FlowControl};

mod error;
mod ioctl;

pub mod port_lister;

/// A TTY port
#[derive(Debug, Clone)]
pub struct TTYPort {
    fd: RawFd,
    settings: SerialPortSettings,
    path: String,
}


impl TTYPort {
    /// Creates a new TTY port
    pub fn new(path: String, settings: Option<SerialPortSettings>) -> SerialResult<Self> {

        let mut flags = OFlag::O_RDWR | OFlag::O_NOCTTY;
        if !settings.unwrap_or_default().blocking {
            flags |= OFlag::O_NONBLOCK
        } 

        let fd = nix::fcntl::open(Path::new(&path), flags, nix::sys::stat::Mode::empty())?;

        let mut port = TTYPort {
            fd,
            settings: settings.unwrap_or_default(),
            path
        };

        port.reconfigure_port()?;
        if port.settings.flow_control != FlowControl::DsrDtr {
            port.set_data_terminal_ready(true)?;
        }

        if port.settings.flow_control != FlowControl::RtsCts {
            port.set_request_to_send(true)?;
        }
        port.clear_input_buffer()?;
        port.clear_output_buffer()?;
        Ok(port)
    }
}

impl super::SerialPort for TTYPort {
    fn reconfigure_port(&mut self) -> crate::SerialResult<()> {
        flock(self.fd, FlockArg::Unlock)?;
        let mut vmin: u128 = 0;
        let mut vtime: u128 = 0;

        if let Some(timeout) = self.settings.inter_byte_timeout {
            vmin = 1;
            vtime = timeout*10;
        }
        let mut orig_attr = tcgetattr(self.fd)?;

        orig_attr.control_flags |= ControlFlags::CLOCAL | ControlFlags::CREAD;
        orig_attr.local_flags &= !(
            LocalFlags::ICANON | LocalFlags::ECHO | LocalFlags::ECHOE |
            LocalFlags::ECHOK | LocalFlags::ECHONL | LocalFlags::ISIG |
            LocalFlags::IEXTEN
        );

        for flag in [LocalFlags::ECHOCTL, LocalFlags::ECHOKE] {
            orig_attr.local_flags &= !flag;
        }

        orig_attr.output_flags &= !(OutputFlags::OPOST | OutputFlags::ONLCR | OutputFlags::OCRNL);
        orig_attr.input_flags &= !(InputFlags::INLCR | InputFlags::IGNCR | InputFlags::ICRNL | InputFlags::IGNBRK);
        if orig_attr.input_flags.contains(InputFlags::PARMRK) {
            orig_attr.input_flags &= !InputFlags::PARMRK;
        }
        #[cfg(target_os="linux")]
        {
            let baud = match self.settings.baud_rate {
                50 => BaudRate::B50,
                75 => BaudRate::B75,
                110 => BaudRate::B110,
                134 => BaudRate::B134,
                150 => BaudRate::B150,
                200 => BaudRate::B200,
                300 => BaudRate::B300,
                600 => BaudRate::B600,
                1200 => BaudRate::B1200,
                1800 => BaudRate::B1800,
                2400 => BaudRate::B2400,
                4800 => BaudRate::B4800,
                9600 => BaudRate::B9600,
                19_200 => BaudRate::B19200,
                38_400 => BaudRate::B38400,
                57_600 => BaudRate::B57600,
                115_200 => BaudRate::B115200,
                230_400 => BaudRate::B230400,
                460_800 => BaudRate::B460800, 
                500_000 => BaudRate::B500000,
                576_000 => BaudRate::B576000,
                921_600 => BaudRate::B921600,
                1_000_000 => BaudRate::B1000000,
                1_152_000 => BaudRate::B1152000,
                1_500_000 => BaudRate::B1500000,
                2_000_000 => BaudRate::B2000000,
                2_500_000 => BaudRate::B2500000,
                3_000_000 => BaudRate::B3000000,
                3_500_000 => BaudRate::B3500000,
                4_000_000 => BaudRate::B4000000,
                _ => return Err(SerialError::LibraryError(format!("Baud rate {} is unsupported on NIX", self.settings.baud_rate)))
            };

            // Set baudrate
            cfsetispeed(&mut orig_attr, baud)?;
            cfsetospeed(&mut orig_attr, baud)?;
        }

        orig_attr.control_flags |= match self.settings.byte_size {
            crate::ByteSize::Five => ControlFlags::CS5,
            crate::ByteSize::Six => ControlFlags::CS6,
            crate::ByteSize::Seven => ControlFlags::CS7,
            crate::ByteSize::Eight => ControlFlags::CS8,
        };

        match self.settings.stop_bits {
            crate::StopBits::One => orig_attr.control_flags &= !(ControlFlags::CSTOPB),
            crate::StopBits::Two => orig_attr.control_flags |= ControlFlags::CSTOPB,
            crate::StopBits::OnePointFive => { return Err(SerialError::LibraryError(format!("1.5 stop bits is unsupported on NIX"))) },
        };

        orig_attr.input_flags &= !(InputFlags::INPCK | InputFlags::ISTRIP);
        // Parity

        #[cfg(not(target_os="macos"))]
        {
            orig_attr.control_flags &= !(ControlFlags::CMSPAR);
        }

        match self.settings.parity {
            crate::Parity::None => orig_attr.control_flags &= !(ControlFlags::PARENB | ControlFlags::PARODD),
            crate::Parity::Even => {
                orig_attr.control_flags &= !(ControlFlags::PARODD);
                orig_attr.control_flags |= ControlFlags::PARENB;
            },
            crate::Parity::Odd => {
                orig_attr.control_flags |= ControlFlags::PARENB | ControlFlags::PARODD;
            },
        };

        // Flow control type
        match self.settings.flow_control {
            crate::FlowControl::None | crate::FlowControl::DsrDtr => { // DSR/DTR is not supported on UNIX, use no FC in that case
                orig_attr.input_flags &= !(InputFlags::IXON | InputFlags::IXOFF | InputFlags::IXANY);
                orig_attr.control_flags &= !(ControlFlags::CRTSCTS)
            },
            crate::FlowControl::XonXoff => {
                orig_attr.input_flags |= InputFlags::IXON | InputFlags::IXOFF;
                orig_attr.control_flags &= !ControlFlags::CRTSCTS;
            },
            crate::FlowControl::RtsCts => {
                orig_attr.input_flags &= !(InputFlags::IXON | InputFlags::IXOFF | InputFlags::IXANY);
                orig_attr.control_flags |= ControlFlags::CRTSCTS;
            },
        };

        if vmin > 255 {
            return Err(SerialError::LibraryError(format!("VMIN of {vmin} is unsupported")));
        }
        orig_attr.control_chars[SpecialCharacterIndices::VMIN as usize] = vmin as u8;
        
        if vtime > 255 {
            return Err(SerialError::LibraryError(format!("VTIME of {vtime} is unsupported")));
        }
        orig_attr.control_chars[SpecialCharacterIndices::VTIME as usize] = vtime as u8;
        tcsetattr(self.fd, nix::sys::termios::SetArg::TCSANOW, &orig_attr)?;
        
        #[cfg(target_os="macos")]
        {
            ioctl::iossiospeed(self.fd, &(self.settings.baud_rate as libc::speed_t))?;
        }
        Ok(())
    }

    fn close(self) -> crate::SerialResult<()> {
        unsafe {
            close(self.fd);
        }
        Ok(())
    }

    fn set_buffer_size(&mut self, _rx_size: usize, _tx_size: usize) -> crate::SerialResult<()> {
        Ok(())
    }

    fn set_output_flow_control(&self, enable: bool) -> crate::SerialResult<()> {
        match enable {
            true => tcflow(self.fd, FlowArg::TCOON),
            false =>  tcflow(self.fd, FlowArg::TCOOFF),
        }?;
        Ok(())
    }

    fn set_data_terminal_ready(&mut self, enable: bool) -> crate::SerialResult<()> {
        unsafe { 
            match enable {
                true => ioctl::tiocmbis(self.fd, &libc::TIOCM_DTR),
                false => ioctl::tiocmbic(self.fd, &libc::TIOCM_DTR)
            }
        }?;
        Ok(())
    }

    fn set_request_to_send(&mut self, enable: bool) -> crate::SerialResult<()> {
        unsafe { 
            match enable {
                true => ioctl::tiocmbis(self.fd, &libc::TIOCM_RTS),
                false => ioctl::tiocmbic(self.fd, &libc::TIOCM_RTS)
            }
        }?;
        Ok(())
    }

    fn set_break_state(&mut self, enable: bool) -> crate::SerialResult<()> {
        unsafe { 
            match enable {
                true => ioctl::tiocsbrk(self.fd),
                false => ioctl::tioccbrk(self.fd)
            }
        }?;
        Ok(())
    }

    fn read_clear_to_send(&self) -> crate::SerialResult<bool> {
        Ok(unsafe { ioctl::tiocmget(self.fd, &mut 0) }? & libc::TIOCM_CTS != 0)
    }

    fn read_data_set_ready(&self) -> crate::SerialResult<bool> {
        Ok(unsafe { ioctl::tiocmget(self.fd, &mut 0) }? & libc::TIOCM_DSR != 0)
    }

    fn read_ring_indicator(&self) -> crate::SerialResult<bool> {
        Ok(unsafe { ioctl::tiocmget(self.fd, &mut 0) }? & libc::TIOCM_RI != 0)
    }

    fn read_carrier_detect(&self) -> crate::SerialResult<bool> {
        Ok(unsafe { ioctl::tiocmget(self.fd, &mut 0) }? & libc::TIOCM_CD != 0)
    }

    fn bytes_to_read(&self) -> crate::SerialResult<usize> {
        let mut bytes: i32 = 0;
        unsafe {ioctl::tiocinq(self.fd, &mut bytes)?};
        Ok(bytes as usize)
    }

    fn bytes_to_write(&self) -> crate::SerialResult<usize> {
        let mut bytes: i32 = 0;
        unsafe {ioctl::tiocoutq(self.fd, &mut bytes)?};
        Ok(bytes as usize)
    }

    fn get_path(&self) -> String {
        self.path.clone()
    }

    fn try_clone(&mut self) -> crate::SerialResult<Box<dyn crate::SerialPort>> {
        Ok(Box::new(TTYPort {
            fd: fcntl(self.fd, fcntl::F_DUPFD(self.fd))?,
            settings: self.settings.clone(),
            path: self.path.clone()
        }))
    }

    fn clear_input_buffer(&mut self) -> SerialResult<()> {
        tcflush(self.fd, nix::sys::termios::FlushArg::TCIFLUSH)?;
        Ok(())
    }

    fn clear_output_buffer(&mut self) -> SerialResult<()> {
        tcflush(self.fd, nix::sys::termios::FlushArg::TCIOFLUSH)?;
        Ok(())
    }
}


impl std::io::Read for TTYPort {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if let Some(timeout) = self.settings.read_timeout {
            wait_fd(self.fd, PollFlags::POLLIN, timeout)?;
        }
        nix::unistd::read(self.fd, buf).map_err(|e| {
            std::io::Error::new(io::ErrorKind::Other, format!("Read failed {}", e))
        })
    }
}

impl std::io::Write for TTYPort {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if let Some(timeout) = self.settings.write_timeout {
            wait_fd(self.fd, PollFlags::POLLOUT, timeout)?;
        }
        nix::unistd::write(self.fd, buf).map_err(|e| {
            std::io::Error::new(io::ErrorKind::Other, format!("Write failed {}", e))
        })
    }

    fn flush(&mut self) -> std::io::Result<()> {
        tcdrain(self.fd)?;
        Ok(())
    }
}

impl Drop for TTYPort {
    fn drop(&mut self) {
        unsafe {
            close(self.fd);
        }
    }
}

/// From Serialport-rs
fn wait_fd(fd: RawFd, events: PollFlags, timeout: u128) -> std::io::Result<()> {
    use nix::errno::Errno::{EIO, EPIPE};

    let mut fd = PollFd::new(fd, events);

    #[cfg(target_os = "linux")]
    let wait_res = {
        let timespec = TimeSpec::from_duration(std::time::Duration::from_millis(timeout as u64));
        nix::poll::ppoll(slice::from_mut(&mut fd), Some(timespec), SigSet::empty())
    };

    #[cfg(not(target_os = "linux"))]
    let wait_res = nix::poll::poll(slice::from_mut(&mut fd), timeout as nix::libc::c_int);

    let wait = match wait_res {
        Ok(r) => r,
        Err(e) => {return Err(io::Error::new(
            io::ErrorKind::TimedOut,
            format!("Operation failed {}", e),
        ))}
    };
    // All errors generated by poll or ppoll are already caught by the nix wrapper around libc, so
    // here we only need to check if there's at least 1 event
    if wait != 1 {
        return Err(io::Error::new(
            io::ErrorKind::TimedOut,
            "Operation timed out",
        ));
    }

    // Check the result of ppoll() by looking at the revents field
    match fd.revents() {
        Some(e) if e == events => return Ok(()),
        // If there was a hangout or invalid request
        Some(e) if e.contains(PollFlags::POLLHUP) || e.contains(PollFlags::POLLNVAL) => {
            return Err(io::Error::new(io::ErrorKind::BrokenPipe, EPIPE.desc()));
        }
        Some(_) | None => (),
    }

    Err(io::Error::new(io::ErrorKind::Other, EIO.desc()))
}
