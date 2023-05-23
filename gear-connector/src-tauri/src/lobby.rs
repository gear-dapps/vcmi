use crate::gear_client::RECV_TIMEOUT;
use crossbeam_channel::{Receiver, RecvTimeoutError, Sender};
use serde::Serialize;
use std::net::TcpStream;
use std::{
    io::{Read, Write},
    sync::{
        atomic::{AtomicBool, Ordering::Relaxed},
        Arc,
    },
};

#[derive(Debug)]
pub enum LobbyCommand {
    Connect(String),
    Greeting(String, String),
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

#[derive(Debug)]
pub enum LobbyReply {
    Connected,
    Created(String),
    Sessions(Vec<Room>),
    Joined(String, String),
    Kicked,
    Start,
    Host,
    Status(String, String, String),
    ServerError(String),
    Mods,
    ClientMods,
    Chat(String, String),
    Users(Vec<String>),
    Health,
    GameMode,
}
#[derive(Debug, Serialize)]
pub struct Room {
    pub joined: u32,
    pub total: u32,
    pub protected: bool,
    pub name: String,
}

const PROTOCOL_VERSION: u8 = 4;
const PROTOCOL_ENCODING: &str = "utf8";
pub const VCMI_VERSION: &str = "VCMI 1.2.1.6f9e76ad3ee0ec77ba9b52c857b8d50e631d1ef6";

const SESSIONS: &str = ":>>SESSIONS:";
const USERS: &str = ":>>USERS:";
const MSG: &str = ":>>MSG:";
const ERROR: &str = ":>>ERROR:";
const CREATED: &str = ":>>CREATED:";
const JOIN: &str = ":>>JOIN:";
const GAMEMODE: &str = ":>>GAMEMODE:";
const STATUS: &str = ":>>STATUS:";
const KICK: &str = ":>>KICK:";
const MODS: &str = ":>>MODS:";
const MODSOTHER: &str = ":>>MODSOTHER:";

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

    pub fn run(&mut self) -> std::io::Result<()> {
        let need_stop = self.need_stop.clone();
        let need_stop_clone = self.need_stop.clone();
        let lobby_reply_sender2 = self.lobby_reply_sender.clone();

        let mut raw_reply = [0; 4096];

        while !need_stop.load(Relaxed) {
            let command: Result<LobbyCommand, RecvTimeoutError> =
                self.lobby_command_receiver.recv_timeout(RECV_TIMEOUT);
            // tracing::info!("send thread");
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

            if let Some(mut stream) = self.connection.as_ref() {
                // tracing::info!("read");
                match stream.read(&mut raw_reply) {
                    Ok(n) => {
                        let mut raw_reply = raw_reply.to_vec();
                        raw_reply.truncate(n);
                        let raw = String::from_utf8(raw_reply).expect("Can't convert reply to ut8");
                        tracing::info!("Received from lobby: {}", raw);

                        let commands = split_commands(&raw);
                        for command in commands {
                            let reply = parse_raw_reply(command);

                            lobby_reply_sender2.send(reply).expect("Send error");
                        }
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        // tracing::debug!("would block");
                    }
                    Err(e) => {
                        need_stop_clone.store(true, Relaxed);
                        tracing::error!("Can't read lobby socket: {}", e);
                    }
                }
            }

            std::thread::sleep(std::time::Duration::from_millis(5));
        }

        Ok(())
    }

    pub fn process_command(&mut self, command: LobbyCommand) {
        tracing::info!("process lobby command(): {:?}", command);
        match command {
            LobbyCommand::Connect(address) => {
                self.connect(address).expect("Can't connect to lobby");
                self.lobby_reply_sender
                    .send(LobbyReply::Connected)
                    .expect("Can't send");
            }
            command => {
                if let Some(mut connection) = self.connection.as_mut() {
                    Self::send(&mut connection, &command);
                }
            }
        }
    }

    pub fn connect(&mut self, address: String) -> std::io::Result<()> {
        let stream = TcpStream::connect(&address)?;
        stream.set_read_timeout(Some(RECV_TIMEOUT)).unwrap();
        self.connection = Some(stream);

        Ok(())
    }

    pub fn send(connection: &mut TcpStream, command: &LobbyCommand) {
        // let command = LobbyCommand::Message("TEST TEST".to_string());
        tracing::debug!("Send command {:?} to lobby", command);
        let command = command.to_bytes();

        let command_len = command.len() as u32;
        let command_len_bytes = command_len.to_le_bytes();

        let mut bytes: Vec<u8> = vec![];

        bytes.extend(command_len_bytes);
        bytes.extend(&command);

        connection.write_all(&bytes).expect("Can't send");
    }
}

impl LobbyCommand {
    fn to_bytes(&self) -> Vec<u8> {
        match self {
            LobbyCommand::Greeting(name, vcmi_version) => {
                let mut bytes = vec![];
                bytes.push(PROTOCOL_VERSION);

                let encoding = PROTOCOL_ENCODING.as_bytes();

                let encoding_len = encoding.len() as u8;
                bytes.push(encoding_len);

                bytes.extend(encoding);
                let greetings = format!(
                    "{}<GREETINGS>{}<VER>{}",
                    PROTOCOL_ENCODING, name, vcmi_version
                );
                bytes.extend(greetings.as_bytes());
                bytes
            }
            LobbyCommand::Username(username) => format!("<USER>{}", username).as_bytes().to_vec(),
            LobbyCommand::Message(message) => format!("<MSG>{}", message).as_bytes().to_vec(),
            LobbyCommand::Create(room_name, passwd, max_players, mods) => format!(
                "<NEW>{}<PSWD>{}<COUNT>{}<MODS>{}",
                room_name, passwd, max_players, mods
            )
            .as_bytes()
            .to_vec(),
            LobbyCommand::Join(room_name, passwd, mods) => {
                format!("<JOIN>{}<PSWD>{}<MODS>{}", room_name, passwd, mods)
                    .as_bytes()
                    .to_vec()
            }
            LobbyCommand::Leave(room_name) => format!("<LEAVE>{}", room_name).as_bytes().to_vec(),
            LobbyCommand::Kick(username) => format!("<KICK>{}", username).as_bytes().to_vec(),
            LobbyCommand::Ready(room_name) => format!("<READY>{}", room_name).as_bytes().to_vec(),
            LobbyCommand::ForceStart(room_name) => {
                format!("<FORCESTART>{}", room_name).as_bytes().to_vec()
            }
            LobbyCommand::Here => "<HERE>".to_string().as_bytes().to_vec(),
            LobbyCommand::Alive => "<ALIVE>".to_string().as_bytes().to_vec(),
            LobbyCommand::HostMode(host_mode) => {
                format!("<HOSTMODE>{}", host_mode).as_bytes().to_vec()
            }
            LobbyCommand::Connect(_address) => unreachable!(),
        }
    }
}

fn parse_raw_reply(raw: String) -> LobbyReply {
    match raw {
        raw if raw.starts_with(CREATED) => parse_created(raw),
        raw if raw.starts_with(SESSIONS) => parse_sessions(raw),
        raw if raw.starts_with(USERS) => parse_users(raw),
        raw if raw.starts_with(MSG) => parse_message(raw),
        raw if raw.starts_with(ERROR) => parse_error(raw),
        raw if raw.starts_with(JOIN) => parse_join(raw),
        raw if raw.starts_with(STATUS) => parse_status(raw),
        raw if raw.starts_with(MODS) => parse_mods(raw),
        raw if raw.starts_with(MODSOTHER) => parse_modsother(raw),
        _ => unreachable!(),
    }
}

fn parse_sessions(sessions: String) -> LobbyReply {
    let mut splitted = sessions.split(":");
    let len_str = splitted.nth(2).unwrap(); // rooms count

    let len = len_str.parse::<usize>().unwrap();
    let mut rooms = vec![];
    for _ in 0..len {
        let name = splitted.next().unwrap().to_string();
        let joined = splitted.next().unwrap().to_string().parse().unwrap();
        let total = splitted.next().unwrap().to_string().parse().unwrap();
        let protected = splitted.next().unwrap().to_string().eq("True");
        let room = Room {
            joined,
            total,
            protected,
            name,
        };
        rooms.push(room);
    }
    LobbyReply::Sessions(rooms)
}

fn parse_users(users: String) -> LobbyReply {
    let mut splitted = users.split(":");
    let len_str = splitted.nth(2).unwrap(); // users count

    let len = len_str.parse::<usize>().unwrap();
    let mut users = vec![];
    for i in 0..len {
        let name = splitted.next().unwrap().to_string();

        users.push(name);
    }
    LobbyReply::Users(users)
}

fn parse_created(created: String) -> LobbyReply {
    let mut splitted = created.split(":");
    let room_name = splitted.nth(2).unwrap().to_string();

    LobbyReply::Created(room_name)
}

fn parse_message(message: String) -> LobbyReply {
    let mut splitted = message.split(":");
    let username = splitted.nth(2).unwrap().to_string();
    let message = splitted.next().unwrap().to_string();

    LobbyReply::Chat(username, message)
}

fn parse_error(message: String) -> LobbyReply {
    let mut splitted = message.split(":");
    let error = splitted.nth(2).unwrap().to_string();

    LobbyReply::ServerError(error)
}

fn parse_join(message: String) -> LobbyReply {
    let mut splitted = message.split(":");
    let room_name = splitted.nth(2).unwrap().to_string();
    let username = splitted.next().unwrap().to_string();

    LobbyReply::Joined(room_name, username)
}

fn parse_status(message: String) -> LobbyReply {
    let mut splitted = message.split(":");
    let users_count = splitted.nth(2).unwrap().to_string();
    let room_name = splitted.next().unwrap().to_string();
    let username = splitted.next().unwrap().to_string();

    LobbyReply::Status(users_count, room_name, username)
}

fn parse_mods(_message: String) -> LobbyReply {
    LobbyReply::Mods
}

fn parse_modsother(_message: String) -> LobbyReply {
    LobbyReply::ClientMods
}

fn split_commands(input: &str) -> Vec<String> {
    let delimiter = ":>>";

    let mut result: Vec<String> = Vec::new();

    let splitted = input.split(delimiter);

    for s in splitted {
        if !s.is_empty() {
            result.push(format!("{delimiter}{s}"));
        }
    }

    result
}
