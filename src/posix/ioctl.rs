use nix::{ioctl_none_bad, libc, ioctl_read_bad, ioctl_write_ptr_bad, ioctl_read, ioctl_write_ptr};


ioctl_none_bad!(tiocexcl, libc::TIOCEXCL);
ioctl_none_bad!(tiocnxcl, libc::TIOCNXCL);
ioctl_read_bad!(tiocmget, libc::TIOCMGET, libc::c_int);
ioctl_none_bad!(tiocsbrk, libc::TIOCSBRK);
ioctl_none_bad!(tioccbrk, libc::TIOCCBRK);
ioctl_read_bad!(fionread, libc::FIONREAD, libc::c_int);
ioctl_read_bad!(tiocoutq, libc::TIOCOUTQ, libc::c_int);
ioctl_write_ptr_bad!(tiocmbic, libc::TIOCMBIC, libc::c_int);
ioctl_write_ptr_bad!(tiocmbis, libc::TIOCMBIS, libc::c_int);
ioctl_read!(tcgets2, b'T', 0x2A, libc::termios);
ioctl_write_ptr!(tcsets2, b'T', 0x2B, libc::termios2);
