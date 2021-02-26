// Lib related
pub mod broker;
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

// MQTT related
use librumqttd::async_locallink::construct_broker;

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

    // Ota task
    let task_sender = broker_sender.clone();
    let task_settings = settings.clone();
    let ota_db_task = task::spawn(async move {
        ota::run(task_settings, task_sender).await;
    });

    let task_settings = settings.clone();
    let ota_http_task = task::spawn(async move {
        ota::ota_http_run(task_settings).await;
    });

    // Clone these appropriately
    let task_sender = broker_sender.clone();
    let task_settings = settings.clone();

    // Start unix socket task
    let unix_sock_task = task::spawn(async move {
        sock::run(task_settings, task_sender).await;
    });

    // Set up broker
    let (mut router, _, rumqtt_server, builder) = construct_broker(settings.mqtt.rumqtt.clone());

    // Spawn router task (needs to be done before anything else or else builder.connect blocks)
    let mqtt_router_task = task::spawn_blocking(move || {
        router.start().unwrap();
    });

    // Get the rx/tx channels
    let (mut tx, mut rx) = builder.connect("localclient", 200).await.unwrap();

    // Subscribe
    tx.subscribe(settings.mqtt.topics.clone()).await.unwrap();

    // Spawn a new task(s) for the MQTT stuff
    let task_sender = broker_sender.clone();

    // Start server task
    let mqtt_server_task = task::spawn(async move {
        mqtt::mqtt_run(&mut rx, task_sender).await;
    });

    // Start mqtt broker task
    let task_sender = broker_sender.clone();
    let mqtt_task = task::spawn(async move {
        mqtt::run(&mut tx, task_sender).await;
    });

    // Spawn the broker task that handles it all!
    let broker_task = task::spawn(broker::run(broker_reciever));

    // Join hands kids
    let _join = join!(
        ota_db_task,
        ota_http_task,
        influx_task,
        unix_sock_task,
        mqtt_router_task,
        mqtt_server_task,
        mqtt_task,
        rumqtt_server,
        broker_task
    );
}
