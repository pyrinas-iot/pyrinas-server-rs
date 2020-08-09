// Sytem related
use log::info;

// Tokio Related
use tokio::sync::mpsc::channel;
use tokio::task;

// Local lib related
extern crate pyrinas_server;
use pyrinas_server::{broker, bucket, influx, mqtt, settings, sled, sock};

#[tokio::main()]
async fn main() {
    // Initialize the logger from the environment
    env_logger::init();

    // Parse config file
    let settings = settings::Settings::new().unwrap();

    // Channels for communication
    let (broker_sender, broker_reciever) = channel::<pyrinas_shared::Event>(100);

    // Init influx connection
    let influx_task = influx::run(settings.clone(), broker_sender.clone());

    // Start sled task
    let bucket_task = bucket::run(settings.clone(), broker_sender.clone());

    // Start sled task
    let sled_task = sled::run(settings.clone(), broker_sender.clone());

    // Start unix socket task
    let unix_sock_task = sock::run(settings.clone(), broker_sender.clone());

    // Spawn a new task for the MQTT stuff
    let mqtt_task = mqtt::run(settings.clone(), broker_sender.clone());

    // Spawn the broker task that handles it all!
    let broker_task = task::spawn(broker::run(broker_reciever));

    // Join hands kids
    let _join = tokio::join!(
        sled_task,
        influx_task,
        unix_sock_task,
        mqtt_task,
        broker_task,
        bucket_task
    );

    info!("Done!");
}
