use std::{
    sync::{
        atomic::{AtomicBool, Ordering::Relaxed},
        Arc,
    },
};

use crossbeam_channel::{Receiver, RecvTimeoutError, Sender};

use gclient::WSAddress;
use gear_connector_api::{VcmiCommand, VcmiReply, VcmiSavedGame};
use gstd::ActorId;
use tauri::{PhysicalSize, Size, Window};
use tauri_plugin_positioner::{Position, WindowExt};

use crate::{
    gear_client::{GearCommand, GearReply, RECV_TIMEOUT},
    ipfs_client::{IpfsCommand, IpfsReply},
    program_io::{Action, ArchiveDescription, Event, GameState},
    GuiCommand,
};

pub enum Recipient {
    GearClient,
    Vcmi,
    Gui,
}

pub enum Message {
    VcmiCommand,
    VcmiReply,
    Action,
    Event,
}

pub struct Logic {
    need_stop: Arc<AtomicBool>,
    gear_command_sender: Sender<GearCommand>,
    gear_reply_receiver: Receiver<GearReply>,
    vcmi_command_receiver: Receiver<VcmiCommand>,
    vcmi_reply_sender: Sender<VcmiReply>,
    ipfs_reply_receiver: Receiver<IpfsReply>,
    ipfs_command_sender: Sender<IpfsCommand>,
    gui_command_receiver: Receiver<GuiCommand>,
    main_window: Window,
    log_window: Window,
}

impl Logic {
    pub fn new(
        need_stop: Arc<AtomicBool>,
        gear_command_sender: Sender<GearCommand>,
        gear_reply_receiver: Receiver<GearReply>,
        vcmi_command_receiver: Receiver<VcmiCommand>,
        vcmi_reply_sender: Sender<VcmiReply>,
        ipfs_reply_receiver: Receiver<IpfsReply>,
        ipfs_command_sender: Sender<IpfsCommand>,
        gui_command_receiver: Receiver<GuiCommand>,
        main_window: Window,
        log_window: Window,
    ) -> Self {
        Self {
            need_stop,
            gear_command_sender,
            gear_reply_receiver,
            vcmi_command_receiver,
            vcmi_reply_sender,
            ipfs_reply_receiver,
            ipfs_command_sender,
            gui_command_receiver,
            main_window,
            log_window,
        }
    }

    pub async fn run(&self) {
        while !self.need_stop.load(Relaxed) {
            self.process_gui_command();
            self.process_vcmi_command().await;
            // futures::future::join(self.process_ipfs_reply(), self.process_gear_reply()).await;
        }
    }

    fn show_connection_dialog(&self) {
        self.main_window.center().unwrap();
        self.main_window.show().unwrap();
        self.main_window.set_focus().unwrap();
        self.vcmi_reply_sender
            .send(VcmiReply::ConnectDialogShowed)
            .expect("Error in another thread");
    }

    fn show_load_game_dialog(&self) {
        let command = GearCommand::GetSavedGames;
        self.gear_command_sender.send(command).expect("Can't send");
        let reply = self.gear_reply_receiver.recv().expect("Can't recv");

        match reply {
            GearReply::SavedGames(games) => {}
            _ => unreachable!("Unexpected reply to GetSavedGames command"),
        }
        self.vcmi_reply_sender
            .send(VcmiReply::LoadGameDialogShowed)
            .expect("Error in another thread");
    }

    fn save(&self, filename: String, compressed_archive: Vec<u8>) {
        let archive_name = format!("{filename}");

        tracing::info!("Archive len: {}", compressed_archive.len());
        // let file = OpenOptions::new()
        //     .write(true)
        //     .create(true)
        //     .read(true)
        //     .truncate(true)
        //     .open(&archive_name)
        //     .unwrap();
        // let mut tar = Builder::new(BufWriter::new(file));
        // let mut header = Header::new_gnu();

        // tar.append_data(&mut header, &archive_name, compressed_archive.as_slice())
        //     .unwrap();

        // let mut buffer = tar.into_inner().unwrap();
        // buffer.flush().unwrap(); //.into_inner().unwrap();

        // let mut tar_data = Vec::new();
        // let mut tar_file = File::open(&archive_name).unwrap();
        // tar_file.read_to_end(&mut tar_data).unwrap();

        let command = IpfsCommand::UploadData {
            filename,
            data: compressed_archive,
        };
        self.ipfs_command_sender.send(command).expect("Send error");

        let reply = self.ipfs_reply_receiver.recv().expect("Recv error");

        if let IpfsReply::Uploaded { name, hash } = reply {
            let saver_id = ActorId::default();
            let tar = ArchiveDescription {
                filename: archive_name,
                name,
                hash,
            };

            let gear_command = GearCommand::SendAction(Action::Save(GameState {
                saver_id,
                archive: tar,
            }));
            self.gear_command_sender
                .send(gear_command)
                .expect("Send error");
            let gear_reply = self.gear_reply_receiver.recv().expect("Recv error");

            if let GearReply::Event(e) = gear_reply {
                if matches!(e, Event::Saved) {
                    self.vcmi_reply_sender
                        .send(VcmiReply::Saved)
                        .expect("Send error");
                    return;
                }
            }
        }

        unreachable!();
    }

    fn load_all(&self) {
        self.gear_command_sender
            .send(GearCommand::GetSavedGames)
            .expect("Send error");

        let gear_reply = self.gear_reply_receiver.recv().expect("Recv Error");
        match gear_reply {
            GearReply::SavedGames(games) => {
                let mut archives = Vec::with_capacity(games.len());
                for (_actor_id, state) in games.into_iter() {
                    let hash = state.archive.hash;
                    let ipfs_command = IpfsCommand::DownloadData { hash };
                    self.ipfs_command_sender
                        .send(ipfs_command)
                        .expect("Send err");
                    let ipfs_reply = self.ipfs_reply_receiver.recv().expect("Recv err");
                    match ipfs_reply {
                        IpfsReply::Downloaded { data } => {
                            archives.push(VcmiSavedGame {
                                filename: state.archive.filename,
                                data,
                            });
                        }
                        _ => unreachable!("Wrong reply to Ipfs Download command"),
                    }
                }
                let vcmi_reply = VcmiReply::AllLoaded { archives };
                self.vcmi_reply_sender.send(vcmi_reply).expect("Send err");
            }
            _ => unreachable!("Wrong reply to GetSavedGames"),
        }
    }

    async fn update_balance(&self) {
        self.gear_command_sender
            .send(GearCommand::GetFreeBalance)
            .expect("Send Error");

        let reply = self.gear_reply_receiver.recv().expect("Recv error");
        match reply {
            GearReply::FreeBalance(balance) => {
                self.log_window.emit("update_balance", balance).unwrap();
                tracing::info!("Free balance: {}", balance);
            }
            _ => unreachable!("Reply {reply:?} is wrong to command FreeBalance"),
        }
    }

    async fn process_vcmi_command(&self) {
        match self.vcmi_command_receiver.recv_timeout(RECV_TIMEOUT) {
            Ok(vcmi_command) => match vcmi_command {
                VcmiCommand::ShowConnectDialog => self.show_connection_dialog(),
                VcmiCommand::Save {
                    filename,
                    compressed_archive,
                } => {
                    self.save(filename, compressed_archive);
                    self.update_balance().await;
                }
                VcmiCommand::Load(name) => self
                    .gear_command_sender
                    .send(GearCommand::SendAction(Action::Load { hash: name }))
                    .expect("Error in another thread"),
                VcmiCommand::ShowLoadGameDialog => self.show_load_game_dialog(),
                VcmiCommand::LoadAll => self.load_all(),
            },
            Err(e) if e == RecvTimeoutError::Timeout => {}
            Err(e) => {
                tracing::error!("Error in another thread: {}", e);
                self.need_stop.store(true, Relaxed);
            }
        }
    }

    // async fn process_ipfs_reply(&self) {
    //     match self.ipfs_reply_receiver.recv_timeout(RECV_TIMEOUT) {
    //         Ok(reply) => match reply {
    //             IpfsReply::Uploaded { name, hash, size } => {
    //                 let action = Action::Save(GameState { name, hash, size });
    //                 self.gear_command_sender
    //                     .send(Command::SendAction(action))
    //                     .expect("Error in another thread");
    //             }
    //             IpfsReply::Downloaded { data } => {
    //                 let reply = VcmiReply::Loaded(VcmiState {
    //                     name: "her".to_string(),
    //                     data,
    //                 });
    //                 self.vcmi_reply_sender
    //                     .send(reply)
    //                     .expect("Error in another thread")
    //             }
    //         },
    //         Err(e) if e == RecvTimeoutError::Timeout => {}
    //         Err(e) => {
    //             tracing::error!("Error in another thread: {}", e);
    //             self.need_stop.store(true, Relaxed);
    //         }
    //     }
    // }

    // fn process_event(&self, event: Event) {
    //     match event {
    //         Event::Loaded(game_state) => {
    //             tracing::info!("State Successfully loaded");
    //             match game_state {
    //                 Some(game_state) => self
    //                     .ipfs_command_sender
    //                     .send(IpfsCommand::Download {
    //                         hash: game_state,
    //                     })
    //                     .expect("Error in another thread"),
    //                 None => {}
    //             };
    //         }
    //         Event::Saved => tracing::info!("State Successfully saved"),
    //     }
    // }

    // async fn process_gear_reply(&self) {
    //     match self.gear_reply_receiver.recv_timeout(RECV_TIMEOUT) {
    //         Ok(reply) => {
    //             // let msg = format!("{:?}", reply);
    //             match reply {
    //                 Reply::Connected => {
    //                     // self.main_window.app_handle().emit_all("connection_view", String::new()).unwrap();
    //                     self.main_window
    //                         .emit_all("connection_view", String::new())
    //                         .unwrap();
    //                     // self.main_window
    //                     //     .set_size(Size::Physical(tauri::PhysicalSize {
    //                     //         width: 300,
    //                     //         height: 300,
    //                     //     }))
    //                     //     .unwrap();
    //                     self.main_window.center().unwrap();
    //                     self.main_window.move_window(Position::BottomRight).unwrap();
    //                     self.vcmi_reply_sender
    //                         .send(VcmiReply::ShowedDialog)
    //                         .expect("Error in another thread");

    //                     self.vcmi_reply_sender
    //                         .send(VcmiReply::Connected)
    //                         .expect("Panic in another thread");
    //                 }
    //                 Reply::NotConnected => self
    //                     .vcmi_reply_sender
    //                     .send(VcmiReply::CanceledDialog)
    //                     .expect("Panic in another thread"),
    //                 Reply::ProgramNotFound { program_id } => {
    //                     self.main_window.emit("not_found", program_id).unwrap();
    //                 }
    //                 Reply::Event(event) => self.process_event(event),
    //             }
    //             // self.main_window.emit("log", msg).unwrap();
    //         }
    //         Err(e) if e == RecvTimeoutError::Timeout => {}
    //         Err(e) => {
    //             tracing::error!("Error in another thread: {}", e);
    //             self.need_stop.store(true, Relaxed);
    //         }
    //     }
    // }

    fn connect(&self, _address: String, program_id: String, account_id: String, password: String) {
        let address = WSAddress::new("ws://localhost", 9944);
        // let address = WSAddress::new("wss://rpc-node.gear-tech.io", 9944);
        self.gear_command_sender
            .send(GearCommand::ConnectToNode {
                address,
                program_id,
                password,
                account_id,
            })
            .expect("Error in another thread");

        let reply = self.gear_reply_receiver.recv().expect("Recv error");

        match reply {
            GearReply::Connected => {
                self.main_window.center().unwrap();
                self.main_window.hide().unwrap();
                self.log_window.show().unwrap();

                self.log_window.move_window(Position::BottomRight).unwrap();
                self.vcmi_reply_sender
                    .send(VcmiReply::ConnectDialogShowed)
                    .expect("Error in another thread");
            }
            GearReply::NotConnected(reason) => self.main_window.emit("alert", reason).unwrap(),
            GearReply::ProgramNotFound { program_id } => {
                self.main_window.emit("alert", program_id).unwrap()
            }
            _ => unreachable!("Reply {reply:?} is wrong to command Connect"),
        }
    }

    fn process_gui_command(&self) {
        match self.gui_command_receiver.recv_timeout(RECV_TIMEOUT) {
            Ok(gui_command) => {
                tracing::debug!("Process Gui Command: {:?}", gui_command);
                match gui_command {
                    GuiCommand::ConnectToNode {
                        address,
                        program_id,
                        password,
                        account_id,
                    } => {
                        self.connect(address, program_id, account_id, password);
                    }
                    GuiCommand::Cancel => {
                        // main_window.set_fullscreen(true).unwrap();
                        self.main_window.hide().unwrap();
                        self.vcmi_reply_sender
                            .send(VcmiReply::CanceledDialog)
                            .expect("Panic in another thread");
                    }
                    GuiCommand::ExpandLog => {
                        let size = self.log_window.inner_size().unwrap();
                        const EXPANDED_SIZE: u32 = 600;
                        let height = match size.height == EXPANDED_SIZE {
                            true => 150,
                            false => EXPANDED_SIZE,
                        };
                        let width = size.width;
                        self.log_window
                            .set_size(Size::Physical(PhysicalSize::new(width, height)))
                            .unwrap();
                        std::thread::sleep(std::time::Duration::from_millis(1));
                        self.log_window.move_window(Position::BottomRight).unwrap();
                    }
                }
            }
            Err(e) if e == RecvTimeoutError::Timeout => {}
            Err(e) => {
                tracing::error!("Error in another thread: {}", e);
                self.need_stop.store(true, Relaxed);
            }
        }
    }
}
