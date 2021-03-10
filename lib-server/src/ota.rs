// async Related
use flume::{unbounded, Sender};
use std::io::Write;
use std::net::{Ipv4Addr, SocketAddrV4};

// Std
use std::fs::{self, File};

// Anyhow
use anyhow::{anyhow, Result};

// Local lib related
use pyrinas_shared::settings;
use pyrinas_shared::{Event, OTAPackage, OtaRequestCmd, OtaUpdate};

// warp
use warp::{self, Filter};

// Static image folder path
const IMAGE_FOLDER_PATH: &str = "./_images";

// Get the OTA package from database
// TODO: make this consistent with other calls..
fn get_ota_package(db: &sled::Db, update: &OtaUpdate) -> Result<OTAPackage> {
    // Check if there's a package available and ready
    let entry = match db.get(&update.uid)? {
        Some(e) => e,
        None => {
            return Err(anyhow!("No data available."));
        }
    };

    // Deserialize it
    let package: OTAPackage = serde_cbor::de::from_slice(&entry)?;
    Ok(package)
}

// Only requires a sender. No response necessary here... yet.
pub async fn run(settings: &settings::Ota, broker_sender: Sender<Event>) {
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
    let db = sled::open(&settings.db_path).expect("Error opening sled db.");

    // Wait for event on reciever
    while let Ok(event) = reciever.recv_async().await {
        match event {
            // Process OtaRequests
            Event::OtaRequest { uid, msg } => {
                log::debug!("sled_run: Event::OtaRequest");

                // Do something different depending on the situation
                match msg.cmd {
                    OtaRequestCmd::Done => {
                        log::debug!("Done!");

                        // Send the DeletePackage command (for S3 Bucket)
                        let package = match get_ota_package(
                            &db,
                            &OtaUpdate {
                                uid: uid.clone(),
                                package: None,
                                image: None,
                            },
                        ) {
                            Ok(p) => p,
                            Err(e) => {
                                log::info!("Unable to get OTA package: Error: {}", e);
                                continue;
                            }
                        };

                        // Send it
                        broker_sender
                            .send_async(Event::OtaDeletePackage(OtaUpdate {
                                uid: uid.clone(),
                                package: Some(package),
                                image: None,
                            }))
                            .await
                            .unwrap();
                    }
                    OtaRequestCmd::Check => {
                        log::debug!("Check!");

                        // Check if there's a package available and ready
                        let package = get_ota_package(
                            &db,
                            &OtaUpdate {
                                uid: uid.clone(),
                                package: None,
                                image: None,
                            },
                        )
                        .ok();

                        // Send it
                        broker_sender
                            .send_async(Event::OtaResponse(OtaUpdate {
                                uid: uid.clone(),
                                package: package,
                                image: None,
                            }))
                            .await
                            .unwrap();
                    }
                }
            }
            // Pprocess OtaNewPackage events
            Event::OtaNewPackage(update) => {
                log::debug!("sled_run: Event::OtaNewPackage");

                // Save image to file before we muck with the OtaUpdate
                match update.image {
                    Some(i) => {
                        if let Err(e) = save_ota_firmware_image(&update.uid, &i).await {
                            log::error!("Unable to save OTA firmware image. Err: {}", e);
                            continue;
                        }
                    }
                    None => {
                        log::error!("Image not valid!");
                        continue;
                    }
                };

                // Now reconfigure the pakcage to include the file and host path
                let package = match update.package {
                    Some(p) => {
                        // Need to set the file path.
                        let mut pack = p;
                        pack.file = format!("images/{}.bin", &update.uid);

                        // Set the server url
                        pack.host = settings.url.clone();

                        Some(pack)
                    }
                    None => None,
                };

                // Copy only useful stuff in update (no image binary data.)
                let update = OtaUpdate {
                    uid: update.uid,
                    package: package,
                    image: None,
                };

                // Save the OTA package to database
                if let Err(e) = save_ota_package(&db, &update).await {
                    log::error!("Unable to save OTA package. Error: {}", e);
                    continue;
                }

                // Notify mqtt to send update!
                broker_sender
                    .send_async(Event::OtaResponse(update))
                    .await
                    .unwrap();
            }
            Event::OtaDeletePackage(update) => {
                log::debug!("bucket_run: OtaDeletePackage");

                if let Err(e) = delete_ota_firmware_image(&update.uid).await {
                    log::warn!("Unable to delete OTA firmwar image: Error: {}", e);
                }

                if let Err(e) = delete_ota_package(&db, &update).await {
                    log::warn!("Unable to delete OTA package: Error: {}", e);
                }
            }
            _ => (),
        }
    }
}

/// Take binary data and save it to the image directory..
async fn save_ota_firmware_image(name: &str, image: &Vec<u8>) -> Result<()> {
    let mut file = File::create(format!("{}/{}.bin", IMAGE_FOLDER_PATH, name))?;

    // Write and sync
    file.write_all(&image)?;
    file.sync_all()?;

    Ok(())
}

async fn delete_ota_firmware_image(name: &str) -> Result<()> {
    // Delete from filesystem
    fs::remove_file(format!("{}/{}.bin", IMAGE_FOLDER_PATH, &name))?;

    Ok(())
}

/// Creates the OTA package in the database and filesystem.
async fn save_ota_package(db: &sled::Db, update: &OtaUpdate) -> Result<()> {
    // Make directory if it doesn't exist...
    if let Err(e) = fs::create_dir_all(IMAGE_FOLDER_PATH) {
        log::warn!("Unable to create image directory: {}", e);
    }

    if let Ok(entry) = db.get(&update.uid) {
        // Get the u8 data
        let data = entry.as_ref();
        if data.is_some() {
            log::warn!("Update already exists for {}.", &update.uid);

            // Remove
            db.remove(&update.uid)?;
            db.flush_async().await?;
        }
    }

    // Turn entry.package into CBOR
    let cbor_data = serde_cbor::ser::to_vec_packed(&update.package)?;

    // Check if insert worked ok
    db.insert(&update.uid, cbor_data)?;
    db.flush_async().await?;

    Ok(())
}

/// Deletes the OTA package from the database and filesystem.
async fn delete_ota_package(db: &sled::Db, update: &OtaUpdate) -> Result<()> {
    // Delete entry from dB
    db.remove(&update.uid)?;
    db.flush_async().await?;

    Ok(())
}

/// Small server with one endpoint for handling OTA updates.
/// i.e. hosts static firmware images that can be pulled by the
/// firmware.
pub async fn ota_http_run(settings: &settings::Ota) {
    // TODO: for async-std use `tide`
    // TODO: API key for more secure transfers

    // Only one folder that we're interested in..
    let images = warp::path("images").and(warp::fs::dir(&IMAGE_FOLDER_PATH));

    // Run the `warp` server
    warp::serve(images)
        .run(SocketAddrV4::new(
            Ipv4Addr::new(127, 0, 0, 1),
            settings.http_port,
        ))
        .await;
}

#[cfg(test)]
mod tests {
    use pyrinas_shared::OTAPackageVersion;

    use super::*;
    use std::sync::Once;

    static INIT: Once = Once::new();

    /// Setup function that is only run once, even if called multiple times.
    fn setup() {
        INIT.call_once(|| env_logger::init());
    }

    #[tokio::test]
    async fn save_ota_package_sucess() {
        // Log setup
        setup();

        // Creates temporary in-memory database
        let db: sled::Db = sled::Config::new().temporary(true).open().unwrap();

        let hash: [u8; 8] = [0, 1, 2, 3, 4, 5, 6, 7];
        let image: [u8; 4] = [0, 0, 0, 0];

        // Update
        let update = OtaUpdate {
            uid: "4".to_string(),
            package: Some(OTAPackage {
                version: OTAPackageVersion {
                    major: 1,
                    minor: 0,
                    patch: 1,
                    commit: 2,
                    hash: hash,
                },
                host: "test.jaredwolff.com".to_string(),
                file: "gombo.bin".to_string(),
                force: false,
            }),
            image: Some(image.to_vec()),
        };

        // Test the save_ota_package
        if let Err(e) = save_ota_package(&db, &update).await {
            log::error!("Error: {}", e);
            assert!(false);
        }

        // Check the database to make sure there's an entry
        assert!(db.contains_key(&update.uid).unwrap());

        // check to make sure the file exists int he correct folder.
        assert!(
            std::path::Path::new(&format!("{}/{}.bin", IMAGE_FOLDER_PATH, update.uid)).exists()
        );
    }

    #[tokio::test]
    async fn get_ota_package_success() {
        // Log setup
        setup();

        // Creates temporary in-memory database
        let db: sled::Db = sled::Config::new().temporary(true).open().unwrap();

        let hash: [u8; 8] = [0, 1, 2, 3, 4, 5, 6, 7];
        let image: [u8; 4] = [0, 0, 0, 0];

        let package = OTAPackage {
            version: OTAPackageVersion {
                major: 1,
                minor: 0,
                patch: 1,
                commit: 2,
                hash: hash,
            },
            host: "test.jaredwolff.com".to_string(),
            file: "gombo.bin".to_string(),
            force: false,
        };

        // Update
        let update = OtaUpdate {
            uid: "3".to_string(),
            package: Some(package),
            image: Some(image.to_vec()),
        };

        // Test the save_ota_package
        if let Err(e) = save_ota_package(&db, &update).await {
            log::error!("Error: {}", e);
            assert!(false);
        }

        let update = OtaUpdate {
            uid: "3".to_string(),
            package: None,
            image: None,
        };

        // Get OTA package
        let package = match get_ota_package(&db, &update) {
            Ok(p) => p,
            Err(e) => {
                log::error!("Error getting OTA package. Error: {}", e);
                assert!(false);
                return;
            }
        };

        // Make sure everything is equal
        assert_eq!(package.file, package.file);
        assert_eq!(package.host, package.host);
        assert_eq!(package.force, package.force);
    }

    #[tokio::test]
    async fn delete_ota_package_success() {
        // Log setup
        setup();

        // Creates temporary in-memory database
        let db: sled::Db = sled::Config::new().temporary(true).open().unwrap();

        let hash: [u8; 8] = [0, 1, 2, 3, 4, 5, 6, 7];
        let image: [u8; 4] = [0, 0, 0, 0];

        let package = OTAPackage {
            version: OTAPackageVersion {
                major: 1,
                minor: 0,
                patch: 1,
                commit: 2,
                hash: hash,
            },
            host: "test.jaredwolff.com".to_string(),
            file: "gombo.bin".to_string(),
            force: false,
        };

        // Update
        let update = OtaUpdate {
            uid: "2".to_string(),
            package: Some(package),
            image: Some(image.to_vec()),
        };

        // Test the save_ota_package
        if let Err(e) = save_ota_package(&db, &update).await {
            log::error!("Error: {}", e);
            assert!(false);
        }

        // Save the image to disk
        if let Err(e) = save_ota_firmware_image(&update.uid, &image.to_vec()).await {
            log::error!("Error: {}", e);
            assert!(false);
        }

        // Delete the package
        let res = delete_ota_package(&db, &update).await;
        assert!(res.is_ok());

        let res = delete_ota_firmware_image(&update.uid).await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn delete_ota_package_failure() {
        // Log setup
        setup();

        // Creates temporary in-memory database
        let db: sled::Db = sled::Config::new().temporary(true).open().unwrap();

        let update = OtaUpdate {
            uid: "1".to_string(),
            package: None,
            image: None,
        };

        // Delete the package
        let res = delete_ota_package(&db, &update).await;
        log::info!("{:?}", res);
        assert!(res.is_ok());

        let res = delete_ota_firmware_image(&update.uid).await;
        assert!(res.is_err());
    }
}
