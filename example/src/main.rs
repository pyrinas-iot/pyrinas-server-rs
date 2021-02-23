// App based module
mod application;

// Sytem related
use log::info;

// Tokio + async Related
use std::sync::Arc;
use tokio::sync::mpsc::channel;
use tokio::task;

// Command line parsing
use clap::{crate_version, Clap};

// Local lib related
use pyrinas_server::{broker, bucket, influx, mqtt, ota_db, settings, sock};
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
    let (broker_sender, broker_reciever) = channel::<pyrinas_server::Event>(100);

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

    // Start (very) basic application
    let app_task = application::run(&settings, broker_sender.clone());

    // Spawn the broker task that handles it all!
    let broker_task = task::spawn(broker::run(broker_reciever));

    // Join hands kids
    let _join = tokio::join!(
        app_task,
        ota_db_task,
        influx_task,
        unix_sock_task,
        mqtt_task,
        broker_task,
        bucket_task
    );

    info!("Done!");
}
