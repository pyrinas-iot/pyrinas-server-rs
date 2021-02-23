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

    // Clone these appropriately
    let task_sender = broker_sender.clone();
    let task_settings = settings.clone();

    // Init influx connection
    let influx_task = task::spawn(async move {
        influx::run(task_settings, task_sender).await;
    });

    // Clone these appropriately
    let task_sender = broker_sender.clone();
    let task_settings = Arc::clone(&settings);

    // Start sled task
    let bucket_task = task::spawn(async move {
        bucket::run(task_settings, task_sender).await;
    });

    // Start sled task
    let task_sender = broker_sender.clone();
    let task_settings = settings.clone();
    let ota_db_task = task::spawn(async move {
        ota_db::run(task_settings, task_sender).await;
    });

    // Clone these appropriately
    let task_sender = broker_sender.clone();
    let task_settings = settings.clone();

    // Start unix socket task
    let unix_sock_task = task::spawn(async move {
        sock::run(task_settings, task_sender).await;
    });

    // Spawn a new task(s) for the MQTT stuff
    let task_sender = broker_sender.clone();
    let task_settings = settings.clone();

    // Get eventloop stuff
    let mut mqtt_eventloop = mqtt::setup(task_settings).await;
    let mut mqtt_sender = mqtt_eventloop.handle();

    // Start server task
    let mqtt_server_task = task::spawn(async move {
        mqtt::mqtt_run(&mut mqtt_eventloop, task_sender).await;
    });

    // Start mqtt task
    let task_sender = broker_sender.clone();
    let mqtt_task = task::spawn(async move {
        mqtt::run(&mut mqtt_sender, task_sender).await;
    });

    // Start (very) basic application
    let task_settings = settings.clone();
    let app_task = task::spawn(application::run(task_settings, broker_sender));

    // Spawn the broker task that handles it all!
    let broker_task = task::spawn(broker::run(broker_reciever));

    // Join hands kids
    let _join = tokio::join!(
        app_task,
        ota_db_task,
        influx_task,
        unix_sock_task,
        mqtt_server_task,
        mqtt_task,
        broker_task,
        bucket_task
    );

    info!("Done!");
}
