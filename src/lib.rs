//! A Rust Serial Line CAN (slcan) library. Useful for embedded systems.

#![cfg_attr(not(test), no_std)]

mod frame;

use core::fmt::{Debug, Display};
use embedded_can::{ExtendedId, Frame as _, Id, StandardId};
pub use frame::Frame;
use nom::{
    branch::alt,
    bytes::complete::{tag, take},
    character::complete::{digit1, one_of},
    combinator::map,
    error::{Error, ErrorKind},
    sequence::tuple,
    Err, IResult,
};

/// Bitrate options.
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
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
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub struct Setup {
    pub bitrate: Bitrate,
}

impl Setup {
    pub fn new(bitrate: Bitrate) -> Self {
        Self { bitrate }
    }

    /// Try parsing a [`Setup`] command from a string.
    pub fn try_parse(input: &str) -> IResult<&str, Self> {
        let (input, (_, bitrate, _)) = tuple((tag("S"), digit1, tag("\r")))(input)?;

        let bitrate = match bitrate {
            "0" => Bitrate::Rate10kbit,
            "1" => Bitrate::Rate20kbit,
            "2" => Bitrate::Rate50kbit,
            "3" => Bitrate::Rate100kbit,
            "4" => Bitrate::Rate125kbit,
            "5" => Bitrate::Rate250kbit,
            "6" => Bitrate::Rate500kbit,
            "7" => Bitrate::Rate800kbit,
            "8" => Bitrate::Rate1000kbit,
            _ => return Err(Err::Failure(Error::new(input, ErrorKind::Digit))),
        };

        Ok((input, Self { bitrate }))
    }
}

impl Display for Setup {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "S{:}\r", self.bitrate as u8)
    }
}

/// Open port command.
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub struct Open {}

impl Open {
    pub fn new() -> Self {
        Self {}
    }

    /// Try parsing an [`Open`] command from a string.
    pub fn try_parse(input: &str) -> IResult<&str, Self> {
        let (input, _) = tag("O\r")(input)?;

        Ok((input, Self::new()))
    }
}

impl Display for Open {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "O\r")
    }
}

/// Close port command.
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub struct Close {}

impl Close {
    pub fn new() -> Self {
        Self {}
    }

    /// Try parsing a [`Close`] command from a string.
    pub fn try_parse(input: &str) -> IResult<&str, Self> {
        let (input, _) = tag("C\r")(input)?;

        Ok((input, Self::new()))
    }
}

impl Display for Close {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "C\r")
    }
}

/// Transmit frame command.
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
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

    /// Try parsing a [`Transmit`] command from a string.
    pub fn try_parse(input: &str) -> IResult<&str, Self> {
        let (input, kind) = one_of("tTrR")(input)?;
        let (input, id) = match kind {
            't' | 'r' => {
                let (input, id_hex) = take(3_usize)(input)?;
                let id = u16::from_str_radix(id_hex, 16)
                    .map_err(|_| Err::Failure(Error::new(input, ErrorKind::HexDigit)))?;
                (input, Id::Standard(StandardId::new(id).unwrap()))
            }
            'T' | 'R' => {
                let (input, id_hex) = take(8_usize)(input)?;
                let id = u32::from_str_radix(id_hex, 16)
                    .map_err(|_| Err::Failure(Error::new(input, ErrorKind::HexDigit)))?;
                (input, Id::Extended(ExtendedId::new(id).unwrap()))
            }
            _ => unreachable!(), // other cases are impossible due to `one_of`
        };

        let (input, dlc) = take(1_usize)(input)?;
        let dlc = usize::from_str_radix(dlc, 16)
            .map_err(|_| Err::Failure(Error::new(input, ErrorKind::HexDigit)))?;

        let (input, data) = if dlc > 0 {
            take(dlc * 2_usize)(input)?
        } else {
            (input, "")
        };

        let data = if data.is_empty() {
            [0; 8]
        } else {
            let mut array = [0; 8];
            for i in 0..dlc {
                array[i] = u8::from_str_radix(&data[i * 2..i * 2 + 2], 16)
                    .map_err(|_| Err::Failure(Error::new(input, ErrorKind::HexDigit)))?;
            }
            array
        };

        let frame = if kind == 't' || kind == 'T' {
            Frame::new(id, &data[..dlc]).unwrap()
        } else {
            Frame::new_remote(id, dlc).unwrap()
        };

        let (input, _) = tag("\r")(input)?;

        Ok((input, Self::new(&frame)))
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

/// Command variants.
enum Command {
    Setup(Setup),
    Open(Open),
    Close(Close),
    Transmit(Transmit),
}

impl Command {
    /// Try parsing a command from a string.
    pub fn try_parse(input: &str) -> IResult<&str, Self> {
        alt((
            map(Setup::try_parse, Command::Setup),
            map(Open::try_parse, Command::Open),
            map(Close::try_parse, Command::Close),
            map(Transmit::try_parse, Command::Transmit),
        ))(input)
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

    #[test]
    fn parse_transmit() {
        assert_eq!(
            Transmit::try_parse("t1230\r"),
            Ok((
                "",
                Transmit::new(
                    &Frame::new(Id::Standard(StandardId::new(0x123).unwrap()), &[]).unwrap()
                )
            ))
        );

        assert_eq!(
            Transmit::try_parse("t4563112233\r"),
            Ok((
                "",
                Transmit::new(
                    &Frame::new(
                        Id::Standard(StandardId::new(0x456).unwrap()),
                        &[0x11, 0x22, 0x33]
                    )
                    .unwrap()
                )
            ))
        );

        assert_eq!(
            Transmit::try_parse("T12ABCDEF2AA55\r"),
            Ok((
                "",
                Transmit::new(
                    &Frame::new(
                        Id::Extended(ExtendedId::new(0x12ABCDEF).unwrap()),
                        &[0xAA, 0x55]
                    )
                    .unwrap()
                )
            ))
        );

        assert_eq!(
            Transmit::try_parse("r1230\r"),
            Ok((
                "",
                Transmit::new(
                    &Frame::new_remote(Id::Standard(StandardId::new(0x123).unwrap()), 0).unwrap()
                )
            ))
        );
    }
}
