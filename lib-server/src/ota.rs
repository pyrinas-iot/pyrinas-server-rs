// System Related
use log::{debug, error, warn};

// async Related
use flume::{unbounded, Sender};
use std::sync::Arc;

// Local lib related
use pyrinas_shared::settings::PyrinasSettings;
use pyrinas_shared::{Event, OTAPackage, OtaRequestCmd, OtaUpdate};

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
pub async fn run(settings: Arc<PyrinasSettings>, broker_sender: Sender<Event>) {
    // Get the sender/reciever associated with this particular task
    let (sender, reciever) = unbounded::<Event>();

    // Register this task
    broker_sender
        .send_async(Event::NewRunner {
            name: "ota".to_string(),
            sender: sender.clone(),
        })
        .await
        .unwrap();

    // Open the DB
    let tree = sled::open(&settings.ota_db.path).expect("Error opening sled db.");

    // TODO: wait for event on reciever
    while let Ok(event) = reciever.recv_async().await {
        match event {
            // Process OtaRequests
            Event::OtaRequest { uid, msg } => {
                debug!("sled_run: Event::OtaRequest");

                // Do something different depending on the situation
                match msg.cmd {
                    OtaRequestCmd::Done => {
                        debug!("Done!");

                        // Send the DeletePackage command (for S3 Bucket)
                        let package = get_ota_package(&tree, &uid).ok();

                        // Send it
                        broker_sender
                            .send_async(Event::OtaDeletePackage(OtaUpdate {
                                uid: uid.clone(),
                                package: package,
                            }))
                            .await
                            .unwrap();

                        // Delete entry from dB
                        if let Err(e) = tree.remove(&uid) {
                            error!("Unable to remove {} from OTA database. Error: {}", &uid, e);
                        }
                    }
                    OtaRequestCmd::Check => {
                        debug!("Check!");

                        // Check if there's a package available and ready
                        let package = get_ota_package(&tree, &uid).ok();

                        // Send it
                        broker_sender
                            .send_async(Event::OtaResponse(OtaUpdate {
                                uid: uid.clone(),
                                package: package,
                            }))
                            .await
                            .unwrap();
                    }
                }
            }
            // Pprocess OtaNewPackage events
            Event::OtaNewPackage(update) => {
                debug!("sled_run: Event::OtaNewPackage");

                if let Ok(entry) = tree.get(&update.uid) {
                    // Get the u8 data
                    let data = entry.as_ref();
                    if data.is_some() {
                        warn!("Update already exists for {}.", &update.uid);

                        // Remove
                        if let Err(e) = tree.remove(&update.uid) {
                            warn!("Unable to delete OTA entry. Error: {}", e);
                            continue;
                        }

                        // Save it to disk
                        if let Err(e) = tree.flush_async().await {
                            error!("Unable to flush tree. Error: {}", e);
                        }
                    }
                }

                // Turn entry.package into CBOR
                let res = serde_cbor::ser::to_vec_packed(&update.package);

                // Write into database
                match res {
                    Ok(cbor_data) => {
                        // Check if insert worked ok
                        if let Err(e) = tree.insert(&update.uid, cbor_data) {
                            error!("Unable to insert into sled. Error: {}", e);
                            continue;
                        }

                        // Save it to disk
                        if let Err(e) = tree.flush_async().await {
                            error!("Unable to flush tree. Error: {}", e);
                        }

                        // Notify mqtt to send update!
                        broker_sender
                            .send_async(Event::OtaResponse(update))
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
