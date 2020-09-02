// System Related
use log::{debug, error, info, warn};

// Config related
use pyrinas_shared::settings::Settings;

// Tokio related
use tokio::sync::mpsc::{channel, Sender};
use tokio::time::{delay_for, Duration};

// Local lib related
use pyrinas_shared::{Event, OTAPackage, OtaRequestCmd};

// Todo better way of passing error..
fn get_ota_package(db: &sled::Db, uid: &str) -> Result<OTAPackage, String> {
  // Check if there's a package available and ready
  let entry = db.get(&uid);
  if entry.is_err() {
    return Err(format!("{}", entry.unwrap_err()));
  }
  let entry = entry.unwrap();

  // Raw data
  let data = entry.as_ref();
  if data.is_none() {
    return Err(format!("Unable to get u8 data."));
  }

  // Deserialize it
  let package: Result<OTAPackage, serde_cbor::error::Error> =
    serde_cbor::de::from_slice(&data.unwrap());

  // Return the result
  match package {
    Err(e) => Err(format!("{}", e)),
    Ok(p) => Ok(p),
  }
}

// Only requires a sender. No response necessary here... yet.
pub async fn run(settings: Settings, mut broker_sender: Sender<Event>) {
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

  // Open the DB
  let tree = sled::open(&settings.ota_db.path).expect("Error opening sled db.");

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
        debug!("sled_run: Event::OtaRequest");

        // Do something different depending on the situation
        match msg.cmd {
          OtaRequestCmd::Done => {
            info!("Done!");

            // Send the DeletePackage command (for S3 Bucket)
            let package = get_ota_package(&tree, &uid);
            match package {
              Ok(p) => {
                info!("Package found!");
                // Send it
                broker_sender
                  .send(Event::OtaDeletePackage {
                    uid: uid.clone(),
                    package: p,
                  })
                  .await
                  .unwrap();
              }
              Err(e) => {
                warn!("Unable to get package. Err: {}", e);
              }
            }

            // Delete entry from dB
            if let Err(e) = tree.remove(&uid) {
              error!("Unable to remove {} from OTA database. Error: {}", &uid, e);
            }
          }
          OtaRequestCmd::Check => {
            debug!("Check!");

            // Check if there's a package available and ready
            let package = get_ota_package(&tree, &uid);
            match package {
              Ok(p) => {
                info!("Package found!");
                // Send it
                broker_sender
                  .send(Event::OtaResponse {
                    uid: uid.clone(),
                    package: p,
                  })
                  .await
                  .unwrap();
              }
              Err(e) => {
                warn!("Unable to get package. Err: {}", e);
              }
            }
          }
        }
      }
      // Pprocess OtaNewPackage events
      Event::OtaNewPackage { uid, package } => {
        debug!("sled_run: Event::OtaNewPackage");

        if let Ok(entry) = tree.get(&uid) {
          // Get the u8 data
          let data = entry.as_ref();
          if data.is_some() {
            error!("Update already exists for {}.", &uid);

            // If there's someting there, no chance to update yet..
            continue;
          }
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
