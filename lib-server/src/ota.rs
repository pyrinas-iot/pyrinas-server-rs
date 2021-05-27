// async Related
use flume::{unbounded, Sender};
use pyrinas_shared::OtaGroupListResponse;
use std::io::Write;
use std::net::{Ipv4Addr, SocketAddrV4};

// Std
use std::fs::{self, File};

// Anyhow
use anyhow::{anyhow, Result};

// Local lib related
use crate::{settings, Event};
use pyrinas_shared::{
    OTAImageData, OTAImageType, OTAPackage, OTAPackageFileInfo, OtaImageListResponse,
    OtaRequestCmd, OtaUpdate,
};

// warp
use warp::{self, Filter};

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
pub(crate) fn get_ota_package(db: &OTADatabase, update_id: &String) -> Result<OTAPackage> {
    // Check if there's a package available and ready
    let entry = match db.images.get(&update_id)? {
        Some(e) => e,
        None => {
            return Err(anyhow!("No data available."));
        }
    };

    // Deserialize it
    let package: OTAPackage = serde_cbor::de::from_slice(&entry)?;
    Ok(package)
}

/// Get the OTA package by device ID
fn get_ota_package_by_device_id(db: &OTADatabase, device_id: &String) -> Result<OTAPackage> {
    // Get the group_id
    let group_id: String = match db.devices.get(&device_id)? {
        Some(e) => String::from_utf8(e.to_vec())?,
        None => {
            return Err(anyhow!("Unable to find device: {}", device_id));
        }
    };

    // Get the image_id
    let image_id: String = match db.groups.get(&group_id)? {
        Some(e) => String::from_utf8(e.to_vec())?,
        None => {
            return Err(anyhow!("Unable to find group: {}", group_id));
        }
    };

    // Check if there's a package available and ready
    let package: OTAPackage = match db.images.get(&image_id)? {
        Some(e) => serde_cbor::from_slice(&e)?,
        None => {
            return Err(anyhow!("No data available."));
        }
    };

    Ok(package)
}

/// Used to initialize the separate trees involved in the database.
/// Used for quick lookup for devices, groups and images
pub(crate) fn init_trees(db: &sled::Db) -> Result<OTADatabase> {
    Ok(OTADatabase {
        images: db.open_tree("images")?,
        devices: db.open_tree("devices")?,
        groups: db.open_tree("groups")?,
    })
}

/// Function that is called outside of the thread so it can be tested separately.
pub(crate) async fn process_event(
    settings: &settings::Ota,
    broker_sender: &Sender<Event>,
    db: &OTADatabase,
    event: &Event,
) {
    match event {
        // Process OtaRequests
        Event::OtaRequest { device_id, msg } => {
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
                    log::debug!("Check!");

                    // Lookup
                    let package = get_ota_package_by_device_id(&db, &device_id).ok();

                    // Send it
                    broker_sender
                        .send_async(Event::OtaResponse(OtaUpdate {
                            uid: Some(device_id.clone()),
                            package: package,
                            images: None,
                        }))
                        .await
                        .unwrap();
                }
            }
        }

        Event::OtaDissociate {
            device_id,
            group_id,
        } => {
            // Match the different possiblities
            match (&device_id, &group_id) {
                (None, Some(g)) => {
                    if let Err(_) = dissociate_group(&db, &g).await {
                        log::warn!("Unable to disassociate group: {}", g);
                    }
                }
                (Some(d), None) => {
                    if let Err(_) = dissociate_device(&db, &d).await {
                        log::warn!("Unable to disassociate device: {}", d);
                    }
                }
                (Some(d), Some(g)) => {
                    if let Err(_) = dissociate_group(&db, &g).await {
                        log::warn!("Unable to disassociate group: {}", g);
                    }

                    if let Err(_) = dissociate_device(&db, &d).await {
                        log::warn!("Unable to disassociate device: {}", d);
                    }
                }
                _ => {}
            };
        }
        Event::OtaAssociate {
            device_id,
            group_id,
            image_id,
        } => {
            // Match the different possiblities
            match (&device_id, &group_id, &image_id) {
                (None, Some(group), Some(update)) => {
                    // Connect group -> image
                    if let Err(err) = associate_group_with_update(&db, &group, &update).await {
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
                    if let Err(err) = associate_device_with_group(&db, &device, &group).await {
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
                    if let Err(err) = associate_device_with_group(&db, &device, &group).await {
                        log::error!(
                            "Unable to associate {} with {}. Err: {}",
                            device,
                            group,
                            err
                        );
                        return;
                    }

                    // connect group -> image
                    if let Err(err) = associate_group_with_update(&db, &group, &update).await {
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
            // TODO: done in a separate function call?
            if let Some(device) = device_id {
                // Gather update information and then send it off to the device
                let package = match get_ota_package_by_device_id(&db, &device) {
                    Ok(p) => p,
                    Err(e) => {
                        log::warn!("Unable to get OTA package: Error: {}", e);
                        return;
                    }
                };

                let update = OtaUpdate {
                    uid: Some(device.to_string()),
                    package: Some(package),
                    images: None,
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

            // Get the package
            let mut package = match &update.package {
                Some(p) => p.clone(),
                None => {
                    log::error!("Package must exist!");
                    return;
                }
            };

            // Save image to file before we muck with the OtaUpdate
            let images = match &update.images {
                Some(i) => i,
                None => {
                    // There should be image data..
                    log::error!("Image(s) not valid!");
                    return;
                }
            };

            // Update ID
            let update_id = package.to_string();

            // Vector of file information
            let mut files: Vec<OTAPackageFileInfo> = Vec::new();

            // Save each of the images with the type attached to it as well.
            for image in images {
                if let Err(e) =
                    save_ota_firmware_image(&settings.image_path, &update_id, &image).await
                {
                    log::error!("Unable to save OTA firmware image. Err: {}", e);
                    return;
                }

                // Add image data to the Ota Package
                files.push(OTAPackageFileInfo {
                    image_type: image.image_type,
                    host: settings.url.clone(),
                    file: format!(
                        "{}{}/{}.bin",
                        &settings.image_path, &update_id, &image.image_type
                    ),
                });
            }

            // Set the files
            package.files = files;

            // Copy only useful stuff in update (no image binary data.)
            let update = OtaUpdate {
                uid: None,
                package: Some(package.clone()),
                images: None,
            };

            // Save the OTA package to database
            if let Err(e) = save_ota_package(&db, &update).await {
                log::error!("Unable to save OTA package. Error: {}", e);
                return;
            }
        }
        Event::OtaDeletePackage(_update) => {
            // TODO: implement this
        }
        Event::OtaUpdateImageListRequest() => {
            let mut response = OtaImageListResponse { images: Vec::new() };

            for image in db.images.into_iter() {
                match image {
                    Ok((k, v)) => {
                        // Deserialize
                        let key = match String::from_utf8(k.to_vec()) {
                            Ok(k) => k,
                            Err(_) => continue,
                        };

                        // Deserialize
                        let value: OTAPackage = match serde_cbor::from_slice(&v) {
                            Ok(v) => v,
                            Err(_) => continue,
                        };

                        // Add the image
                        response.images.push((key, value));
                    }
                    Err(_) => (),
                }
            }

            // Notify mqtt to send update!
            broker_sender
                .send_async(Event::OtaUpdateImageListRequestResponse(response))
                .await
                .unwrap();
        }
        Event::OtaUpdateGroupListRequest() => {
            let mut response = OtaGroupListResponse { groups: Vec::new() };

            for image in db.groups.into_iter() {
                match image {
                    Ok((k, _v)) => {
                        // Deserialize
                        let key = match String::from_utf8(k.to_vec()) {
                            Ok(k) => k,
                            Err(_) => continue,
                        };

                        // Add the image
                        response.groups.push(key);
                    }
                    Err(_) => (),
                }
            }

            // Notify mqtt to send update!
            broker_sender
                .send_async(Event::OtaUpdateGroupListRequestResponse(response))
                .await
                .unwrap();
        }
        _ => {}
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
        process_event(settings, &broker_sender, &db, &event).await;
    }
}

async fn dissociate_device(db: &OTADatabase, device_id: &String) -> Result<()> {
    // Delete entry from dB
    db.devices.remove(&device_id)?;
    db.devices.flush_async().await?;

    Ok(())
}

async fn dissociate_group(db: &OTADatabase, group_id: &String) -> Result<()> {
    // Delete entry from dB
    db.groups.remove(&group_id)?;
    db.groups.flush_async().await?;

    Ok(())
}

/// Associate device_id with group_id
async fn associate_device_with_group(
    db: &OTADatabase,
    device_id: &String,
    group_id: &String,
) -> Result<()> {
    // Insert the encoded data back into the db
    db.devices.insert(&device_id, group_id.as_bytes())?;
    db.devices.flush_async().await?;

    Ok(())
}

/// Associate group_id with update_id
async fn associate_group_with_update(
    db: &OTADatabase,
    group_id: &String,
    update_id: &String,
) -> Result<()> {
    // Check if insert worked ok
    db.groups.insert(&group_id, update_id.as_bytes())?;
    db.groups.flush_async().await?;

    Ok(())
}

fn get_update_file_path(image_type: &OTAImageType, update_name: &String) -> String {
    format!("{}/{}.bin", update_name, image_type)
}

/// Take binary data and save it to the image directory..
pub(crate) async fn save_ota_firmware_image(
    folder_path: &String,
    name: &String,
    image: &OTAImageData,
) -> Result<()> {
    let base_path = format!("{}/{}", folder_path, name);

    log::debug!("Base path: {}", base_path);

    // Make directory if it doesn't exist...
    if let Err(e) = fs::create_dir_all(&base_path) {
        log::warn!("Unable to create image directory: {}", e);
    }

    let full_path = format!(
        "{}/{}",
        folder_path,
        get_update_file_path(&image.image_type, name)
    );

    // Create the file
    // Ideal path is something like "images//"
    let mut file = File::create(full_path)?;

    log::debug!("File path: {:?}", file);

    // Write and sync
    file.write_all(&image.data)?;
    file.sync_all()?;

    Ok(())
}

pub(crate) async fn delete_ota_firmware_image(path: &str, name: &str) -> Result<()> {
    // Delete the folder from the filesystem
    fs::remove_dir_all(format!("{}/{}/", path, &name))?;

    Ok(())
}

/// Creates the OTA package in the database and filesystem.
///
/// This function overwrites any updates that may exist
pub(crate) async fn save_ota_package(db: &OTADatabase, update: &OtaUpdate) -> Result<()> {
    // Get the package
    let package = match &update.package {
        Some(p) => p,
        None => return Err(anyhow!("Package must exist!")),
    };

    // Generate the update ID
    let update_id = package.to_string();

    // Turn entry.package into CBOR
    let cbor_data = serde_cbor::ser::to_vec_packed(&package)?;

    // Check if insert worked ok
    db.images.insert(&update_id, cbor_data)?;
    db.images.flush_async().await?;

    Ok(())
}

/// Deletes the OTA package from the database and filesystem.
pub(crate) async fn delete_ota_package(db: &OTADatabase, update_id: &String) -> Result<()> {
    // Delete entry from dB
    db.images.remove(&update_id)?;
    db.images.flush_async().await?;

    Ok(())
}

/// Small server with one endpoint for handling OTA updates.
/// i.e. hosts static firmware images that can be pulled by the
/// firmware.
pub async fn ota_http_run(settings: &settings::Ota) {
    // TODO: for async-std use `tide`
    // TODO: API key for more secure transfers

    // Only one folder that we're interested in..
    let images = warp::path("images").and(warp::fs::dir(settings.image_path.clone()));

    // Run the `warp` server
    warp::serve(images)
        .run(SocketAddrV4::new(
            Ipv4Addr::new(127, 0, 0, 1),
            settings.http_port,
        ))
        .await;
}
