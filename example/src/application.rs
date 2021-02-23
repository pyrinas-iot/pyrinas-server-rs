// Config related
use pyrinas_server::{settings::PyrinasSettings, Event};

// Log
use log::info;

// async Related
use flume::{unbounded, Sender};
use std::sync::Arc;

pub async fn run(_settings: Arc<PyrinasSettings>, broker_sender: Sender<Event>) {
    // Get the sender/reciever associated with this particular task
    let (sender, reciever) = unbounded::<Event>();

    // Register this task
    broker_sender
        .send_async(Event::NewRunner {
            name: "app".to_string(),
            sender: sender.clone(),
        })
        .await
        .unwrap();

    // Wait for event on reciever
    while let Ok(event) = reciever.recv_async().await {
        info!("{:?}", event);
    }
}
