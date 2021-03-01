// System related
use log::{debug, error};

// Config related
use pyrinas_shared::{settings::PyrinasSettings,Event};

// async Related
use std::sync::Arc;
use flume::{unbounded,Sender};

#[cfg(feature = "runtime_tokio")]
use tokio_compat_02::FutureExt;

// Influx Related
use influxdb::Client;

pub async fn run(settings: Arc<PyrinasSettings>, broker_sender: Sender<Event>) {
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
  let url = format!("http://{}:{}", settings.influx.host, settings.influx.port);

  // Create the client
  let client = Client::new(url, settings.influx.database.clone()).with_auth(
    settings.influx.user.clone(),
    settings.influx.password.clone(),
  );

  // Process putting new data away
  while let Ok(event) = reciever.recv_async().await {
    // Process telemetry and app data
    match event {
      Event::InfluxDataSave(query) => {
        debug!("influx_run: InfluxDataSave");
        // Create the query. Shows error if it fails
        #[cfg(feature = "runtime_tokio")]
        if let Err(e) = client.query(&query).compat().await {
          error!("Unable to write query. Error: {}", e);
        }

        #[cfg(feature = "runtime_async_std")]
        if let Err(e) = client.query(&query).await {
          error!("Unable to write query. Error: {}", e);
        }
      }
      Event::InfluxDataRequest(_query) => {
        debug!("influx_run: InfluxDataRequest");
      }
      _ => (),
    };
  }
}
