// Config related
use pyrinas_shared::{self, settings::PyrinasSettings, Event};

// Log
use log::info;

// Tokio + Async Related
use std::sync::Arc;
use tokio::sync::mpsc::{channel, Sender};

pub async fn run(_settings: &Arc<PyrinasSettings>, mut broker_sender: Sender<Event>) {
  // Get the sender/reciever associated with this particular task
  let (sender, mut reciever) = channel::<pyrinas_shared::Event>(20);

  // Register this task
  broker_sender
    .send(Event::NewRunner {
      name: "app".to_string(),
      sender: sender.clone(),
    })
    .await
    .unwrap();

  // Wait for event on reciever
  while let Some(event) = reciever.recv().await {
    info!("{:?}", event);
  }
}
