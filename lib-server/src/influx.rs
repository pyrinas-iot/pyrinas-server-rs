// System related
use dotenv;
use log::{error, info};
use std::{collections::hash_map::HashMap, process};

// Tokio Related
use tokio::sync::mpsc::{channel, Sender};

// Influx Related
use influxdb::{Client, InfluxDbWriteable};

// Local lib related
use pyrinas_shared::Event;

pub async fn run(mut broker_sender: Sender<Event>) {
  // All the vars involved
  let env_vars = vec![
    String::from("PYRINAS_INFLUX_HOST"),
    String::from("PYRINAS_INFLUX_HOST_PORT"),
    String::from("PYRINAS_INFLUX_DB"),
    String::from("PYRINAS_INFLUX_USER"),
    String::from("PYRINAS_INFLUX_PASSWORD"),
  ];

  // Used for storing temporary array of input params
  let mut params = HashMap::new();

  // Iterate and get each of the environment variables
  for item in env_vars.iter() {
    let ret = dotenv::var(item).unwrap_or_else(|_| {
      error!("{} must be set in environment!", item);
      process::exit(1);
    });

    // Insert ret into map
    params.insert(item, ret);
  }

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
  let host = params.get(&env_vars[0]).unwrap();
  let port = params.get(&env_vars[1]).unwrap();
  let url = format!("http://{}:{}", host, port);

  // Get the db params
  let db_name = params.get(&env_vars[2]).unwrap();
  let user = params.get(&env_vars[3]).unwrap();
  let password = params.get(&env_vars[4]).unwrap();

  // Create the client
  let client = Client::new(url, db_name).with_auth(user, password);

  // Process putting new data away
  while let Some(event) = reciever.recv().await {
    // Process telemetry and app data
    match event {
      Event::TelemetryData { uid, msg } => {
        info!("influx_run: TelemetryData");

        // Convert to data used by influx
        let data = msg.to_influx_data(uid);

        // Query
        let query = data.into_query("telemetry");

        // Create the query. Shows error if it fails
        if let Err(e) = client.query(&query).await {
          error!("Unable to write query. Error: {}", e);
        }
      }
      Event::ApplicationData { uid: _, msg: _ } => {
        info!("influx_run: ApplicationData");
      }
      _ => (),
    };
  }
}
