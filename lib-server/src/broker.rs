// System related
use log::{debug};
use std::collections::hash_map::{Entry, HashMap};

// Tokio related
use tokio::sync::mpsc::{Receiver, Sender};

// Local lib related
use pyrinas_shared::Event;

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
            debug!("Adding {} to broker.", name);
            entry.insert(sender);
          }
        }
      }
      // Handle OtaNewPackage generated by sock_run
      Event::OtaNewPackage { uid: _, package: _ } => {
        debug!("broker_run: Event::OtaNewPackage");

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
        debug!("broker_run: OtaRequest");

        // Send to sled
        runners
          .get_mut("sled")
          .unwrap()
          .send(event.clone())
          .await
          .unwrap();
      }
      Event::InfluxDataSave { query: _ } => {
        debug!("broker_run: InfluxDataSave");

        // Send to influx
        runners
          .get_mut("influx")
          .unwrap()
          .send(event.clone())
          .await
          .unwrap();
      }
      Event::OtaDeletePackage { uid: _, package: _ } => {
        debug!("broker_run: OtaDeletePackage");

        // Send to bucket handler
        runners
          .get_mut("bucket")
          .unwrap()
          .send(event.clone())
          .await
          .unwrap();
      }
      Event::ApplicationRequest { uid: _, target: _, msg: _ } => {
        debug!("broker_run: ApplicationRequest");

        // Send to app handler
        runners
          .get_mut("app")
          .unwrap()
          .send(event.clone())
          .await
          .unwrap();
      }
      Event::ApplicationResponse{ uid: _, target: _, msg: _ } => {
        debug!("broker_run: ApplicationResponse");
        // Send to mqtt handler
        runners
          .get_mut("mqtt")
          .unwrap()
          .send(event.clone())
          .await
          .unwrap();
      }
      _ => (),
    }
  }
}
