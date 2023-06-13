mod io;
mod utils;

use crate::ffi::SelectionScreen;
use crate::utils::get_file_as_byte_vec;
use futures::{SinkExt, StreamExt};
use gear_connector_api::utils::split_to_reply_read_command_write;
use gear_connector_api::*;
use gstd::prelude::*;
use once_cell::sync::OnceCell;
use std::{
    fs::{File, OpenOptions},
    io::Cursor,
    path::Path,
};

use tokio::net::TcpStream as TokioTcpStream;
// use tokio::sync::broadcast::{self, Receiver, Sender};
use crossbeam_channel::{bounded, Receiver, Sender};
use std::io::{Read, Write};
use tokio::task::JoinHandle;

use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::Arc;
use zip::{
    write::{FileOptions, ZipWriter},
    ZipArchive,
};

struct Connection {
    runtime: tokio::runtime::Runtime,
    need_stop: Arc<AtomicBool>,
    command_sender: Sender<VcmiCommand>,
    reply_receiver: Receiver<VcmiReply>,
    read_t: JoinHandle<()>,
    write_t: JoinHandle<()>,
}

static mut CONNECTION: OnceCell<Connection> = OnceCell::new();

pub fn save_state_onchain(vcgm_path: String, vsgm_path: String) -> i32 {
    let connection = try_init_connection!(connection_init);
    println!("Rust save_state_onchain");

    let path = Path::new(&vcgm_path);
    let filename = path.file_stem().unwrap().to_str().unwrap();
    assert_eq!(
        filename,
        Path::new(&vsgm_path).file_stem().unwrap().to_str().unwrap()
    );
    let filename = format!("{filename}");
    println!(
        "Save current state {} {} {} on gear chain",
        filename, vcgm_path, vsgm_path,
    );

    let archive = File::create(&filename).unwrap();
    let options = FileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    let mut zip = ZipWriter::new(archive);

    let mut original_vsgm_len = 0;
    match OpenOptions::new().read(true).write(false).open(&vsgm_path) {
        Ok(mut vsgm_file) => {
            let vsgm_inner_path = format!("{filename}.vsgm1");

            zip.start_file(vsgm_inner_path, options).unwrap();
            let mut buffer = Vec::new();
            vsgm_file.read_to_end(&mut buffer).unwrap();
            zip.write_all(&*buffer).unwrap();

            let vsgm_metadata = vsgm_file.metadata().unwrap();
            original_vsgm_len = vsgm_metadata.len();
        }
        Err(e) => println!("Can't open {vsgm_path}: {}", e),
    }

    let mut original_vcgm_len = 0;
    let vcgm_inner_path = format!("{filename}.vcgm1");
    match OpenOptions::new().read(true).write(false).open(&vcgm_path) {
        Ok(mut vcgm_file) => {
            zip.start_file(vcgm_inner_path, options).unwrap();
            let mut buffer = Vec::new();
            vcgm_file.read_to_end(&mut buffer).unwrap();
            zip.write_all(&*buffer).unwrap();

            let vcgm_metadata = vcgm_file.metadata().unwrap();
            original_vcgm_len = vcgm_metadata.len();
        }
        Err(e) => println!("Can't open {vcgm_path}: {}", e),
    }
    let len = {
        let archive = zip.finish().unwrap();
        archive.metadata().unwrap().len()
    };
    let mut buf: Vec<u8> = Vec::with_capacity(len as usize);

    let mut archive = File::open(&filename).unwrap();
    archive.read_to_end(&mut buf).unwrap();

    // archive.read_to_end(&mut buf).unwrap();
    let compressed_len = buf.len();

    let original_len = original_vcgm_len + original_vsgm_len;

    let vcmi_command = VcmiCommand::Save {
        filename: filename.clone(),
        compressed_archive: buf,
    };
    connection
        .command_sender
        .send(vcmi_command)
        .expect("Error in another thread");
    println!(
        "Save Command.  Sended {filename}.tar {} (vec: {}) (original {}: {} + {} to gear-connector",
        len, compressed_len, original_len, original_vcgm_len, original_vsgm_len,
    );

    0
}

pub fn load_all_from_chain() -> i32 {
    let connection = try_init_connection!(connection_init);

    println!("Load all saved gamges from gear chain");

    connection
        .command_sender
        .send(VcmiCommand::LoadAll)
        .expect("Error in another thread");
    println!("Try to receive all saved games");
    let reply = connection.reply_receiver.recv().expect("Recv error");
    match reply {
        VcmiReply::AllLoaded { archives } => {
            for saved_game in archives {
                println!(
                    "Game name: {} {} bytes",
                    saved_game.filename,
                    saved_game.data.len()
                );

                let cursor = Cursor::new(saved_game.data);
                let mut archive = ZipArchive::new(cursor).unwrap();
                archive.extract("~/.local/share/vcmi/Saves/").unwrap();

                // let mut tar = OpenOptions::new()
                //     .append(true)
                //     .write(true)
                //     .create(true)
                //     .read(true)
                //     .open(saved_game.filename.as_str())
                //     .expect("Can't create file");
                // tar.write_all(&saved_game.data).unwrap();
                // let mut zip = zip::ZipArchive::new(tar).unwrap();

                // zip.extract("~./.local/share/vcmi/Saves/").unwrap();
            }
        }
        _ => unreachable!(),
    }
    0
}


fn connection_init() -> Result<Connection, std::io::Error> {
    let (command_sender, command_receiver) = bounded(1);
    let (reply_sender, reply_receiver) = bounded(1);
    let need_stop = Arc::new(AtomicBool::new(false));
    let need_stop_clone = need_stop.clone();

    let runtime = tokio::runtime::Runtime::new()?;
    let tokio_stream =
        runtime.block_on(async move { TokioTcpStream::connect("127.0.0.1:6666").await })?;

    let need_stop = need_stop_clone.clone();
    let (mut reply_read_stream, mut command_write_stream) =
        split_to_reply_read_command_write(tokio_stream);
    let read_t = runtime.spawn(async move {
        while !need_stop.load(Relaxed) {
            let command = command_receiver.recv().unwrap();
            command_write_stream
                .send(command)
                .await
                .expect("Send VCMI command Error");
        }
        println!("[Read thread] Stop listen gear-proxy")
    });

    let need_stop = need_stop_clone.clone();
    let write_t = runtime.spawn(async move {
        while !need_stop.load(Relaxed) {
            match reply_read_stream.next().await {
                Some(reply) => {
                    let reply = reply.expect("Failed to parse");
                    reply_sender.send(reply).expect("Error in another thread");
                }
                None => {}
            }
        }
        println!("[Write thread] Stop listen gear-proxy")
    });

    let connection = Connection {
        runtime,
        need_stop: need_stop_clone,
        command_sender,
        reply_receiver,
        read_t,
        write_t,
    };
    Ok(connection)
}
#[cxx::bridge]
mod ffi {
    extern "Rust" {
        fn get_file_as_byte_vec(filename: String) -> Vec<u8>;
    }

      extern "Rust" {
        fn save_state_onchain(vcgm_path: String, vsgm_path: String) -> i32;
    }

    extern "Rust" {
        fn load_all_from_chain() -> i32;
    }

    // TODO! Try to understand how to include C++ header file
    // enum ESelectionScreen {
    //     unknown, newGame, loadGame, saveGame, scenarioInfo, campaignList,
    // }
    // extern "C++" {
    //     include!("src/headers.h");
    //     type ESelectionScreen;
    // }

    #[repr(u8)]
    enum SelectionScreen {
        Unknown = 0,
        NewGame,
        LoadGame,
        SaveGame,
        ScenarioInfo,
        CampaignList,
    }
}
