// Pyrinas related
use pyrinas_codec_example::EnvironmentData;
use pyrinas_server::{settings::PyrinasSettings, Event};

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
        log::info!("{:?}", event);

        // Match the event. The only one we're interested in is the `ApplicationRequest`
        if let Event::ApplicationRequest(req) = event {
            if req.target.as_str() == "env" {
                // Deserialize data from MQTT clients
                let msg: EnvironmentData = minicbor::decode(&req.msg)?;

                log::info!("{:?}", msg);
            }
        }
    }
}
