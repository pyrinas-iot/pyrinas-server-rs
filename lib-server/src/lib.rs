// Lib related
pub mod broker;
pub mod bucket;
pub mod influx;
pub mod mqtt;
pub mod ota;
pub mod sock;

pub use pyrinas_shared::*;

// Async Related
use flume::{Receiver, Sender};
use std::sync::Arc;

#[cfg(feature = "runtime_tokio")]
use tokio::{join, task};

#[cfg(feature = "runtime_async_std")]
use async_macros::join;
#[cfg(feature = "runtime_async_std")]
use async_std::task;

// TODO: conditional use of tokio OR async_std
pub async fn run(
    settings: Arc<settings::PyrinasSettings>,
    broker_sender: Sender<Event>,
    broker_reciever: Receiver<Event>,
) {
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
        ota::run(task_settings, task_sender).await;
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

    // Spawn the broker task that handles it all!
    let broker_task = task::spawn(broker::run(broker_reciever));

    // Join hands kids
    let _join = join!(
        ota_db_task,
        influx_task,
        unix_sock_task,
        mqtt_server_task,
        mqtt_task,
        broker_task,
        bucket_task
    );
}
