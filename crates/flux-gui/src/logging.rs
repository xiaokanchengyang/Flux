//! Custom tracing subscriber for GUI integration

use crossbeam_channel::Sender;
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::{
    fmt::{format::FmtSpan, time::ChronoLocal},
    layer::{Context, Layer},
    registry::LookupSpan,
};
use std::fmt;
use std::sync::Arc;

/// A tracing layer that sends log messages to the GUI
pub struct GuiLogLayer {
    sender: Arc<Sender<(Level, String)>>,
}

impl GuiLogLayer {
    pub fn new(sender: Sender<(Level, String)>) -> Self {
        Self {
            sender: Arc::new(sender),
        }
    }
}

impl<S> Layer<S> for GuiLogLayer
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        // Format the event message
        let mut message = String::new();
        
        // Get event metadata
        let metadata = event.metadata();
        let level = metadata.level();
        
        // Format level with color-like prefix
        let level_str = match *level {
            Level::ERROR => "ERROR",
            Level::WARN => "WARN",
            Level::INFO => "INFO",
            Level::DEBUG => "DEBUG",
            Level::TRACE => "TRACE",
        };
        
        // Add level and target
        message.push_str(&format!("[{}] ", level_str));
        
        if let Some(target) = metadata.target().split("::").last() {
            message.push_str(&format!("{}: ", target));
        }
        
        // Format the event fields
        let mut visitor = MessageVisitor(&mut message);
        event.record(&mut visitor);
        
        // Send to GUI (ignore errors if channel is closed)
        let _ = self.sender.send((*level, message));
    }
}

/// Visitor to format event fields
struct MessageVisitor<'a>(&'a mut String);

impl<'a> tracing::field::Visit for MessageVisitor<'a> {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn fmt::Debug) {
        if field.name() == "message" {
            self.0.push_str(&format!("{:?}", value));
        } else {
            self.0.push_str(&format!(" {}={:?}", field.name(), value));
        }
    }
    
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            self.0.push_str(value);
        } else {
            self.0.push_str(&format!(" {}=\"{}\"", field.name(), value));
        }
    }
}

/// Initialize tracing with both console and GUI output
pub fn init_tracing(gui_sender: Option<Sender<(Level, String)>>) {
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
    
    let env_filter = EnvFilter::from_default_env()
        .add_directive("flux_gui=debug".parse().unwrap())
        .add_directive("flux_core=info".parse().unwrap());
    
    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_timer(ChronoLocal::new("%H:%M:%S%.3f".to_string()))
        .with_target(false)
        .with_span_events(FmtSpan::CLOSE);
    
    let subscriber = tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer);
    
    if let Some(sender) = gui_sender {
        let gui_layer = GuiLogLayer::new(sender);
        subscriber.with(gui_layer).init();
    } else {
        subscriber.init();
    }
}