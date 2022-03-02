use nix::errno::{Errno};

use crate::{SerialError};


impl From<nix::errno::Errno> for SerialError {
    fn from(e: Errno) -> SerialError {
        SerialError::OsError {
            code: e as u32,
            desc:e.desc().to_string()
        }
    }
}

