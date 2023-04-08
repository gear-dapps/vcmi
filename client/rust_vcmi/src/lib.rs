mod utils;
mod io;

use gclient::{EventProcessor, EventListener, GearApi, Result};
use tokio::runtime::Runtime;
use gstd::prelude::*;

use crate::utils::get_file_as_byte_vec;
use crate::io::*;

#[no_mangle]
pub extern "C" fn rusty_extern_c_integer() -> i32 { 322 }

pub enum Error {
    Connection,
}

struct Connection {
    client: GearApi,
    listener: EventListener,
    program_id: Vec<u8>
}


static mut CONNECTION: Option<Connection> = None;
pub async fn connect_impl(program_id: String) -> Result<()> {
    unsafe { 
        if  CONNECTION.is_some() {
            println!("GClient Already Connected");
            return Ok(())}
    };
    let client = GearApi::dev().await.unwrap();
    let mut listener = client.subscribe().await?; // Subscribing for events.
    assert!(listener.blocks_running().await?);

    
    let program_id = hex::decode(&program_id[2..]).unwrap();
    println!("GClient connected. Homm3 ProgramId: {:?}", program_id);    
    unsafe {
        CONNECTION = Some(Connection { client, listener, program_id })
    }

    save_state_impl("TEST AUTOSAVE".to_string(), vec![6, 6, 6, 6, 6, 6])
        .await
        .expect("Can't save TEST data onchain");

    Ok(())
}

pub fn connect_to_contract(program_id: String) -> i32 {
    println!("Enter to Rust code");
   let _rt = Runtime::new().unwrap().block_on(
        connect_impl(program_id)
    ).expect("tokio Connect to contract Runtime Error");
    0
}

pub fn save_state_onchain(name: String, data: Vec<u8>) -> i32 {
    let _rt = Runtime::new().unwrap().block_on(
        save_state_impl(name, data)
    ).expect("tokio Save Contract State Runtime Error");
    0
}

pub async fn save_state_impl(name: String, data: Vec<u8>) -> Result<i32> {
    let Connection { client, listener, program_id } = get_connection();
    let program_id = program_id.as_slice().into();

    println!("save_state_impl(): program_id: {:?}, name: {:?}, data.len(): {}", program_id, name, data.len());
    let demo_state = GameState { name, data };
    let save_action = Action::Save(demo_state);

    let gas_limit = client
        .calculate_handle_gas(None, program_id, save_action.encode(), 0, true)
        .await.expect("Can't calculate gas for Action::Save")
        .min_limit;
    let (message_id, _) = client
        .send_message(program_id, save_action, gas_limit, 0)
        .await.expect("Error at sending Action::Save");

    // assert!(listener.message_processed(message_id).await.expect("Check processed error").succeed());
    let (_m, raw_reply, _) = listener.reply_bytes_on(message_id).await.expect("Reply bytes error");
    let raw_reply = raw_reply.unwrap();
    let decoded_reply: io::Event =
        Decode::decode(&mut raw_reply.as_slice()).expect("Can't decode reply");

    println!(
        "raw_reply {:?}, decoded_reply = {:?}, encoded Event::Saved = {:?}",
        raw_reply,
        decoded_reply,
        Event::Saved.encode()
    );
    assert_eq!(Event::Saved, decoded_reply);
    
    Ok(0)
}

fn get_connection() -> &'static mut Connection {
    let state = unsafe { CONNECTION.as_mut() };

    debug_assert!(state.is_some(), "state isn't initialized");

    unsafe { state.unwrap_unchecked() }
}

fn show_connection_dialog() {
    
}

// fn show_connection_dialog() {
//     let html_content = "<html><body><h1>Hello, World!</h1></body></html>";
// 	use web_view::Content;
//     web_view::builder()
//         .title("My Project")
//         .content(Content::Html(html_content))
//         .size(320, 480)
//         .resizable(false)
//         .debug(true)
//         .user_data(())
//         .invoke_handler(|_webview, _arg| Ok(()))
//         .run()
//         .unwrap();
// }

// fn show_connection_dialog() {
//     use fltk::{app::App, button::Button, frame::Frame, prelude::*, window::Window};

//     let app = App::default();
//     let mut wind = Window::default()
//         .with_size(160, 200)
//         .center_screen()
//         .with_label("Counter");
//     let mut frame = Frame::default()
//         .with_size(100, 40)
//         .center_of(&wind)
//         .with_label("0");
//     let mut but_inc = Button::default()
//         .size_of(&frame)
//         .above_of(&frame, 0)
//         .with_label("+");
//     let mut but_dec = Button::default()
//         .size_of(&frame)
//         .below_of(&frame, 0)
//         .with_label("-");
//     wind.make_resizable(true);
//     wind.end();
//     wind.show();
//     /* Event handling */
//     app.run().unwrap();
// }

// fn show_connection_dialog() {
//     use wry::{
//         application::{
//         event::{Event, StartCause, WindowEvent},
//         event_loop::{ControlFlow, EventLoop},
//         window::WindowBuilder,
//         },
//         webview::WebViewBuilder,
//     };

//     let event_loop = EventLoop::new();
//     let window = WindowBuilder::new()
//         .with_title("Hello World")
//         .build(&event_loop).unwrap();
//     let _webview = WebViewBuilder::new(window).unwrap()
//         .with_url("https://tauri.studio").unwrap()
//         .build().unwrap();

//     event_loop.run(move |event, _, control_flow| {
//         *control_flow = ControlFlow::Wait;

//         match event {
//         Event::NewEvents(StartCause::Init) => println!("Wry has started!"),
//         Event::WindowEvent {
//             event: WindowEvent::CloseRequested,
//             ..
//         } => *control_flow = ControlFlow::Exit,
//         _ => (),
//         }
//     });
// }


// fn show_connection_dialog() {
//     use headless_chrome::{Browser};
//     use headless_chrome::protocol::cdp::Page;

//     fn browse_wikipedia() -> Result<(), Error> {
//         let browser = Browser::default().unwrap();

//         let tab = browser.new_tab().unwrap();

//         /// Navigate to wikipedia
//         tab.navigate_to("https://www.wikipedia.org").unwrap();

//         /// Wait for network/javascript/dom to make the search-box available
//         /// and click it.
//         tab.wait_for_element("input#searchInput").unwrap().click().unwrap();

//         /// Type in a query and press `Enter`
//         tab.type_str("WebKit").unwrap().press_key("Enter").unwrap();

//         /// We should end up on the WebKit-page once navigated
//         tab.wait_for_element("#firstHeading").unwrap();
//         assert!(tab.get_url().ends_with("WebKit"));

//         /// Take a screenshot of the entire browser window
//         let _jpeg_data = tab.capture_screenshot(
//             Page::CaptureScreenshotFormatOption::Png,
//             Some(75),
//             None,
//             true).unwrap();

//         /// Take a screenshot of just the WebKit-Infobox
//         // let _png_data = tab
//         //     .wait_for_element("#mw-content-text > div > table.infobox.vevent").unwrap()
//         //     .capture_screenshot(ScreenshotFormat::PNG).unwrap();
//         Ok(())
//     }

//     assert!(browse_wikipedia().is_ok());
// }

#[cxx::bridge]
mod ffi {
    extern "Rust" { fn rusty_cxxbridge_integer() -> i32; }

    extern "Rust" {
        fn connect_to_contract(program_id: String) -> i32;
    }

    extern "Rust" {
        fn save_state_onchain(name: String, data: Vec<u8>) -> i32;
    }

    extern "Rust" {
        fn get_file_as_byte_vec(filename: String) -> Vec<u8>;
    }

    extern "Rust" {
        fn show_connection_dialog();
    }
}

pub fn rusty_cxxbridge_integer() -> i32 { 42 }