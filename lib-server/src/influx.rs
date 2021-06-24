// Config related
use crate::{settings, Event};

// async Related
use flume::{unbounded, Sender};

// Tokio compat
use tokio_compat_02::FutureExt;

// Influx Related
use influxdb::Client;

pub async fn run(settings: &settings::Influx, broker_sender: Sender<Event>) {
    // Get the sender/reciever associated with this particular task
    let (sender, reciever) = unbounded::<Event>();

    // Register this task
    broker_sender
        .send_async(Event::NewRunner {
            name: "influx".to_string(),
            sender: sender.clone(),
        })
        .await
        .unwrap();

    // Set up the URL
    let url = format!("http://{}:{}", settings.host, settings.port);

    // Create the client
    let client = Client::new(url, settings.database.clone())
        .with_auth(settings.user.clone(), settings.password.clone());

    // Process putting new data away
    while let Ok(event) = reciever.recv_async().await {
        // Process telemetry and app data
        match event {
            Event::InfluxDataSave(query) => {
                log::debug!("influx_run: InfluxDataSave");
                // Create the query. Shows error if it fails
                if let Err(e) = client.query(&query).compat().await {
                    log::error!("Unable to write query. Error: {}", e);
                }
            }
            Event::InfluxDataRequest(_query) => {
                log::debug!("influx_run: InfluxDataRequest");
            }
            _ => (),
        };
    }
}
