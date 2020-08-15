// System related
use log::{debug, error};

// Config related
use crate::settings::Settings;

// Tokio Related
use tokio::sync::mpsc::{channel, Sender};

// Influx Related
use influxdb::Client;

// Local lib related
use pyrinas_shared::Event;

pub async fn run(settings: Settings, mut broker_sender: Sender<Event>) {
  // Get the sender/reciever associated with this particular task
  let (sender, mut reciever) = channel::<pyrinas_shared::Event>(20);

  // Register this task
  broker_sender
    .send(Event::NewRunner {
      name: "influx".to_string(),
      sender: sender.clone(),
    })
    .await
    .unwrap();

  // Set up the URL
  let url = format!("http://{}:{}", settings.influx.host, settings.influx.port);

  // Create the client
  let client = Client::new(url, settings.influx.database.clone()).with_auth(
    settings.influx.user.clone(),
    settings.influx.password.clone(),
  );

  // Process putting new data away
  while let Some(event) = reciever.recv().await {
    // Process telemetry and app data
    match event {
      Event::InfluxDataSave { query } => {
        debug!("influx_run: InfluxDataSave");
        // Create the query. Shows error if it fails
        if let Err(e) = client.query(&query).await {
          error!("Unable to write query. Error: {}", e);
        }
      }
      Event::InfluxDataRequest { query: _ } => {
        debug!("influx_run: InfluxDataRequest");
      }
      _ => (),
    };
  }
}
