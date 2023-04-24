use crate::gear_client::RECV_TIMEOUT;
use crossbeam_channel::{Receiver, RecvTimeoutError, Sender};
use futures::{SinkExt, StreamExt};
use gear_connector_api::{utils::*, VcmiCommand, VcmiReply};
use std::net::SocketAddr;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::Arc;
use tokio::net::TcpListener;

#[derive(Debug)]
pub struct VcmiServer {
    need_stop: Arc<AtomicBool>,
    listener: TcpListener,
    vcmi_command_sender: Sender<VcmiCommand>,
    vcmi_reply_receiver: Receiver<VcmiReply>,
}

impl VcmiServer {
    pub async fn new(
        need_stop: Arc<AtomicBool>,
        address: SocketAddr,
        vcmi_command_sender: Sender<VcmiCommand>,
        vcmi_reply_receiver: Receiver<VcmiReply>,
    ) -> Self {
        tracing::debug!("Create Server");
        Self {
            need_stop,
            listener: TcpListener::bind(address)
                .await
                .expect("Can't bind gear-connector address"),
            vcmi_command_sender,
            vcmi_reply_receiver,
        }
    }
}
impl VcmiServer {
    pub async fn run(&mut self) -> std::io::Result<()> {
        let (stream, _addr) = self.listener.accept().await?;
        // let stream = Arc::new(RwLock::new(stream));
        let (mut read_stream, mut write_stream) = wrap_to_command_read_reply_write(stream);
        tracing::info!("Connected");

        let vcmi_command_sender = self.vcmi_command_sender.clone();
        let vcmi_reply_receiver = self.vcmi_reply_receiver.clone();
        let need_stop_clone = self.need_stop.clone();

        tokio::spawn(async move {
            while !need_stop_clone.load(Relaxed) {
                let command = match read_stream.next().await {
                    Some(data) => data.expect("Can't parse"),
                    None => continue,
                };

                vcmi_command_sender
                    .send(command)
                    .expect("Can't send command to Logic. Maybe thread crashed incorrectly");
            }
        });
        let need_stop_clone = self.need_stop.clone();
        tokio::spawn(async move {
            while !need_stop_clone.load(Relaxed) {
                let reply = vcmi_reply_receiver.recv_timeout(RECV_TIMEOUT);
                match reply {
                    Ok(reply) => {
                        match &reply {
                            VcmiReply::AllLoaded { archives } => {
                                tracing::info!(
                                    "Send Reply to VCMI: AllLoaded len: {}",
                                    archives.len()
                                );
                            }
                            _ => tracing::info!("Send Reply to VCMI: {:?}", reply),
                        }

                        write_stream
                            .send(reply)
                            .await
                            .expect("Cant' send VcmiReply");
                    }
                    Err(error) if error == RecvTimeoutError::Timeout => {}
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
