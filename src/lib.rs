//! serial-rs is a cross-platform serial library
//! A lot of the code here is based on the [Pyserial project](https://github.com/pyserial/pyserial)

#![deny(
    missing_docs,
    missing_debug_implementations,
    missing_copy_implementations,
    dead_code,
    while_true
)]
use std::io::ErrorKind;

#[allow(unused)]
const XON: i8 = 17;
#[allow(unused)]
const XOFF: i8 = 19;
#[allow(unused)]
const CR: i8 = 13;
#[allow(unused)]
const LF: i8 = 10;

#[cfg(unix)]
pub mod posix;

#[cfg(windows)]
pub mod windows;

/// Serial port result type
pub type SerialResult<T> = std::result::Result<T, SerialError>;

/// Serial port error type
pub enum SerialError {
    /// IO Error
    IoError(std::io::Error),
    /// OS Specific error
    OsError {
        /// OS Error code
        code: u32,
        /// OS Error description
        desc: String,
    },
    /// Internal library error
    LibraryError(String)
}

impl std::fmt::Debug for SerialError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IoError(arg0) => f.debug_tuple("IoError").field(arg0).finish(),
            Self::OsError { code, desc } => f
                .debug_struct("OsError")
                .field("code", code)
                .field("desc", desc)
                .finish(),
            SerialError::LibraryError(e) => f.debug_tuple("LibraryError").field(e).finish(),
        }
    }
}

impl std::fmt::Display for SerialError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SerialError::IoError(e) => {
                write!(f, "IoError {}", e)
            }
            SerialError::OsError { code, desc } => write!(f, "OsError {code} ({desc})"),
            SerialError::LibraryError(e) => write!(f, "Serial-RS Lib error '{e}'"),
        }
    }
}

impl std::error::Error for SerialError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        if let Self::IoError(e) = self {
            Some(e)
        } else {
            None
        }
    }
}

/// Serial port settings
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SerialPortSettings {
    baud_rate: u32,
    byte_size: ByteSize,
    parity: Parity,
    stop_bits: StopBits,
    read_timeout: Option<u128>,
    flow_control: FlowControl,
    write_timeout: Option<u128>,
    inter_byte_timeout: Option<u128>,
    blocking: bool
}

impl Default for SerialPortSettings {
    fn default() -> Self {
        Self {
            baud_rate: 9600,
            byte_size: ByteSize::Eight,
            parity: Parity::None,
            stop_bits: StopBits::One,
            read_timeout: None,
            write_timeout: None,
            flow_control: FlowControl::None,
            inter_byte_timeout: None,
            blocking: true
        }
    }
}

#[allow(missing_docs)]
impl SerialPortSettings {
    /// Set baud rate
    pub fn baud(mut self, baud: u32) -> Self {
        self.baud_rate = baud;
        self
    }

    pub fn read_timeout(mut self, timeout: Option<u128>) -> Self {
        self.read_timeout = timeout;
        self
    }

    pub fn byte_size(mut self, byte_size: ByteSize) -> Self {
        self.byte_size = byte_size;
        self
    }

    pub fn write_timeout(mut self, timeout: Option<u128>) -> Self {
        self.write_timeout = timeout;
        self
    }

    pub fn parity(mut self, parity: Parity) -> Self {
        self.parity = parity;
        self
    }

    pub fn stop_bits(mut self, stop_bits: StopBits) -> Self {
        self.stop_bits = stop_bits;
        self
    }

    pub fn set_flow_control(mut self, method: FlowControl) -> Self {
        self.flow_control = method;
        self
    }

    pub fn set_blocking(mut self, blocking: bool) -> Self {
        self.blocking = blocking;
        self
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
/// Flow control method
pub enum FlowControl {
    /// No flow control
    None,
    /// DSR DTR flow control (Software)
    DsrDtr,
    /// XON XOFF flow control (Software)
    XonXoff,
    /// CTS RTS flow control (Hardware)
    RtsCts
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
/// Bytesize for serial port
pub enum ByteSize {
    /// 5 bits
    Five,
    /// 6 bits
    Six,
    /// 7 bits
    Seven,
    /// 8 bits
    Eight,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
/// Parity definitions
pub enum Parity {
    /// No parity
    None,
    /// Even parity
    Even,
    /// Odd parity
    Odd,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
/// Stop bits for serial port
pub enum StopBits {
    /// 1 stop bit
    One,
    /// 1.5 stop bits
    OnePointFive,
    /// 2 stop bits
    Two,
}

/// Information on a listed serial port
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct PortInfo {
    /// Name of the device
    port: String,
    /// Hardware-ID of the device
    hwid: String,
    /// Vendor ID
    vid: u16,
    /// Product ID
    pid: u16,
    /// Manufacturer
    manufacturer: String,
    /// Description of the device
    description: String,
}

impl PortInfo {
    /// Gets port name
    pub fn get_port(&self) -> &str { &self.port }
    /// Gets port system hardware-ID
    pub fn get_hwid(&self) -> &str { &self.hwid }
    /// Gets port devices' ProductID
    pub fn get_pid(&self) -> u16 { self.pid }
    /// Gets port devices' VendorID
    pub fn get_vid(&self) -> u16 { self.vid }
    /// Gets port devices' manufacturer
    pub fn get_manufacturer(&self) -> &str { &self.manufacturer }
    /// Gets port devices' description
    pub fn get_desc(&self) -> &str { &self.description }
}

/// Serial port trait
pub trait SerialPort: Send + std::io::Write + std::io::Read {
    /// Make the serial port Settings reconfigurable
    fn setting(&mut self) -> &mut SerialPortSettings;
    /// Reconfigures an open port with the current settings
    fn reconfigure_port(&mut self) -> SerialResult<()>;
    /// Closes the port
    fn close(self) -> SerialResult<()>;
    /// Sets Tx and Rx buffer size. A sensible value for these is 4096 bytes
    fn set_buffer_size(&mut self, rx_size: usize, tx_size: usize) -> SerialResult<()>;
    /// Sets flow control state manually
    fn set_output_flow_control(&self, enable: bool) -> SerialResult<()>;
    /// Sets data terminal flag
    fn set_data_terminal_ready(&mut self, enable: bool) -> SerialResult<()>;
    /// Sets request to send flag
    fn set_request_to_send(&mut self, enable: bool) -> SerialResult<()>;
    /// Sets break state flag
    fn set_break_state(&mut self, enable: bool) -> SerialResult<()>;
    /// Reads clear to send flag
    fn read_clear_to_send(&self) -> SerialResult<bool>;
    /// Reads data set ready flag
    fn read_data_set_ready(&self) -> SerialResult<bool>;
    /// Reads ring indicator flag
    fn read_ring_indicator(&self) -> SerialResult<bool>;
    /// Reads carrier detect flag
    fn read_carrier_detect(&self) -> SerialResult<bool>;
    /// Returns number of bytes left to read in serial buffer
    fn bytes_to_read(&self) -> SerialResult<usize>;
    /// Returns number of bytes left to write in serial buffer
    fn bytes_to_write(&self) -> SerialResult<usize>;
    /// Gets the path of the port
    fn get_path(&self) -> String;
    /// Tries to clone the port.
    /// 
    /// # Note about cloning
    /// You must be careful when cloning a port as this can have interesting
    /// effects. For example, if one thread tries to close the port but another
    /// thread wants the port open
    fn try_clone(&mut self) -> SerialResult<Box<dyn SerialPort>>;
    /// Clears serial input buffer
    fn clear_input_buffer(&mut self) -> SerialResult<()>;
    /// Clears serial output buffer
    fn clear_output_buffer(&mut self) -> SerialResult<()>;
}

/// Scanner to list avaliable serial ports on a system
pub trait PortScanner {
    /// Lists avaliable serial ports on a system
    fn list_devices(&mut self) -> SerialResult<Vec<PortInfo>>;
}

impl From<SerialError> for std::io::Error {
    fn from(e: SerialError) -> Self {
        match e {
            SerialError::IoError(i) => i,
            SerialError::OsError { code: _ , desc } => std::io::Error::new(ErrorKind::Other, desc),
            SerialError::LibraryError(e) => std::io::Error::new(ErrorKind::Other, e),
        }
    }
}

/// Creates a new serial port from port info
pub fn new(info: PortInfo, settings: Option<SerialPortSettings>) -> SerialResult<Box<dyn SerialPort>> {
    #[cfg(unix)]
    {
        use posix::*;
        Ok(Box::new(TTYPort::new(info.port, settings)?))
    }
    #[cfg(windows)]
    {
        use windows::*;
        Ok(Box::new(COMPort::new(info.port, settings)?))
    }
}

/// Creates a new serial port from port path
pub fn new_from_path(path: &str, settings: Option<SerialPortSettings>) -> SerialResult<Box<dyn SerialPort>> {
    #[cfg(unix)]
    {
        use posix::*;
        Ok(Box::new(TTYPort::new(path.to_string(), settings)?))
    }
    #[cfg(windows)]
    {
        use windows::*;
        Ok(Box::new(COMPort::new(path.to_string(), settings)?))
    }
}

/// Lists all ports on the system
pub fn list_ports() -> SerialResult<Vec<PortInfo>> {
    #[cfg(unix)]
    {
        use posix::port_lister::TTYPortScanner;
        TTYPortScanner{}.list_devices()
    }
    #[cfg(windows)]
    {
        use windows::port_lister::COMPortLister;
        COMPortLister{}.list_devices()
    }
}
