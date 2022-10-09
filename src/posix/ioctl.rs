use nix::{ioctl_none_bad, libc, ioctl_read_bad, ioctl_write_ptr_bad, ioctl_read, ioctl_write_ptr};


ioctl_none_bad!(tiocexcl, libc::TIOCEXCL);
ioctl_none_bad!(tiocnxcl, libc::TIOCNXCL);
ioctl_read_bad!(tiocmget, libc::TIOCMGET, libc::c_int);
ioctl_none_bad!(tiocsbrk, libc::TIOCSBRK);
ioctl_none_bad!(tioccbrk, libc::TIOCCBRK);

#[cfg(target_os = "linux")]
ioctl_read_bad!(fionread, libc::FIONREAD, libc::c_int);

#[cfg(target_os = "macos")]
ioctl_read!(fionread, b'f', 127, libc::c_int);

#[cfg(target_os = "linux")]
ioctl_read_bad!(tiocoutq, libc::TIOCOUTQ, libc::c_int);

#[cfg(target_os = "macos")]
ioctl_read!(tiocoutq, b't', 115, libc::c_int);

ioctl_read_bad!(tiocinq, libc::TIOCINQ, libc::c_int);
ioctl_write_ptr_bad!(tiocmbic, libc::TIOCMBIC, libc::c_int);
ioctl_write_ptr_bad!(tiocmbis, libc::TIOCMBIS, libc::c_int);

#[cfg(target_os = "linux")]
ioctl_read!(tcgets2, b'T', 0x2A, libc::termios);

#[cfg(target_os = "linux")]
ioctl_write_ptr!(tcsets2, b'T', 0x2B, libc::termios2);

#[cfg(target_os = "macos")]
const IOSSIOSPEED: libc::c_ulong = 0x80045402;

#[cfg(target_os = "macos")]
ioctl_write_ptr_bad!(iossiospeed, IOSSIOSPEED, libc::speed_t);


#[cfg(target_os = "macos")]
pub fn iossiospeed(fd: RawFd, baud_rate: &libc::speed_t) -> Result<()> {
    unsafe { raw::iossiospeed(fd, baud_rate) }
        .map(|_| ())
        .map_err(|e| e.into())
}