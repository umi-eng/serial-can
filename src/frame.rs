use embedded_can::Id;

/// Serial CAN frame.
#[derive(Debug)]
pub struct Frame {
    id: Id,
    remote: bool,
    dlc: u8,
    data: [u8; 8],
}

impl embedded_can::Frame for Frame {
    fn new(id: impl Into<Id>, data: &[u8]) -> Option<Self> {
        if data.len() > 8 {
            return None;
        }

        let mut data_all = [0; 8];
        data_all[0..data.len()].copy_from_slice(&data);

        Some(Self {
            id: id.into(),
            remote: false,
            dlc: data.len() as u8,
            data: data_all,
        })
    }

    fn new_remote(id: impl Into<Id>, dlc: usize) -> Option<Self> {
        if dlc > 8 {
            return None;
        }

        Some(Self {
            id: id.into(),
            remote: true,
            dlc: dlc as u8,
            data: [0; 8],
        })
    }

    fn id(&self) -> Id {
        self.id
    }

    fn dlc(&self) -> usize {
        self.dlc as usize
    }

    fn data(&self) -> &[u8] {
        &self.data[0..self.dlc()]
    }

    fn is_extended(&self) -> bool {
        match self.id {
            Id::Extended(_) => true,
            Id::Standard(_) => false,
        }
    }

    fn is_remote_frame(&self) -> bool {
        self.remote
    }
}
