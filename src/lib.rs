//! A Rust Serial Line CAN (slcan) library. Useful for embedded systems.

#![cfg_attr(not(test), no_std)]

mod frame;

use core::fmt::{Debug, Display};
use embedded_can::{Frame as _, Id};
pub use frame::Frame;

/// Bitrate options.
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum Bitrate {
    Rate10kbit = 0,
    Rate20kbit = 1,
    Rate50kbit = 2,
    Rate100kbit = 3,
    Rate125kbit = 4,
    Rate250kbit = 5,
    Rate500kbit = 6,
    Rate800kbit = 7,
    Rate1000kbit = 8,
}

/// Setup port command.
#[derive(Debug)]
pub struct Setup {
    pub bitrate: Bitrate,
}

impl Setup {
    pub fn new(bitrate: Bitrate) -> Self {
        Self { bitrate }
    }
}

impl Display for Setup {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "S{:}\r", self.bitrate as u8)
    }
}

/// Open port command.
pub struct Open {}

impl Open {
    pub fn new() -> Self {
        Self {}
    }
}

impl Display for Open {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "O\r")
    }
}

/// Close port command.
pub struct Close {}

impl Close {
    pub fn new() -> Self {
        Self {}
    }
}

impl Display for Close {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "C\r")
    }
}

/// Transmit frame command.
pub struct Transmit {
    frame: Frame,
}

impl Transmit {
    pub fn new(frame: &impl embedded_can::Frame) -> Self {
        // Convert foreign frame to library frame.
        let frame = if frame.is_remote_frame() {
            Frame::new_remote(frame.id(), frame.dlc()).unwrap()
        } else {
            Frame::new(frame.id(), frame.data()).unwrap()
        };

        Self { frame }
    }
}

impl Display for Transmit {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let cmd = match (self.frame.is_extended(), self.frame.is_remote_frame()) {
            (false, false) => 't',
            (true, false) => 'T',
            (true, true) => 'R',
            (false, true) => 'r',
        };

        match self.frame.id() {
            Id::Standard(id) => write!(f, "{}{:03X}", cmd, id.as_raw())?,
            Id::Extended(id) => write!(f, "{}{:08X}", cmd, id.as_raw())?,
        }

        write!(f, "{}", self.frame.dlc())?;

        if self.frame.is_data_frame() {
            for byte in self.frame.data() {
                write!(f, "{:02X}", *byte)?;
            }
        }

        write!(f, "\r")?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use embedded_can::{ExtendedId, StandardId};

    #[test]
    fn format_setup() {
        let setup = Setup::new(Bitrate::Rate10kbit);
        assert_eq!(format!("{}", setup), "S0\r");
    }

    #[test]
    fn format_open() {
        let open = Open::new();
        assert_eq!(format!("{}", open), "O\r");
    }

    #[test]
    fn format_close() {
        let close = Close::new();
        assert_eq!(format!("{}", close), "C\r");
    }

    #[test]
    fn format_transmit() {
        let frame = Frame::new(Id::Standard(StandardId::new(0x123).unwrap()), &[]).unwrap();
        let transmit = Transmit::new(&frame);
        assert_eq!(format!("{}", transmit), "t1230\r");

        let frame = Frame::new(
            Id::Standard(StandardId::new(0x456).unwrap()),
            &[0x11, 0x22, 0x33],
        )
        .unwrap();
        let transmit = Transmit::new(&frame);
        assert_eq!(format!("{}", transmit), "t4563112233\r");

        let frame = Frame::new(
            Id::Extended(ExtendedId::new(0x12ABCDEF).unwrap()),
            &[0xAA, 0x55],
        )
        .unwrap();
        let transmit = Transmit::new(&frame);
        assert_eq!(format!("{}", transmit), "T12ABCDEF2AA55\r");

        let frame = Frame::new_remote(Id::Standard(StandardId::new(0x123).unwrap()), 0).unwrap();
        let transmit = Transmit::new(&frame);
        assert_eq!(format!("{}", transmit), "r1230\r");
    }
}
