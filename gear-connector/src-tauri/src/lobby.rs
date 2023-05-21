use crossbeam_channel::{Receiver, RecvTimeoutError, Sender};
use parity_scale_codec::Output;
use std::sync::{
    atomic::{AtomicBool, Ordering::Relaxed},
    Arc,
};
use tokio::{io::AsyncWriteExt, net::TcpStream};

use crate::gear_client::RECV_TIMEOUT;

pub enum LobbyCommand {
    Connect(String),
    Greeting(u8, String, String),
    Username(String),
    Message(String),
    Create(String, String, u8, String),
    Join(String, String, String),
    Leave(String),
    Kick(String),
    Ready(String),
    ForceStart(String),
    Here,
    Alive,
    HostMode(u8),
}

pub enum LobbyReply {
    Created,
    Sessions,
    Joined,
    Kicked,
    Start,
    Host,
    Status,
    ServerError,
    Mods,
    ClientMods,
    Chat,
    Users,
    Health,
    GameMode,
}

const PROTOCOL_VERSION: u8 = 4;
const PROTOCOL_ENCODING: &str = "utf8";

pub struct LobbyClient {
    need_stop: Arc<AtomicBool>,
    connection: Option<TcpStream>,
    lobby_command_receiver: Receiver<LobbyCommand>,
    lobby_reply_sender: Sender<LobbyReply>,
    username: String,
}

impl LobbyClient {
    pub fn new(
        need_stop: Arc<AtomicBool>,
        lobby_command_receiver: Receiver<LobbyCommand>,
        lobby_reply_sender: Sender<LobbyReply>,
    ) -> Self {
        Self {
            need_stop,
            connection: None,
            lobby_command_receiver,
            lobby_reply_sender,
            username: String::new(),
        }
    }

    pub async fn run(&'static mut self) -> std::io::Result<()> {
        let lobby_command_receiver = self.lobby_command_receiver.clone();
        let lobby_reply_sender = self.lobby_reply_sender.clone();
        let need_stop = self.need_stop.clone();
        let need_stop_clone = self.need_stop.clone();
        tokio::spawn(async move {
            while !need_stop.load(Relaxed) {
                let command = lobby_command_receiver.recv_timeout(RECV_TIMEOUT);

                match command {
                    Ok(command) => {
                        self.process_command(command);
                    }
                    Err(error) if error == RecvTimeoutError::Timeout => {}
                    Err(error) => {
                        tracing::error!("Error in another thread: {}", error);
                        need_stop.store(true, Relaxed);
                    }
                }
            }
            tokio::spawn(async move {
                let mut msg = vec![0u8; 4096];
                while need_stop_clone.load(Relaxed) {
                    if let Some(stream) = &self.connection {
                        stream.readable().await.unwrap();
                        match stream.try_read(&mut msg) {
                            Ok(n) => {
                                msg.truncate(n);
                                tracing::info!(
                                    "Received from lobby: {}",
                                    String::from_utf8(msg).expect("From utf8")
                                );

                                break;
                            }
                            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                                continue;
                            }
                            Err(e) => {
                                need_stop_clone.store(true, Relaxed);
                                panic!("{}", e);
                            }
                        }
                    }
                }
            });
        });

        Ok(())
    }

    pub async fn process_command(&mut self, command: LobbyCommand) {
        match command {
            LobbyCommand::Connect(address) => self
                .connect(address)
                .await
                .expect("Can't connecto to lobby"),
            LobbyCommand::Greeting(protocol_version, username, vcmi_version) => {
                self.greetings(protocol_version, username, vcmi_version)
                    .await;
            }
            LobbyCommand::Username(username) => todo!(),
            LobbyCommand::Message(_) => todo!(),
            LobbyCommand::Create(_, _, _, _) => todo!(),
            LobbyCommand::Join(_, _, _) => todo!(),
            LobbyCommand::Leave(_) => todo!(),
            LobbyCommand::Kick(_) => todo!(),
            LobbyCommand::Ready(_) => todo!(),
            LobbyCommand::ForceStart(_) => todo!(),
            LobbyCommand::Here => todo!(),
            LobbyCommand::Alive => todo!(),
            LobbyCommand::HostMode(_) => todo!(),
        }
    }

    pub async fn connect(&mut self, address: String) -> std::io::Result<()> {
        let stream = TcpStream::connect(&address).await?;

        self.connection = Some(stream);
        Ok(())
    }

    pub async fn greetings(&mut self, username: String, vcmi_version: String) {
        self.username = username.clone();
        let command = LobbyCommand::Greeting(PROTOCOL_VERSION, username, vcmi_version).to_string();

        let command_len = command.len() as u32;
        let len_bytes = command_len.to_le_bytes();

        let encoding = PROTOCOL_ENCODING.as_bytes();

        let encoding_len = encoding.len() as u8;
        let encoding_len_bytes = encoding_len.to_le_bytes();

        let mut bytes: Vec<u8> = vec![];
        bytes.extend(&len_bytes);
        bytes.extend(encoding_len_bytes);
        bytes.extend(encoding);
        bytes.extend(command.as_bytes());

        if let Some(ref mut stream) = self.connection {
            stream
                .write_all(&bytes)
                .await
                .expect("Can't send greetings");
        }
    }

    pub fn send(&mut self, command: &LobbyCommand) {
        let command = command.to_string();

        let command_len = command.len() as u32;
        let len_bytes = command_len.to_le_bytes();

        let mut bytes: Vec<u8> = vec![];

        bytes.extend(command.as_bytes());

        if let Some(ref mut stream) = self.connection {
            stream.write(&bytes);
        }
    }
}

impl LobbyCommand {
    fn to_string(&self) -> String {
        match self {
            LobbyCommand::Greeting(protocol_version, name, vcmi_version) => format!(
                "{}<GREETINGS>{}<VER>{}",
                protocol_version, name, vcmi_version
            ),
            LobbyCommand::Username(username) => format!("<USER>{}", username),
            LobbyCommand::Message(message) => format!("<MSG>{}", message),
            LobbyCommand::Create(room_name, passwd, max_players, mods) => format!(
                "<NEW>{}<PSWD>{}<COUNT>{}<MODS>{}",
                room_name, passwd, max_players, mods
            ),
            LobbyCommand::Join(room_name, passwd, mods) => {
                format!("<JOIN>{}<PSWD>{}<MODS>{}", room_name, passwd, mods)
            }
            LobbyCommand::Leave(room_name) => format!("<LEAVE>{}", room_name),
            LobbyCommand::Kick(username) => format!("<KICK>{}", username),
            LobbyCommand::Ready(room_name) => format!("<READY>{}", room_name),
            LobbyCommand::ForceStart(room_name) => format!("<FORCESTART>{}", room_name),
            LobbyCommand::Here => "<HERE>".to_string(),
            LobbyCommand::Alive => "<ALIVE>".to_string(),
            LobbyCommand::HostMode(host_mode) => format!("<HOSTMODE>{}", host_mode),
            LobbyCommand::Connect(address) => unreachable!(),
        }
    }
}

fn request_new_session(session: String, total_players: i32, password: String, mods: String) {}
