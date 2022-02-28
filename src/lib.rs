//! pyserial-rs is a cross-platform serial library
//! based on the [Pyserial project](https://github.com/pyserial/pyserial)

#![deny(
    missing_docs,
    missing_debug_implementations,
    missing_copy_implementations,
    unused
)]

const XON: i8 = 17;
const XOFF: i8 = 19;
//const CR: i8 = 13;
//const LF: i8 = 10;

#[cfg(unix)]
mod posix;

#[cfg(windows)]
mod windows;

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
pub struct SerialPortState {
    baud_rate: u32,
    byte_size: ByteSize,
    parity: Parity,
    stop_bits: StopBits,
    timeout: Option<u128>,
    xon_xoff: bool,
    rts_cts: bool,
    write_timeout: Option<u128>,
    dsr_dtr: bool,
    inter_byte_timeout: Option<u128>,
    rs485_mode: bool,
    rts_state: bool,
    dtr_state: bool,
    break_state: bool,
}

impl Default for SerialPortState {
    fn default() -> Self {
        Self {
            baud_rate: 9600,
            byte_size: ByteSize::Eight,
            parity: Parity::None,
            stop_bits: StopBits::One,
            timeout: None,
            xon_xoff: false,
            rts_cts: false,
            write_timeout: None,
            dsr_dtr: false,
            inter_byte_timeout: None,
            rs485_mode: false,
            rts_state: true,
            dtr_state: true,
            break_state: false,
        }
    }
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
    /// Mark parity
    Mark,
    /// Space parity
    Space,
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
    name: String,
    /// Hardware-ID of the device
    hwid: String,
    /// Vendor ID
    vid: u16,
    /// Product ID
    pid: u16,
    /// Serial number of the device
    serial_number: String,
    /// Location of the device
    location: String,
    /// Manufacturer
    manufacturer: String,
    /// Product name
    product: String,
    /// Interface type
    interface: String,
    /// Subsystem device is using
    subsystem: String,
}

/// Serial port trait
pub trait SerialPort: Send + std::io::Write + std::io::Read {
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
}

/// Scanner to list avaliable serial ports on a system
pub trait PortScanner {
    /// Lists avaliable serial ports on a system
    fn list_devices(&mut self) -> Vec<PortInfo>;
}
