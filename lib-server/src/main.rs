// Sytem related
use dotenv;
use log::info;

// Tokio Related
use tokio::sync::mpsc::channel;
use tokio::task;

// Local lib related
mod broker;
mod influx;
mod mqtt;
mod sled;
mod sock;

#[tokio::main()]
async fn main() {
    // Initialize the logger from the environment
    env_logger::init();

    // Parse .env file
    dotenv::dotenv().ok();

    // Channels for communication
    let (broker_sender, broker_reciever) = channel::<pyrinas_shared::Event>(100);

    // Init influx connection
    let influx_task = task::spawn(influx::run(broker_sender.clone()));

    // TODO: init http service

    // Start sled task
    let sled_task = task::spawn(sled::run(broker_sender.clone()));

    // Start unix socket task
    let unix_sock_task = task::spawn(sock::run(broker_sender.clone()));

    // Spawn a new task for the MQTT stuff
    let mqtt_task = task::spawn(mqtt::run(broker_sender.clone()));

    // Spawn the broker task that handles it all!
    let broker_task = task::spawn(broker::run(broker_reciever));

    // Join hands kids
    let _join = tokio::join!(
        sled_task,
        influx_task,
        unix_sock_task,
        mqtt_task,
        broker_task
    );

    info!("Done!");
}
