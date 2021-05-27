// async Related
use flume::unbounded;

use pyrinas_shared::{
    OTAImageData, OTAImageType, OTAPackage, OTAPackageVersion, OtaRequest, OtaRequestCmd, OtaUpdate,
};

use crate::Event;
use crate::{ota, settings};

use std::path::Path;
use std::sync::Once;

static INIT: Once = Once::new();

/// Setup function that is only run once, even if called multiple times.
fn setup() {
    INIT.call_once(|| env_logger::init());
}

fn get_update(major: u8, minor: u8, patch: u8, has_secondary: bool) -> OtaUpdate {
    let hash: [u8; 8] = [103, 57, 54, 53, 98, 57, 100, 102];
    let image: [u8; 4] = [0, 0, 0, 0];

    let package = OTAPackage {
        version: OTAPackageVersion {
            major: major,
            minor: minor,
            patch: patch,
            commit: 0,
            hash: hash.into(),
        },
        files: Vec::new(),
    };

    let mut images: Vec<OTAImageData> = Vec::new();

    images.push(OTAImageData {
        data: image.to_vec(),
        image_type: OTAImageType::Primary,
    });

    if has_secondary {
        images.push(OTAImageData {
            data: image.to_vec(),
            image_type: OTAImageType::Secondary,
        });
    }

    // Update
    OtaUpdate {
        uid: None,
        package: Some(package),
        images: Some(images),
    }
}

fn get_default_settings() -> settings::Ota {
    // Create settings
    settings::Ota {
        url: "localhost".to_string(),
        db_path: ".".to_string(),
        http_port: 8080,
        image_path: "_images/".to_string(),
    }
}

#[tokio::test]
async fn save_ota_package_sucess() {
    // Log setup
    setup();

    // Creates temporary in-memory database
    let db: sled::Db = sled::Config::new().temporary(true).open().unwrap();
    let db = ota::init_trees(&db).unwrap();

    // Generate Update
    let update = get_update(1, 0, 1, false);

    // Test the save_ota_package
    if let Err(e) = ota::save_ota_package(&db, &update).await {
        log::error!("Error: {}", e);
        assert!(false);
    }

    // Get the update id
    let update_id = update.package.unwrap().to_string();

    // Check the database to make sure there's an entry
    assert!(db.images.contains_key(&update_id).unwrap());

    // Get the first element
    let image = &update.images.unwrap()[0];

    // Save the image to disk
    ota::save_ota_firmware_image(&"./images".to_string(), &update_id, image)
        .await
        .unwrap();

    // Get the file path
    let file_path = format!(
        "./images/{}/{}-{}.bin",
        update_id, image.image_type, update_id
    );

    log::info!("filepath {}", file_path);

    // Check if the image is in place
    assert!(Path::new(&file_path).exists());

    // check to make sure the file exists int he correct folder.
    assert!(std::path::Path::new(&file_path).exists());
}

#[tokio::test]
async fn get_ota_package_success() {
    // Log setup
    setup();

    // Creates temporary in-memory database
    let db: sled::Db = sled::Config::new().temporary(true).open().unwrap();
    let db = ota::init_trees(&db).unwrap();

    // Generate Update
    let update = get_update(1, 0, 2, false);

    // Get update id
    let update_id = update.package.clone().unwrap().to_string();

    // Test the save_ota_package
    if let Err(e) = ota::save_ota_package(&db, &update).await {
        log::error!("Error: {}", e);
        assert!(false);
    }

    // Get OTA package
    let package = match ota::get_ota_package(&db, &update_id) {
        Ok(p) => p,
        Err(e) => {
            log::error!("Error getting OTA package. Error: {}", e);
            assert!(false);
            return;
        }
    };

    // Make sure everything is equal
    assert_eq!(update.package.unwrap().version, package.version);
}

#[tokio::test]
async fn delete_ota_package_success() {
    // Log setup
    setup();

    // Creates temporary in-memory database
    let db: sled::Db = sled::Config::new().temporary(true).open().unwrap();
    let db = ota::init_trees(&db).unwrap();

    // Generate Update
    let update = get_update(1, 0, 3, false);

    // Test the save_ota_package
    ota::save_ota_package(&db, &update).await.unwrap();

    // Generate the update ID
    let update_id = update.package.unwrap().to_string();

    // Save all the appropriate images
    for image in update.images.unwrap() {
        // Save the image to disk
        if let Err(e) =
            ota::save_ota_firmware_image(&"./images/".to_string(), &update_id, &image).await
        {
            log::error!("Error: {}", e);
            assert!(false);
        }

        let file_path = format!("./images/{}/{}.bin", update_id, image.image_type);

        // Check if the image is in place
        assert!(Path::new(&file_path).exists());
    }

    // Delete the package
    ota::delete_ota_package(&db, &update_id).await.unwrap();

    // Delete the image folder
    ota::delete_ota_firmware_image("./images/", &update_id)
        .await
        .unwrap();

    // Check if the folder is gone
    assert!(!Path::new(&format!("./images/{}/", &update_id)).exists());
}

#[tokio::test]
/// Checks to make sure there's a failure when trying to delete a non-existent file
async fn delete_ota_package_failure() {
    // Log setup
    setup();

    // Creates temporary in-memory database
    let db: sled::Db = sled::Config::new().temporary(true).open().unwrap();
    let db = ota::init_trees(&db).unwrap();

    // Get a bogus update_id
    let update_id = "bogus_id".to_string();

    // Delete the package
    let res = ota::delete_ota_package(&db, &update_id).await;
    assert!(res.is_ok());

    let res = ota::delete_ota_firmware_image("./images/", &update_id).await;
    assert!(res.is_err());
}

#[tokio::test]
/// Checks to make sure there's a failure when trying to delete a non-existent file
async fn test_ota_new_package_event() {
    // Log setup
    setup();

    // Creates temporary in-memory database
    let db: sled::Db = sled::Config::new().temporary(true).open().unwrap();
    let db = ota::init_trees(&db).unwrap();

    let settings = get_default_settings();

    // Generate Update
    let update = get_update(1, 1, 0, false);

    // Get update id
    let update_id = update.package.clone().unwrap().to_string();

    // Get the sender/reciever associated with this particular task
    let (sender, _) = unbounded::<Event>();

    // New OTA package event
    let event = Event::OtaNewPackage(update);

    // Process
    ota::process_event(&settings, &sender, &db, &event).await;

    // check if it's registered
    assert!(ota::get_ota_package(&db, &update_id).is_ok());

    // Get filepath
    let file_path = format!(
        "{}/{}/{}-{}.bin",
        settings.image_path,
        update_id,
        OTAImageType::Primary,
        update_id
    );

    // Check if it's saved to the filesystem
    assert!(Path::new(&file_path).exists());
}

#[tokio::test]
/// Checks to make sure there's a failure when trying to delete a non-existent file
async fn test_ota_request_check_event_not_found() {
    // Log setup
    setup();

    // Creates temporary in-memory database
    let db: sled::Db = sled::Config::new().temporary(true).open().unwrap();
    let db = ota::init_trees(&db).unwrap();

    let settings = get_default_settings();

    // Get the sender/reciever associated with this particular task
    let (sender, receiver) = unbounded::<Event>();

    // New OTA package event
    let event = Event::OtaRequest {
        device_id: "1234".to_string(),
        msg: OtaRequest {
            cmd: OtaRequestCmd::Check,
        },
    };

    // Process
    ota::process_event(&settings, &sender, &db, &event).await;

    // Get the event sent.
    let event = receiver.recv().unwrap();

    // Package should not be found
    match event {
        Event::OtaResponse(update) => {
            // Make sure this is ok
            assert!(update.uid == Some("1234".to_string()));
            assert!(update.package.is_none());
            assert!(update.images.is_none());
        }
        _ => {
            assert!(false, "Unexpected event!")
        }
    }
}

#[tokio::test]
/// Checks to make sure there's a failure when trying to delete a non-existent file
async fn test_ota_request_check_event_found() {
    // Log setup
    setup();

    // Creates temporary in-memory database
    let db: sled::Db = sled::Config::new().temporary(true).open().unwrap();
    let db = ota::init_trees(&db).unwrap();

    let settings = get_default_settings();

    // Generate Update
    let update = get_update(1, 1, 2, false);

    // Get update id
    let update_id = update.package.clone().unwrap().to_string();

    // Get the sender/reciever associated with this particular task
    let (sender, _) = unbounded::<Event>();

    // Save update and then try to get it
    let event = Event::OtaNewPackage(update.clone());

    // Process
    ota::process_event(&settings, &sender, &db, &event).await;

    // Get filepath
    let file_path = format!(
        "{}/{}/{}-{}.bin",
        settings.image_path,
        update_id,
        OTAImageType::Primary,
        update_id
    );

    // Check if the file exists
    assert!(Path::new(&file_path).exists());

    // Check to make sure the OTA package is there and is what's expected
    let package = ota::get_ota_package(&db, &update_id);
    assert!(package.is_ok());

    // Get the package
    let package = package.unwrap();

    assert_eq!(package.files.len(), 1);

    assert_eq!(package.files[0].host, settings.url);

    log::debug!(
        "{} {}",
        package.files[0].file,
        format!(
            "{}{}/{}-{}.bin",
            settings.image_path,
            update_id,
            OTAImageType::Primary,
            update_id
        )
    );

    assert_eq!(package.version, update.package.unwrap().version);
}

#[tokio::test]
/// Checks to make sure there's a failure when trying to delete a non-existent file
async fn test_ota_request_check_seconary_found() {
    // Log setup
    setup();

    // Creates temporary in-memory database
    let db: sled::Db = sled::Config::new().temporary(true).open().unwrap();
    let db = ota::init_trees(&db).unwrap();

    let settings = get_default_settings();

    // Generate Update
    let update = get_update(1, 1, 2, true);

    // Get update id
    let update_id = update.package.clone().unwrap().to_string();

    // Get the sender/reciever associated with this particular task
    let (sender, _) = unbounded::<Event>();

    // Save update and then try to get it
    let event = Event::OtaNewPackage(update.clone());

    // Process
    ota::process_event(&settings, &sender, &db, &event).await;

    // Check if the primary exists
    assert!(Path::new(&format!(
        "{}/{}/{}-{}.bin",
        settings.image_path,
        update_id,
        OTAImageType::Primary,
        update_id
    ))
    .exists());

    // Check if the secondary exists
    assert!(Path::new(&format!(
        "{}/{}/{}-{}.bin",
        settings.image_path,
        update_id,
        OTAImageType::Secondary,
        update_id
    ))
    .exists());

    // Check to make sure the OTA package is there and is what's expected
    let package = ota::get_ota_package(&db, &update_id);
    assert!(package.is_ok());

    // Get the package
    let package = package.unwrap();

    assert_eq!(package.files.len(), 2);

    assert_eq!(package.files[0].host, settings.url);

    log::debug!(
        "{} {}",
        package.files[0].file,
        format!(
            "{}{}/{}-{}.bin",
            settings.image_path,
            update_id,
            OTAImageType::Primary,
            update_id
        )
    );

    assert_eq!(package.files[1].host, settings.url);

    log::debug!(
        "{} {}",
        package.files[1].file,
        format!(
            "{}{}/{}-{}.bin",
            settings.image_path,
            update_id,
            OTAImageType::Secondary,
            update_id
        )
    );

    assert_eq!(package.version, update.package.unwrap().version);
}

#[tokio::test]
/// Checks to make sure there's a failure when trying to delete a non-existent file
async fn test_ota_request_assign_an_check() {
    // Log setup
    setup();

    // Creates temporary in-memory database
    let db: sled::Db = sled::Config::new().temporary(true).open().unwrap();
    let db = ota::init_trees(&db).unwrap();

    let settings = get_default_settings();

    // Generate Update
    let initial_update = get_update(1, 1, 2, false);

    // Get update id
    let update_id = initial_update.package.clone().unwrap().to_string();

    // Get the sender/reciever associated with this particular task
    let (sender, receiver) = unbounded::<Event>();

    // Save update and then try to get it
    let event = Event::OtaNewPackage(initial_update.clone());

    // Process
    ota::process_event(&settings, &sender, &db, &event).await;

    // Then assign the new image to a device
    let event = Event::OtaAssociate {
        device_id: Some("1234".to_string()),
        group_id: Some("1".to_string()),
        update_id: Some(update_id.clone()),
    };

    ota::process_event(&settings, &sender, &db, &event).await;

    // Get the event sent.
    let event = receiver.recv().unwrap();

    // Package should not be found
    match event {
        Event::OtaResponse(update) => {
            // Make sure this is ok
            assert!(update.uid == Some("1234".to_string()));
            assert!(update.package.is_some());
            assert!(update.images.is_none());

            // Confirm contents of package.
            let package = update.package.unwrap();

            assert_eq!(package.version, initial_update.package.unwrap().version);

            assert_eq!(package.files.len(), 1);

            assert_eq!(package.files[0].host, settings.url);

            log::debug!(
                "{} {}",
                package.files[0].file,
                format!(
                    "{}{}/{}-{}.bin",
                    settings.image_path,
                    update_id,
                    OTAImageType::Primary,
                    update_id
                )
            );
        }
        _ => {
            assert!(false, "Unexpected event!")
        }
    }
}

#[tokio::test]
/// Checks to make sure there's a failure when trying to delete a non-existent file
async fn test_ota_request_empty_group_and_device_lists() {
    // Log setup
    setup();

    // Creates temporary in-memory database
    let db: sled::Db = sled::Config::new().temporary(true).open().unwrap();
    let db = ota::init_trees(&db).unwrap();

    let settings = get_default_settings();

    // Get the sender/reciever associated with this particular task
    let (sender, receiver) = unbounded::<Event>();

    // Save update and then try to get it
    let event = Event::OtaUpdateGroupListRequest();

    // Process
    ota::process_event(&settings, &sender, &db, &event).await;

    // Get the event sent.
    let event = receiver.recv().unwrap();

    // Len should be 0
    match event {
        Event::OtaUpdateGroupListRequestResponse(r) => {
            assert_eq!(r.groups.len(), 0);
        }
        _ => {
            assert!(false, "Unexpected event!")
        }
    };

    // Save update and then try to get it
    let event = Event::OtaUpdateImageListRequest();

    // Process
    ota::process_event(&settings, &sender, &db, &event).await;

    // Get the event sent.
    let event = receiver.recv().unwrap();

    // Len should be 0
    match event {
        Event::OtaUpdateImageListRequestResponse(r) => {
            assert_eq!(r.images.len(), 0);
        }
        _ => {
            assert!(false, "Unexpected event!")
        }
    };
}

#[tokio::test]
/// Checks to make sure there's a failure when trying to delete a non-existent file
async fn test_ota_request_associate_device_image_group_and_get_group_list_and_image_list() {
    // Log setup
    setup();

    // Creates temporary in-memory database
    let db: sled::Db = sled::Config::new().temporary(true).open().unwrap();
    let db = ota::init_trees(&db).unwrap();

    let settings = get_default_settings();
    // Generate Update
    let initial_update = get_update(1, 1, 3, false);

    // Get update id
    let update_id = initial_update.package.clone().unwrap().to_string();

    // Get the sender/reciever associated with this particular task
    let (sender, receiver) = unbounded::<Event>();

    // Save update and then try to get it
    let event = Event::OtaNewPackage(initial_update.clone());

    // Process
    ota::process_event(&settings, &sender, &db, &event).await;

    // Then assign the new image to a device
    let event = Event::OtaAssociate {
        device_id: Some("1234".to_string()),
        group_id: Some("1".to_string()),
        update_id: Some(update_id.clone()),
    };

    ota::process_event(&settings, &sender, &db, &event).await;

    // Get the event sent.
    receiver.recv().unwrap();

    // Save update and then try to get it
    let event = Event::OtaUpdateGroupListRequest();

    // Process
    ota::process_event(&settings, &sender, &db, &event).await;

    // Get the event sent.
    let event = receiver.recv().unwrap();

    // Len should be 0
    match event {
        Event::OtaUpdateGroupListRequestResponse(r) => {
            assert_eq!(r.groups.len(), 1);
            assert_eq!(r.groups[0], "1".to_string());
        }
        _ => {
            assert!(false, "Unexpected event!")
        }
    };

    // Save update and then try to get it
    let event = Event::OtaUpdateImageListRequest();

    // Process
    ota::process_event(&settings, &sender, &db, &event).await;

    // Get the event sent.
    let event = receiver.recv().unwrap();

    // Len should be 0
    match event {
        Event::OtaUpdateImageListRequestResponse(r) => {
            assert_eq!(r.images.len(), 1);
            assert_eq!(r.images[0].0, update_id);
        }
        _ => {
            assert!(false, "Unexpected event!")
        }
    };
}

#[tokio::test]
/// Checks to make sure there's a failure when trying to delete a non-existent file
async fn test_ota_request_associate_and_deassociate() {
    // Log setup
    setup();

    // Creates temporary in-memory database
    let db: sled::Db = sled::Config::new().temporary(true).open().unwrap();
    let db = ota::init_trees(&db).unwrap();

    let settings = get_default_settings();
    // Generate Update
    let initial_update = get_update(1, 1, 3, false);

    // Get update id
    let update_id = initial_update.package.clone().unwrap().to_string();

    // Get the sender/reciever associated with this particular task
    let (sender, receiver) = unbounded::<Event>();

    // Save update and then try to get it
    let event = Event::OtaNewPackage(initial_update.clone());

    // Process
    ota::process_event(&settings, &sender, &db, &event).await;

    // Then assign the new image to a device
    let event = Event::OtaAssociate {
        device_id: Some("1234".to_string()),
        group_id: Some("1".to_string()),
        update_id: Some(update_id.clone()),
    };

    ota::process_event(&settings, &sender, &db, &event).await;

    // Get the event sent.
    receiver.recv().unwrap();

    // Then assign the new image to a device
    let event = Event::OtaDeassociate {
        device_id: Some("1234".to_string()),
        group_id: None,
    };

    ota::process_event(&settings, &sender, &db, &event).await;

    assert!(db.devices.get(&"1234".to_string()).unwrap().is_none());
    assert!(db.groups.get(&"1".to_string()).unwrap().is_some());
}
