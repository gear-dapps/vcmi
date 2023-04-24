use bytes::{Buf, BytesMut};
use serde::{Deserialize, Serialize};
use std::error::Error;
use tokio_util::codec::{Decoder, Encoder};

pub mod utils;
#[derive(Serialize, Deserialize, Hash, PartialEq, PartialOrd, Eq, Ord, Clone, Debug)]
pub struct VcmiSavedGame {
    pub filename: String,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VcmiCommand {
    ShowConnectDialog,
    ShowLoadGameDialog,
    Save {
        filename: String,
        compressed_archive: Vec<u8>,
    },
    Load(String),
    LoadAll,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VcmiReply {
    ConnectDialogShowed,
    CanceledDialog,
    Connected,

    Saved,
    Loaded { archive_data: Vec<u8> },
    AllLoaded { archives: Vec<VcmiSavedGame> },
    LoadGameDialogShowed,
}

pub struct VcmiCommandCodec;

// #[derive(Debug, Serialize, Deserialize)]
// pub enum Error {
//     Connect,
//     Save,
//     Load,
// }

impl Encoder<VcmiCommand> for VcmiCommandCodec {
    type Error = Box<dyn Error>;

    fn encode(&mut self, item: VcmiCommand, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let item = serde_json::to_vec(&item)?;
        let len_slice = u32::to_ne_bytes(item.len() as u32);

        // Reserve space in the buffer.
        dst.reserve(4 + item.len());

        // Write the length and string to the buffer.
        dst.extend_from_slice(&len_slice);
        dst.extend_from_slice(item.as_slice());
        Ok(())
    }
}

impl Decoder for VcmiCommandCodec {
    type Item = VcmiCommand;
    type Error = Box<dyn Error>;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.len() < 4 {
            // Not enough data to read length marker.
            return Ok(None);
        }

        let mut length_bytes = [0u8; 4];
        length_bytes.copy_from_slice(&src[..4]);
        let length = u32::from_le_bytes(length_bytes) as usize;

        if src.len() < 4 + length {
            // The full string has not yet arrived.
            //
            // We reserve more space in the buffer. This is not strictly
            // necessary, but is a good idea performance-wise.
            src.reserve(4 + length - src.len());

            // We inform the Framed that we need more bytes to form the next
            // frame.
            return Ok(None);
        }

        let data = src[4..4 + length].to_vec();
        src.advance(4 + length);

        let mut deserializer = serde_json::Deserializer::from_slice(&data);
        let command = VcmiCommand::deserialize(&mut deserializer)?;
        Ok(Some(command))
    }
}
