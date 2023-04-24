use std::fmt::{self};

use tauri::Window;
use tracing::{
    field::{Field, Visit},
    span, Event, Metadata, Subscriber,
};
use tracing_core::Level;
use tracing_subscriber::{layer::Context, registry::LookupSpan, Layer};

pub struct MainWindowSubscriber {
    pub window: Window,
}
struct MessageFormatter {
    message: String,
}

impl fmt::Display for MessageFormatter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

pub struct StringVisitor<'a> {
    string: &'a mut String,
}

impl<'a> Visit for StringVisitor<'a> {
    fn record_debug(&mut self, _field: &Field, value: &dyn fmt::Debug) {
        use std::fmt::Write;
        write!(self.string, "{:?}", value).unwrap();
    }
    fn record_str(&mut self, _field: &Field, value: &str) {
        use std::fmt::Write;
        write!(self.string, "{}", value).unwrap();
    }
}

impl<S> Layer<S> for MainWindowSubscriber
where
    S: Subscriber + for<'span> LookupSpan<'span>,
{
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let mut string = String::new();
        let mut visitor = StringVisitor {
            string: &mut string,
        };
        event.record(&mut visitor);
        match event.metadata().level() {
            &Level::DEBUG => {
                // self.main_window.emit("debug", string).unwrap();
            },
            &Level::INFO => {
                self.window.emit("log", string).unwrap();
            },
            &Level::WARN => {
                self.window.emit("warn", string).unwrap();
            }
            &Level::ERROR => {
                self.window.emit("error", string).unwrap();
            },
            _ => {}
        }
    }
}

impl Subscriber for MainWindowSubscriber {
    fn new_span(&self, _span: &span::Attributes) -> span::Id {
        println!("NEW SPAN");
        // self.main_window.emit("log", )

        println!("VALUE = {}", _span.values().to_string());

        span::Id::from_u64(1)
    }

    fn record(&self, _: &span::Id, _: &span::Record) {}

    fn record_follows_from(&self, _: &span::Id, _: &span::Id) {}

    fn enabled(&self, _: &Metadata) -> bool {
        true
    }

    fn enter(&self, _: &span::Id) {}

    fn exit(&self, _: &span::Id) {}

    fn event(&self, event: &tracing::Event<'_>) {
        println!("EVENT: {:?}", event);
        // let s: String =     format!()
    }
}
