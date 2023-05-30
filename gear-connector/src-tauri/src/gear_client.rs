use std::{
    sync::{
        atomic::{AtomicBool, Ordering::Relaxed},
        Arc, RwLock,
    },
    time::Duration,
};

use crate::program_io::{Action, Event, GameState};
use crossbeam_channel::{Receiver, RecvTimeoutError, Sender};
use gclient::{EventListener, GearApi, WSAddress};
use gmeta::Encode;
use gstd::ActorId;

pub const RECV_TIMEOUT: Duration = std::time::Duration::from_millis(1);

#[derive(Debug)]
pub enum GearCommand {
    ConnectToNode {
        address: WSAddress,
        program_id: String,
        account_id: String,
        password: String,
    },
    GetFreeBalance,
    SendAction(Action),
    GetSavedGames,
}

#[derive(Debug)]
pub enum GearReply {
    Connected,
    NotConnected(String),
    ProgramNotFound { program_id: String },
    Event(Event),
    FreeBalance(u128),
    SavedGames(Vec<(ActorId, GameState)>),
}

pub struct GearConnection {
    client: GearApi,
    listener: EventListener,
    program_id: [u8; 32],
}

pub struct GearClient {
    need_stop: Arc<AtomicBool>,
    gear_reply_sender: Sender<GearReply>,
    gear_command_receiver: Receiver<GearCommand>,
    gear_connection: Arc<RwLock<Option<GearConnection>>>,
}

unsafe impl Send for GearConnection {}
unsafe impl Send for GearClient {}

impl GearClient {
    pub fn new(
        need_stop: Arc<AtomicBool>,
        gear_command_receiver: Receiver<GearCommand>,
        gear_reply_sender: Sender<GearReply>,
    ) -> Self {
        Self {
            need_stop,
            gear_reply_sender,
            gear_command_receiver,
            gear_connection: Arc::new(RwLock::new(None)),
        }
    }

    pub fn run(&self) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {
            while !self.need_stop.load(Relaxed) {
                let command = self.gear_command_receiver.recv_timeout(RECV_TIMEOUT);

                match command {
                    Ok(command) => self.process_command(command).await,
                    Err(error) if error == RecvTimeoutError::Timeout => {}
                    Err(error) => {
                        tracing::error!("Error in another thread: {}", error);
                        self.need_stop.store(true, Relaxed);
                    }
                }

                if let Some(connection) = self.gear_connection.write().unwrap().as_mut() {
                    if let Err(e) = connection.listener.blocks_running().await {
                        self.gear_reply_sender
                            .send(GearReply::NotConnected(format!("{e}")))
                            .expect("Cant' send");
                    }
                }

                std::thread::sleep(std::time::Duration::from_millis(1));
            }
        });
    }

    async fn get_saved_games(&self) {
        let guard = self
            .gear_connection
            .read()
            .expect("Error in another thread");
        if let Some(GearConnection {
            client,
            program_id,
            listener: _,
        }) = guard.as_ref()
        {
            let program_id = (*program_id).into();
            let saved_games: Vec<(ActorId, GameState)> = client
                .read_state(program_id)
                .await
                .expect("Can't read state");
            self.gear_reply_sender
                .send(GearReply::SavedGames(saved_games))
                .expect("Panic in another thread");
        } else {
            unreachable!("Not connected to blockchain");
        }
    }

    async fn get_free_balance(&self) {
        let guard = self
            .gear_connection
            .read()
            .expect("Error in another thread");
        if let Some(GearConnection {
            client,
            program_id: _,
            listener: _,
        }) = guard.as_ref()
        {
            let free_balance = client.free_balance(client.account_id()).await.unwrap();
            self.gear_reply_sender
                .send(GearReply::FreeBalance(free_balance))
                .expect("Panic in another thread");
        } else {
            unreachable!("Not connected to blockchain");
        }
    }

    async fn process_action(&self, action: Action) {
        let mut guard = self
            .gear_connection
            .write()
            .expect("Error in another thread");
        tracing::debug!("Process action {:?}", action);
        if let Some(GearConnection {
            client,
            program_id,
            ref mut listener,
        }) = guard.as_mut()
        {
            let pid = *program_id;
            let program_id = pid.into();

            let gas_limit = client
                .calculate_handle_gas(None, program_id, action.encode(), 0, true)
                .await
                .expect("Can't calculate gas for Action::Save")
                .min_limit;
            tracing::info!("Gas limit {} for Action {:?}", gas_limit, action);
            let (message_id, _) = client
                .send_message(program_id, &action, gas_limit, 0)
                .await
                .expect("Error at sending Action::Save");
            tracing::info!("Send Action to Gear: {:?}", action);

            // !TODO. Code that works with EventListener doesn't work:

            // tracing::info!("Action Succeed");
            // let mut listener = client.subscribe().await.unwrap();
            // assert!(listener
            //     .message_processed(message_id)
            //     .await
            //     .expect("Check processed error")
            //     .succeed());
            // let (_m, raw_reply, _) = listener
            //     .reply_bytes_on(message_id)
            //     .await
            //     .expect("Reply bytes error");
            // let raw_reply = raw_reply.unwrap();
            // let decoded_event: Event =
            //     Decode::decode(&mut raw_reply.as_slice()).expect("Can't decode reply");
            // tracing::info!("Received reply from Gear: {:?}", decoded_event);

            // let reply = GearReply::Event(decoded_event);

            self.gear_reply_sender
                .send(GearReply::Event(Event::Saved))
                .unwrap();
        } else {
            tracing::warn!("Can't connect to Gear Blockchain Node")
        }
    }

    async fn process_command(&self, command: GearCommand) {
        match command {
            GearCommand::ConnectToNode {
                address,
                program_id,
                account_id,
                password,
            } => {
                tracing::info!(
                    "Process GUI command ConnectToNode address: {:?}, Program ID: {}",
                    address,
                    program_id
                );
                let mut guard = self
                    .gear_connection
                    .write()
                    .expect("Error in another thread");
                if guard.is_none() {
                    let suri = format!("{account_id}:{password}");
                    let client = GearApi::init_with(address, suri).await;
                    // let client = GearApi::dev().await;
                    // let client = GearApi::init(address).await;
                    match client {
                        Ok(client) => {
                            let pid =
                                hex::decode(&program_id[2..]).expect("Can't decode Program ID");
                            let mut program_id = [0u8; 32];
                            program_id.copy_from_slice(&pid);

                            match client.read_metahash(program_id.into()).await {
                                Ok(hash) => {
                                    tracing::info!("Program hash: {:?}", hash);
                                    let gear_connection = GearConnection {
                                        client: client.clone(),
                                        program_id,
                                        listener: client.subscribe().await.unwrap(),
                                    };
                                    guard.replace(gear_connection);
                                    self.gear_reply_sender
                                        .send(GearReply::Connected)
                                        .expect("Panic in another thread");
                                }
                                Err(err) => {
                                    tracing::error!("Read State Error: {}", err);
                                    self.gear_reply_sender
                                        .send(GearReply::ProgramNotFound {
                                            program_id: hex::encode(&program_id),
                                        })
                                        .expect("Error in another thread");
                                }
                            }
                        }
                        Err(e) => {
                            tracing::error!("Gear connect Error: {}", e);
                            self.gear_reply_sender
                                .send(GearReply::NotConnected(format!("{e}")))
                                .expect("Panic in another thread");
                        }
                    }
                }
            }
            GearCommand::SendAction(action) => self.process_action(action).await,
            GearCommand::GetFreeBalance => self.get_free_balance().await,
            GearCommand::GetSavedGames => self.get_saved_games().await,
        }
    }
}
