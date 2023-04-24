use std::{
    fs::File,
    sync::{
        atomic::{AtomicBool, Ordering::Relaxed},
        Arc,
    },
};

use crossbeam_channel::{Receiver, Sender};
use futures::TryStreamExt;
use ipfs_api_backend_hyper::{IpfsApi, IpfsClient as Client};

use tauri::async_runtime::RwLock;

#[derive(Debug)]
pub enum IpfsCommand {
    UploadTar { filename: String, tar: File },
    UploadData { filename: String, data: Vec<u8> },
    DownloadTar { hash: String },
    DownloadData { hash: String },
}

#[derive(Debug)]
pub enum IpfsReply {
    Uploaded { name: String, hash: String },
    Downloaded { data: Vec<u8> },
}

pub struct IpfsClient {
    need_stop: Arc<AtomicBool>,
    ipfs_reply_sender: Sender<IpfsReply>,
    ipfs_command_receiver: Receiver<IpfsCommand>,
}

impl IpfsClient {
    pub fn new(
        need_stop: Arc<AtomicBool>,
        ipfs_reply_sender: Sender<IpfsReply>,
        ipfs_command_receiver: Receiver<IpfsCommand>,
    ) -> Self {
        Self {
            need_stop,
            ipfs_reply_sender,
            ipfs_command_receiver,
        }
    }

    pub fn run(&self) -> std::io::Result<()> {
        let ipfs_command_receiver = self.ipfs_command_receiver.clone();
        let ipfs_reply_sender = self.ipfs_reply_sender.clone();
        let need_stop_clone = self.need_stop.clone();
        let ipfs_client = Arc::new(RwLock::new(Client::default()));
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {
            while !need_stop_clone.load(Relaxed) {
                let command = ipfs_command_receiver.recv();

                match command {
                    Ok(command) => {
                        tracing::debug!("Received {:?}", command);
                        match command {
                            IpfsCommand::UploadTar {
                                filename,
                                tar: file,
                            } => {
                                let result = ipfs_client
                                    .read()
                                    .await
                                    .add(file)
                                    .await
                                    .expect("Can't upload to ipfs");
                                tracing::info!(
                                    "File {filename} uploaded to IPFS. Hash: {}, Name: {}",
                                    result.hash,
                                    result.name
                                );
                                ipfs_reply_sender
                                    .send(IpfsReply::Uploaded {
                                        name: result.name,
                                        hash: result.hash,
                                    })
                                    .expect("Send error");
                            }
                            IpfsCommand::DownloadTar { hash } => {
                                match ipfs_client
                                    .read()
                                    .await
                                    .tar_cat(&hash)
                                    .map_ok(|chunk| chunk.to_vec())
                                    .try_concat()
                                    .await
                                {
                                    Ok(data) => ipfs_reply_sender
                                        .send(IpfsReply::Downloaded { data })
                                        .expect("Error in another thread"),
                                    Err(error) => panic!("Game State Is Not Found in ipfs: {}", error),
                                }
                            }
                            IpfsCommand::UploadData { filename, data } => {
                                let data = std::io::Cursor::new(data);
                                let result = ipfs_client
                                    .read()
                                    .await
                                    .add(data)
                                    .await
                                    .expect("Can't upload to ipfs");
                                tracing::info!(
                                    "File {filename} uploaded to IPFS. Hash: {}, Name: {}, Size: {}",
                                    result.hash,
                                    result.name
                                    ,result.size
                                );
                                ipfs_reply_sender
                                    .send(IpfsReply::Uploaded {
                                        name: result.name,
                                        hash: result.hash,
                                    })
                                    .expect("Send error");
                            },
                            IpfsCommand::DownloadData { hash } => {
                                match ipfs_client
                                    .read()
                                    .await
                                    .cat(&hash)
                                    .map_ok(|chunk| chunk.to_vec())
                                    .try_concat()
                                    .await
                                {
                                    Ok(data) => ipfs_reply_sender
                                        .send(IpfsReply::Downloaded { data })
                                        .expect("Error in another thread"),
                                    Err(error) => panic!("Game State Is Not Found in ipfs: {}", error),
                                }
                            },
                        }
                    }
                    Err(error) => {
                        tracing::error!("Error in another thread: {}", error);
                        need_stop_clone.store(true, Relaxed);
                    }
                }
            }
        });
        Ok(())
    }
}
