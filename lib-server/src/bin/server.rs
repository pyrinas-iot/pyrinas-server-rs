// Sytem related
use log::info;

// Tokio + async Related
use std::sync::Arc;
use tokio::sync::mpsc::channel;
use tokio::task;

// Command line parsing
use clap::{crate_version, Clap};

// Local lib related
extern crate pyrinas_server;
use pyrinas_server::{broker, bucket, influx, mqtt, ota_db, sock};
use pyrinas_shared::settings;

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

    // Get the config path
    let opts: Opts = Opts::parse();

    // Parse config file
    let settings = Arc::new(settings::PyrinasSettings::new(opts.config).unwrap());

    // Channels for communication
    let (broker_sender, broker_reciever) = channel::<pyrinas_shared::Event>(100);

    // Init influx connection
    let influx_task = influx::run(&settings, broker_sender.clone());

    // Start sled task
    let bucket_task = bucket::run(&settings, broker_sender.clone());

    // Start sled task
    let ota_db_task = ota_db::run(&settings, broker_sender.clone());

    // Start unix socket task
    let unix_sock_task = sock::run(&settings, broker_sender.clone());

    // Spawn a new task for the MQTT stuff
    let mqtt_task = mqtt::run(&settings, broker_sender.clone());

    // Spawn the broker task that handles it all!
    let broker_task = task::spawn(broker::run(broker_reciever));

    // Join hands kids
    let _join = tokio::join!(
        ota_db_task,
        influx_task,
        unix_sock_task,
        mqtt_task,
        broker_task,
        bucket_task
    );

    info!("Done!");
}
