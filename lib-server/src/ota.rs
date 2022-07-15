// async Related
use flume::{unbounded, Sender};

// Local lib related
use crate::{settings, Event};
use pyrinas_shared::ota::v2::{OTADownload, OTAUpdate};
use pyrinas_shared::{OtaGroupListResponse, OtaImageListResponse, OtaRequestCmd};

// Error
use crate::Error;

/// Used to organize all the trees within the OTA database
pub struct OTADatabase {
    /// Key = image ID, Value = image data
    pub images: sled::Tree,
    /// Key = device ID, Value = group ID
    pub devices: sled::Tree,
    /// Key = group ID, Value = image ID
    pub groups: sled::Tree,
}

/// Get the OTA package from database by `update_id`
pub fn get_ota_update(db: &OTADatabase, update_id: &str) -> Result<OTAUpdate, Error> {
    // Check if there's a package available and ready
    let entry = match db.images.get(&update_id)? {
        Some(e) => e,
        None => {
            return Err(Error::CustomError("No data available.".to_string()));
        }
    };

    // Deserialize it
    let package: OTAUpdate = serde_cbor::de::from_slice(&entry)?;
    Ok(package)
}

/// Get the OTA package by device ID
fn get_ota_update_by_device_id(db: &OTADatabase, device_id: &str) -> Result<OTAUpdate, Error> {
    // Get the group_id
    let group_id: String = match db.devices.get(&device_id)? {
        Some(e) => String::from_utf8(e.to_vec())?,
        None => {
            return Err(Error::CustomError(format!(
                "Unable to find device: {}",
                device_id
            )));
        }
    };

    // Get the image_id
    let image_id: String = match db.groups.get(&group_id)? {
        Some(e) => String::from_utf8(e.to_vec())?,
        None => {
            return Err(Error::CustomError(format!(
                "Unable to find group: {}",
                group_id
            )));
        }
    };

    // Check if there's a package available and ready
    let update: OTAUpdate = match db.images.get(&image_id)? {
        Some(e) => serde_cbor::from_slice(&e)?,
        None => {
            return Err(Error::CustomError("No data available.".to_string()));
        }
    };

    Ok(update)
}

/// Used to initialize the separate trees involved in the database.
/// Used for quick lookup for devices, groups and images
pub fn init_trees(db: &sled::Db) -> Result<OTADatabase, Error> {
    Ok(OTADatabase {
        images: db.open_tree("images")?,
        devices: db.open_tree("devices")?,
        groups: db.open_tree("groups")?,
    })
}

/// Function that is called outside of the thread so it can be tested separately.
pub async fn process_event(broker_sender: &Sender<Event>, db: &OTADatabase, event: &Event) {
    match event {
        // Process OtaRequests
        Event::OtaRequest { device_uid, msg } => {
            log::debug!("sled_run: Event::OtaRequest");

            // Do something different depending on the situation
            match msg.cmd {
                // Deletes the firmware association if all is well.
                // Deletes the file as well if there are no more devices with the update id
                OtaRequestCmd::Done => {
                    log::debug!("Done!");
                    // TODO: clean up here
                }
                OtaRequestCmd::Check => {
                    log::info!("Check!");

                    // Lookup
                    let package = match get_ota_update_by_device_id(db, device_uid).ok() {
                        Some(update) => match update.package {
                            Some(mut package) => {
                                package.file = None;
                                Some(package)
                            }
                            None => {
                                log::warn!("No package found!");
                                None
                            }
                        },
                        None => None,
                    };

                    // Map the OTA update depending on version
                    let update = OTAUpdate {
                        device_uid: Some(device_uid.clone()),
                        package,
                    };

                    // Send it
                    broker_sender
                        .send_async(Event::OtaResponse(update))
                        .await
                        .unwrap();
                }
                OtaRequestCmd::DownloadBytes => {
                    let update_id = match &msg.id {
                        Some(v) => v,
                        None => {
                            log::warn!("Start position invalid!");
                            return;
                        }
                    };

                    let update = match get_ota_update(db, update_id) {
                        Ok(p) => p,
                        Err(e) => {
                            log::warn!("File not found! Err: {}", e);
                            return;
                        }
                    };

                    let mut data: OTADownload = OTADownload {
                        start_pos: match msg.start_pos {
                            Some(v) => v,
                            None => {
                                log::warn!("Start position invalid!");
                                return;
                            }
                        },
                        end_pos: match msg.end_pos {
                            Some(v) => v,
                            None => {
                                log::warn!("End position invalid!");
                                return;
                            }
                        },
                        device_uid: Some(device_uid.to_string()),
                        ..Default::default()
                    };

                    // Get slice of binary
                    data.data = match update.package {
                        Some(package) => {
                            let mut file = match package.file {
                                Some(f) => f,
                                None => {
                                    log::warn!("End position invalid!");
                                    return;
                                }
                            };

                            if data.end_pos > file.data.len() - 1 {
                                log::warn!(
                                    "Out of bounds! Start: {} End: {}",
                                    data.start_pos,
                                    data.end_pos
                                );
                                return;
                            }

                            file.data.drain(data.start_pos..data.end_pos).collect()
                        }
                        None => {
                            log::warn!("No image data!");
                            return;
                        }
                    };

                    // Get length
                    data.len = data.data.len();

                    log::info!("Data: {} {} {}", data.start_pos, data.end_pos, data.len);

                    // Send it
                    broker_sender
                        .send_async(Event::OtaDownloadResponse(data))
                        .await
                        .unwrap();
                }
            }
        }

        Event::OtaUnlink {
            device_id,
            group_id,
        } => {
            // Match the different possiblities
            match (&device_id, &group_id) {
                (None, Some(g)) => {
                    if dissociate_group(db, g).await.is_err() {
                        log::warn!("Unable to disassociate group: {}", g);
                    }
                }
                (Some(d), None) => {
                    if dissociate_device(db, d).await.is_err() {
                        log::warn!("Unable to disassociate device: {}", d);
                    }
                }
                (Some(d), Some(g)) => {
                    if dissociate_group(db, g).await.is_err() {
                        log::warn!("Unable to disassociate group: {}", g);
                    }

                    if dissociate_device(db, d).await.is_err() {
                        log::warn!("Unable to disassociate device: {}", d);
                    }
                }
                _ => {}
            };
        }
        Event::OtaLink {
            device_id,
            group_id,
            image_id,
        } => {
            // Match the different possiblities
            match (&device_id, &group_id, &image_id) {
                (None, Some(group), Some(update)) => {
                    // Connect group -> image
                    if let Err(err) = associate_group_with_update(db, group, update).await {
                        log::error!(
                            "Unable to associate {} with {}. Err: {}",
                            group,
                            update,
                            err
                        );
                        return;
                    }
                }
                (Some(device), Some(group), None) => {
                    // connect device -> group
                    if let Err(err) = associate_device_with_group(db, device, group).await {
                        log::error!(
                            "Unable to associate {} with {}. Err: {}",
                            device,
                            group,
                            err
                        );
                        return;
                    }
                }
                (Some(device), Some(group), Some(update)) => {
                    // connect device -> group
                    if let Err(err) = associate_device_with_group(db, device, group).await {
                        log::error!(
                            "Unable to associate {} with {}. Err: {}",
                            device,
                            group,
                            err
                        );
                        return;
                    }

                    // connect group -> image
                    if let Err(err) = associate_group_with_update(db, group, update).await {
                        log::error!(
                            "Unable to associate {} with {}. Err: {}",
                            group,
                            update,
                            err
                        );
                        return;
                    }
                }
                _ => {
                    log::warn!(
                        "Unsupported associate command: {:?} {:?} {:?}",
                        device_id,
                        group_id,
                        image_id
                    );
                    return;
                }
            }

            // If a device has been pushed, send that device the update
            if let Some(device_id) = device_id {
                // Gather update information and then send it off to the device
                let mut update = match get_ota_update_by_device_id(db, device_id) {
                    Ok(u) => u,
                    Err(e) => {
                        log::warn!("Unable to get OTA package: Error: {}", e);
                        return;
                    }
                };

                // Set device id
                update.device_uid = Some(device_id.to_string());

                // Remove the file contents
                update.package = match update.package {
                    Some(mut p) => {
                        p.file = None;
                        Some(p)
                    }
                    None => None,
                };

                // Notify mqtt to send update!
                broker_sender
                    .send_async(Event::OtaResponse(update))
                    .await
                    .unwrap();
            }
        }
        // Process OtaNewPackage events
        Event::OtaNewPackage(update) => {
            log::debug!("sled_run: Event::OtaNewPackage");

            log::debug!("{:?}", update);

            // Save the OTA package to database
            if let Err(e) = save_ota_update(db, update).await {
                log::error!("Unable to save OTA package. Error: {}", e);
            }
        }
        Event::OtaDeletePackage(update_id) => {
            match update_id.as_str() {
                // Delete all option
                "*" => {
                    if let Err(e) = delete_all_ota_data(db).await {
                        log::warn!("Unable to delete all ota data. Err: {}", e);
                    }
                }
                // Delete a signle update
                _ => {
                    // Delete by ID
                    if let Err(e) = delete_ota_package(db, update_id).await {
                        log::warn!("Unable to remove ota package for {}. Err: {}", update_id, e);
                    };
                }
            };
        }
        Event::OtaUpdateImageListRequest() => {
            let mut response = OtaImageListResponse { images: Vec::new() };

            for image in db.images.into_iter().flatten() {
                let (k, v) = image;

                // Deserialize
                let key = match String::from_utf8(k.to_vec()) {
                    Ok(k) => k,
                    Err(_) => continue,
                };

                // Deserialize
                let value: OTAUpdate = match serde_cbor::from_slice(&v) {
                    Ok(v) => v,
                    Err(_) => continue,
                };

                let package = match value.package {
                    Some(p) => p,
                    None => continue,
                };

                // Add the image
                response.images.push((key, package));
            }

            // Notify mqtt to send update!
            broker_sender
                .send_async(Event::OtaUpdateImageListRequestResponse(response))
                .await
                .unwrap();
        }
        Event::OtaUpdateGroupListRequest() => {
            let mut response = OtaGroupListResponse { groups: Vec::new() };

            for image in db.groups.into_iter().flatten() {
                let (k, _v) = image;

                // Deserialize
                let key = match String::from_utf8(k.to_vec()) {
                    Ok(k) => k,
                    Err(_) => continue,
                };

                // Add the image
                response.groups.push(key);
            }

            // Notify mqtt to send update!
            broker_sender
                .send_async(Event::OtaUpdateGroupListRequestResponse(response))
                .await
                .unwrap();
        }
        _ => (),
    }
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
    let db = init_trees(&db).expect("Unable to create OTA db trees.");

    // Wait for event on reciever
    while let Ok(event) = reciever.recv_async().await {
        process_event(&broker_sender, &db, &event).await;
    }
}

async fn dissociate_device(db: &OTADatabase, device_id: &str) -> Result<(), Error> {
    // Delete entry from dB
    db.devices.remove(&device_id)?;
    db.devices.flush_async().await?;

    Ok(())
}

async fn dissociate_group(db: &OTADatabase, group_id: &str) -> Result<(), Error> {
    // Delete entry from dB
    db.groups.remove(&group_id)?;
    db.groups.flush_async().await?;

    Ok(())
}

async fn delete_all_ota_data(db: &OTADatabase) -> Result<(), Error> {
    // Clear them first
    db.images.clear()?;
    db.images.flush_async().await?;
    Ok(())
}

/// Associate device_id with group_id
async fn associate_device_with_group(
    db: &OTADatabase,
    device_id: &str,
    group_id: &str,
) -> Result<(), Error> {
    // Insert the encoded data back into the db
    db.devices.insert(&device_id, group_id.as_bytes())?;
    db.devices.flush_async().await?;

    Ok(())
}

/// Associate group_id with update_id
async fn associate_group_with_update(
    db: &OTADatabase,
    group_id: &str,
    update_id: &str,
) -> Result<(), Error> {
    // Check if insert worked ok
    db.groups.insert(&group_id, update_id.as_bytes())?;
    db.groups.flush_async().await?;

    Ok(())
}

/// Creates the OTA package in the database and filesystem.
///
/// This function overwrites any updates that may exist
pub async fn save_ota_update(db: &OTADatabase, update: &OTAUpdate) -> Result<(), Error> {
    // Get the package
    let package = match &update.package {
        Some(p) => p,
        None => return Err(Error::CustomError("Package must exist!".to_string())),
    };

    // Turn entry.package into CBOR
    let cbor_data = serde_cbor::ser::to_vec_packed(&update)?;

    // Check if insert worked ok
    db.images.insert(&package.to_string(), cbor_data)?;
    db.images.flush_async().await?;

    Ok(())
}

/// Deletes the OTA package from the database and filesystem.
pub async fn delete_ota_package(db: &OTADatabase, update_id: &str) -> Result<(), Error> {
    // Delete entry from dB
    db.images.remove(&update_id)?;
    db.images.flush_async().await?;

    Ok(())
}
