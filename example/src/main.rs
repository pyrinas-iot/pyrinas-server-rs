// Application
mod application;

// Tokio + async Related
use std::sync::Arc;
use tokio::sync::mpsc::channel;
use tokio::task;

// Command line parsing
use clap::{crate_version, Clap};

// Local crate related
use pyrinas_server::{self, settings, Event};

/// This doc string acts as a help message when the user runs '--help'
/// as do all doc strings on fields
#[derive(Clap)]
#[clap(version = crate_version!())]
struct Opts {
    config: String,
}

#[tokio::main()]
async fn main() {
    // Initialize the logger from the environment
    env_logger::init();

    // Print out info
    println!("Pyrinas Server Version: {}", env!("CARGO_PKG_VERSION"));

    // Get the config path
    let opts: Opts = Opts::parse();

    // Parse config file
    let settings = match settings::PyrinasSettings::new(opts.config) {
        Ok(s) => Arc::new(s),
        Err(e) => {println!("Error parsing config file: {}",e); return},
    };

    // Channels for communication
    let (broker_sender, broker_reciever) = channel::<Event>(100);

    // Start (very) basic application
    let task_settings = settings.clone();
    let task_sender = broker_sender.clone();
    let app_task = task::spawn(application::run(task_settings, task_sender));

    // Start the server and all underlying tasks
    let pyrinas_task = task::spawn(pyrinas_server::run(
        settings,
        broker_sender,
        broker_reciever,
    ));

    // Join hands kids
    let _join = tokio::join!(app_task, pyrinas_task);
}
