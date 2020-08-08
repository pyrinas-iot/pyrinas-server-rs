use dotenv;
use log::{debug, error, info};
use std::process;

use tokio::sync::mpsc::{channel, Sender};
use tokio::time::{delay_for, Duration};

// Local lib related
use pyrinas_shared::{Event, OTAPackage, OtaRequestCmd};

pub async fn run(mut broker_sender: Sender<Event>) {
  // Get the sender/reciever associated with this particular task
  let (mut sender, mut reciever) = channel::<pyrinas_shared::Event>(20);

  // Register this task
  broker_sender
    .send(Event::NewRunner {
      name: "sled".to_string(),
      sender: sender.clone(),
    })
    .await
    .unwrap();

  let sled_db = dotenv::var("PYRINAS_SLED_DB").unwrap_or_else(|_| {
    error!("PYRINAS_SLED_DB must be set in environment!");
    process::exit(1);
  });

  // Open the DB
  let tree = sled::open(sled_db).expect("Error opening sled db.");

  // TODO: smarter way to do this?
  tokio::spawn(async move {
    loop {
      // Delay
      delay_for(Duration::from_secs(10)).await;

      // Flush the database
      sender.send(Event::SledFlush).await.unwrap();
    }
  });

  // TODO: wait for event on reciever
  while let Some(event) = reciever.recv().await {
    match event {
      Event::SledFlush => {
        debug!("sled_run: Event::SledFlush");

        // Save it to disk
        if let Err(e) = tree.flush_async().await {
          error!("Unable to flush tree. Error: {}", e);
        }
      }
      // Process OtaRequests
      Event::OtaRequest { uid, msg } => {
        info!("sled_run: Event::OtaRequest");

        // Do something different depending on the situation
        match msg.cmd {
          OtaRequestCmd::Done => {
            info!("Done!");

            // Delete entry from dB
            if let Err(e) = tree.remove(&uid) {
              error!("Unable to remove {} from OTA database. Error: {}", &uid, e);
            }

            // TODO: send signal to delete it also from S3
          }
          OtaRequestCmd::Check => {
            info!("Check!");

            // Check if there's a package available and ready
            if let Ok(entry) = tree.get(&uid) {
              // Raw data
              let data = entry.unwrap();
              let data = data.as_ref();

              // Deserialize it
              let package: Result<OTAPackage, serde_cbor::error::Error> =
                serde_cbor::de::from_slice(&data);
              if package.is_err() {
                error!("Unable to deserialize data!");
                continue;
              }

              // Send it
              broker_sender
                .send(Event::OtaResponse {
                  uid: uid,
                  package: package.unwrap(),
                })
                .await
                .unwrap();
            }
          }
        }
      }
      // Pprocess OtaNewPackage events
      Event::OtaNewPackage { uid, package } => {
        info!("sled_run: Event::OtaNewPackage");

        if let Ok(_) = tree.get(&uid) {
          error!("Update already exists for {}.", &uid);

          // TODO: return error somehow..
          continue;
        }

        // Turn entry.package into CBOR
        let res = serde_cbor::ser::to_vec_packed(&package);

        // Write into database
        match res {
          Ok(cbor_data) => {
            // Check if insert worked ok
            if let Err(e) = tree.insert(&uid, cbor_data) {
              error!("Unable to insert into sled. Error: {}", e);
              continue;
            }

            // Notify mqtt to send update!
            broker_sender
              .send(Event::OtaResponse {
                uid: uid,
                package: package,
              })
              .await
              .unwrap();
          }
          Err(e) => {
            error!("Unable to serialize. Error: {}", e);
          }
        }
      }
      _ => (),
    }
  }
}
