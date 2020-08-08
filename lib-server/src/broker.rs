use dotenv;
use log::{debug, error, info, warn};
use std::fs::File;
use std::{
  collections::hash_map::{Entry, HashMap},
  env,
  io::Read,
  process, str,
};

use tokio::sync::mpsc::{channel, Receiver, Sender};

// Local lib related
use pyrinas_shared::{Event, OTAPackage, OtaRequestCmd};

pub async fn run(mut broker_reciever: Receiver<Event>) {
  let mut runners: HashMap<String, Sender<Event>> = HashMap::new();

  // Handle broker events
  while let Some(event) = broker_reciever.recv().await {
    match event.clone() {
      // Upon creating a new server thread, the thread has to register with the broker.
      Event::NewRunner { name, sender } => {
        // Check to see if the runner is already in the HashMap
        match runners.entry(name.clone()) {
          Entry::Occupied(..) => (),
          Entry::Vacant(entry) => {
            // Inserts the Sender<event> into the HashMap
            info!("Adding {} to broker.", name);
            entry.insert(sender);
          }
        }
      }
      // Handle OtaNewPackage generated by sock_run
      Event::OtaNewPackage { uid: _, package: _ } => {
        info!("broker_run: Event::OtaNewPackage");

        // Send to sled
        runners
          .get_mut("sled")
          .unwrap()
          .send(event.clone())
          .await
          .unwrap();
      }
      Event::OtaResponse { uid: _, package: _ } => {
        // Send to mqtt
        runners
          .get_mut("mqtt")
          .unwrap()
          .send(event.clone())
          .await
          .unwrap();
      }
      Event::OtaRequest { uid: _, msg: _ } => {
        info!("broker_run: OtaRequest");

        // Send to sled
        runners
          .get_mut("sled")
          .unwrap()
          .send(event.clone())
          .await
          .unwrap();
      }
      Event::TelemetryData { uid: _, msg: _ } => {
        info!("broker_run: TelemetryData");

        // Send to influx
        runners
          .get_mut("influx")
          .unwrap()
          .send(event.clone())
          .await
          .unwrap();
      }

      _ => (),
    }
  }
}
