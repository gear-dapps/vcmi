use crate::gear_client::RECV_TIMEOUT;
use crossbeam_channel::{Receiver, RecvTimeoutError, Sender};
use serde::Serialize;
use std::net::TcpStream;
use std::{
    io::{Read, Write},
    sync::{
        atomic::{AtomicBool, Ordering::Relaxed},
        Arc, RwLock,
    },
};

#[derive(Debug)]
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

#[derive(Debug)]
pub enum LobbyReply {
    Connected,
    Created(String),
    Sessions(Vec<Room>),
    Joined,
    Kicked,
    Start,
    Host,
    Status,
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

const SESSIONS: &str = ":>>SESSIONS:";
const USERS: &str = ":>>USERS:";
const MSG: &str = ":>>MSG:";
const ERROR: &str = ":>>ERROR:";
const CREATED: &str = ":>>CREATED:";
const JOIN: &str = ":>>JOIN:";
const GAMEMODE: &str = ":>>GAMEMODE:";
const STATUS: &str = ":>>STATUS:";
const KICK: &str = ":>>KICK:";

pub struct LobbyClient {
    need_stop: Arc<AtomicBool>,
    connection: Arc<RwLock<Option<TcpStream>>>,
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
            connection: Arc::new(RwLock::new(None)),
            lobby_command_receiver,
            lobby_reply_sender,
            username: String::new(),
        }
    }

    pub fn run(&self) -> std::io::Result<()> {
        let lobby_command_receiver = self.lobby_command_receiver.clone();
        let lobby_reply_sender = self.lobby_reply_sender.clone();
        let need_stop = self.need_stop.clone();
        let need_stop_clone = self.need_stop.clone();
        let connection = self.connection.clone();
        let stream = connection.clone();
        let lobby_reply_sender2 = self.lobby_reply_sender.clone();

        let mut raw_reply = [0; 4096];

        'outer: while !need_stop.load(Relaxed) {
            let command: Result<LobbyCommand, RecvTimeoutError> =
                lobby_command_receiver.recv_timeout(RECV_TIMEOUT);
            // tracing::info!("send thread");
            match command {
                Ok(command) => {
                    Self::process_command(lobby_reply_sender.clone(), connection.clone(), command);
                }
                Err(error) if error == RecvTimeoutError::Timeout => {}
                Err(error) => {
                    tracing::error!("Error in another thread: {}", error);
                    need_stop.store(true, Relaxed);
                }
            }

            if let Some(mut stream) = stream.read().unwrap().as_ref() {
                while !need_stop.load(Relaxed) {
                    // tracing::info!("read");
                    match stream.read(&mut raw_reply) {
                        Ok(n) => {
                            let mut raw_reply = raw_reply.to_vec();
                            raw_reply.truncate(n);
                            let raw =
                                String::from_utf8(raw_reply).expect("Can't convert reply to ut8");
                            tracing::info!("Received from lobby: {}", raw);

                            let commands = split_commands(&raw);
                            for command in commands {
                                let reply = parse_raw_reply(command);

                                lobby_reply_sender2.send(reply).expect("Send error");
                            }
                            break;
                        }
                        Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                            continue 'outer;
                        }
                        Err(e) => {
                            need_stop_clone.store(true, Relaxed);
                            tracing::error!("Can't read lobby socket: {}", e);
                        }
                    }
                }
            }

            std::thread::sleep(std::time::Duration::from_millis(100));
        }

        Ok(())
    }

    pub fn process_command(
        lobby_reply_sender: Sender<LobbyReply>,
        connection: Arc<RwLock<Option<TcpStream>>>,
        command: LobbyCommand,
    ) {
        tracing::info!("process lobby command(): {:?}", command);
        match command {
            LobbyCommand::Connect(address) => {
                Self::connect(connection, address).expect("Can't connect to lobby");
                lobby_reply_sender
                    .send(LobbyReply::Connected)
                    .expect("Can't send");
            }
            LobbyCommand::Greeting(protocol_version, username, vcmi_version) => {
                Self::greetings(connection, username, vcmi_version);
            }
            command => {
                if let Some(connection) = connection.write().expect("Poison Error").as_mut() {
                    Self::send(connection, &command);
                }
            }
        }
    }

    pub fn connect(
        connection: Arc<RwLock<Option<TcpStream>>>,
        address: String,
    ) -> std::io::Result<()> {
        let stream = TcpStream::connect(&address)?;
        stream.set_read_timeout(Some(RECV_TIMEOUT)).unwrap();
        *connection.write().unwrap() = Some(stream);

        Ok(())
    }

    pub fn greetings(
        connection: Arc<RwLock<Option<TcpStream>>>,
        username: String,
        vcmi_version: String,
    ) {
        let mut bytes: Vec<u8> = vec![];
        let command = LobbyCommand::Greeting(PROTOCOL_VERSION, username, vcmi_version).to_string();

        let command_len = command.len() as u32;
        let len_bytes = command_len.to_le_bytes();
        bytes.extend(&len_bytes);

        let encoding = PROTOCOL_ENCODING.as_bytes();

        let encoding_len = encoding.len() as u8;
        let encoding_len_bytes = encoding_len.to_le_bytes();
        bytes.extend(encoding_len_bytes);

        bytes.extend(encoding);
        bytes.extend(command.as_bytes());

        let stream = connection.write();
        let mut s = stream.unwrap();
        if let Some(stream) = s.as_mut() {
            stream.write_all(&bytes).expect("Can't send greetings");
        }
    }

    pub fn send(connection: &mut TcpStream, command: &LobbyCommand) {
        let command = command.to_string();
        let mut bytes: Vec<u8> = vec![];
        tracing::debug!("Send command {:?} to lobby", command);

        bytes.extend(command.as_bytes());

        connection.write_all(&bytes).expect("Can't send");
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
        let protected = splitted.next().unwrap().to_string().parse().unwrap();
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
