use embedded_can::{ExtendedId, Frame as _, Id};
use serial_can::{Bitrate, Frame, Open, Setup, Transmit};
use std::env;

fn main() -> std::io::Result<()> {
    let args: Vec<String> = env::args().collect();

    // Connect to the serial port.
    let mut serial = serialport::new(&args[0], args[1].parse().unwrap())
        .open()
        .expect("Failed to open serial port");

    // Configure the CAN device bitrate.
    serial.write(&format!("{}", Setup::new(Bitrate::Rate500kbit)).as_bytes())?;

    // Open the connection.
    serial.write(&format!("{}", Open::new()).as_bytes())?;

    // Send a single frame.
    let frame = Frame::new(
        Id::Extended(ExtendedId::new(0x1234).unwrap()),
        &[0, 1, 2, 3, 4, 5, 6, 7],
    )
    .unwrap();
    serial.write(&format!("{}", Transmit::new(&frame)).as_bytes())?;

    Ok(())
}
