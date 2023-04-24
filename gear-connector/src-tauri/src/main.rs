// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

pub mod gear_client;
pub mod ipfs_client;
pub mod logic;
pub mod program_io;
pub mod utils;
pub mod vcmi_server;

use std::net::SocketAddr;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use crate::vcmi_server::VcmiServer;
use crossbeam_channel::{bounded, Sender};

use gear_client::GearClient;
use gear_client::GearCommand;

use gear_client::GearReply;
use gear_connector_api::VcmiCommand;
use gear_connector_api::VcmiReply;
use gstd::FromStr;
use ipfs_client::IpfsClient;
use ipfs_client::IpfsCommand;
use ipfs_client::IpfsReply;
use logic::Logic;
use tauri::Manager;
use tracing::info;
use tracing_core::LevelFilter;
use tracing_subscriber::{prelude::*, Registry};
use utils::MainWindowSubscriber;

/// Запускаем vcmiclient совместно с gear-connector
/// Когда пользователь выбирает многопользовательская игра, показываем Диалоговое окно с предложением подключиться к GEAR
/// Если пользователь соглашается - подключаемся, уменьшаем размер окна, показываем статус подключения
/// Если отказывается - закрываем диалоговое окно
// gui  <-> connector
// vcmi <-> connector -> gear

#[derive(Debug)]
pub enum GuiCommand {
    ConnectToNode {
        address: String,
        program_id: String,
        account_id: String,
        password: String,
    },
    ExpandLog,
    Cancel,
}

fn main() {
    let (vcmi_command_sender, vcmi_command_receiver) = bounded::<VcmiCommand>(1);
    let (vcmi_reply_sender, vcmi_reply_receiver) = bounded::<VcmiReply>(1);

    let (gui_sender, gui_command_receiver) = bounded::<GuiCommand>(1);

    let (gear_command_sender, gear_command_receiver) = bounded::<GearCommand>(1);
    let (gear_reply_sender, gear_reply_receiver) = bounded::<GearReply>(1);

    let (ipfs_command_sender, ipfs_command_receiver) = bounded::<IpfsCommand>(1);
    let (ipfs_reply_sender, ipfs_reply_receiver) = bounded::<IpfsReply>(1);

    let need_stop = Arc::new(AtomicBool::new(false));
    let need_stop_clone = need_stop.clone();

    tauri::Builder::default()
        .manage(gui_sender)
        .plugin(tauri_plugin_positioner::init())
        .invoke_handler(tauri::generate_handler![connect, skip, expand_log])
        .setup(|app| {
            let app_handle = app.handle();
            let main_window = app_handle.get_window("main").unwrap();
            let log_window = app_handle.get_window("log").unwrap();
            // let load_game_window = app_handle.get_window("load_game").unwrap();

            let filter = LevelFilter::INFO;
            let stdout_log = tracing_subscriber::fmt::layer().with_filter(filter);
            let my_subscriber = MainWindowSubscriber {
                window: log_window.clone(),
            };
            let subscriber = Registry::default().with(stdout_log).with(my_subscriber);
            tracing::subscriber::set_global_default(subscriber).unwrap();

            main_window.hide().unwrap();
            log_window.hide().unwrap();

            main_window
                .set_size(tauri::Size::Physical(tauri::PhysicalSize {
                    width: 780,
                    height: 585,
                }))
                .unwrap();
            main_window.center().unwrap();
            let address = SocketAddr::from_str("127.0.0.1:6666").unwrap();
            tauri::async_runtime::spawn(async move {
                VcmiServer::new(
                    need_stop.clone(),
                    address,
                    vcmi_command_sender,
                    vcmi_reply_receiver,
                )
                .await
                .run()
                .await
                .expect("Server error")
            });

            let need_stop = need_stop_clone.clone();
            std::thread::spawn(move || {
                IpfsClient::new(need_stop.clone(), ipfs_reply_sender, ipfs_command_receiver)
                    .run()
                    .expect("IpfsClient error");
            });

            let need_stop = need_stop_clone.clone();
            let logic = Logic::new(
                need_stop.clone(),
                gear_command_sender,
                gear_reply_receiver,
                vcmi_command_receiver,
                vcmi_reply_sender,
                ipfs_reply_receiver,
                ipfs_command_sender,
                gui_command_receiver,
                main_window,
                log_window,
            );

            tauri::async_runtime::spawn(async move {
                logic.run().await;
            });

            std::thread::spawn(move || {
                let gear_client =
                    GearClient::new(need_stop_clone, gear_command_receiver, gear_reply_sender);
                gear_client.run()
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("Cant create app");
}

#[tauri::command]
async fn connect(
    address: String,
    account_id: String,
    program_id: String,
    password: String,
    gui_sender: tauri::State<'_, Sender<GuiCommand>>,
) -> Result<(), String> {
    info!(
        "Received Connect from js: Address: {address}, ProgramID: {program_id}, AccountID: {account_id}");
    let cmd = GuiCommand::ConnectToNode {
        address,
        account_id,
        program_id,
        password,
    };
    gui_sender.send(cmd).unwrap();

    Ok(())
}

#[tauri::command]
async fn skip(gui_sender: tauri::State<'_, Sender<GuiCommand>>) -> Result<(), String> {
    info!("Skip");
    let cmd = GuiCommand::Cancel;
    gui_sender.send(cmd).unwrap();

    Ok(())
}

#[tauri::command]
async fn expand_log(gui_sender: tauri::State<'_, Sender<GuiCommand>>) -> Result<(), String> {
    let cmd = GuiCommand::ExpandLog;
    gui_sender.send(cmd).unwrap();

    Ok(())
}
