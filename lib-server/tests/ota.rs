use chrono::Utc;
// async Related
use flume::unbounded;

use pyrinas_shared::ota::v2::{OTAImageData, OTAImageType, OTAPackage, OTAUpdate};
use pyrinas_shared::ota::OTAPackageVersion;
use pyrinas_shared::{OtaRequest, OtaRequestCmd};

use pyrinas_server::ota;
use pyrinas_server::Event;

use std::sync::Once;

static INIT: Once = Once::new();

/// Setup function that is only run once, even if called multiple times.
fn setup() {
    INIT.call_once(|| env_logger::init());
}

fn get_update(major: u8, minor: u8, patch: u8) -> OTAUpdate {
    let hash: [u8; 8] = [103, 57, 54, 53, 98, 57, 100, 102];
    let image: [u8; 4] = [0, 0, 0, 0];
    let version = OTAPackageVersion {
        major: major,
        minor: minor,
        patch: patch,
        commit: 0,
        hash: hash.into(),
    };

    let package = OTAPackage {
        id: version.to_string(),
        version,
        file: Some(OTAImageData {
            data: image.to_vec(),
            image_type: OTAImageType::Primary,
        }),
        size: image.len(),
        date_added: Utc::now().to_string(),
    };

    // Update
    OTAUpdate {
        device_uid: None,
        package: Some(package),
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
    let update = get_update(1, 0, 1);

    // Test the save_ota_update
    if let Err(e) = ota::save_ota_update(&db, &update).await {
        log::error!("Error: {}", e);
        assert!(false);
    }

    // Get the update id
    let update_id = update.package.unwrap().to_string();

    // Check the database to make sure there's an entry
    assert!(db.images.contains_key(&update_id).unwrap());
}

#[tokio::test]
async fn get_ota_package_success() {
    // Log setup
    setup();

    // Creates temporary in-memory database
    let db: sled::Db = sled::Config::new().temporary(true).open().unwrap();
    let db = ota::init_trees(&db).unwrap();

    // Generate Update
    let update = get_update(1, 0, 2);

    // Get update id
    let update_id = update.package.clone().unwrap().to_string();

    // Test the save_ota_update
    if let Err(e) = ota::save_ota_update(&db, &update).await {
        log::error!("Error: {}", e);
        assert!(false);
    }

    // Get OTA package
    let fetched_update = match ota::get_ota_update(&db, &update_id) {
        Ok(p) => p,
        Err(e) => {
            log::error!("Error getting OTA package. Error: {}", e);
            assert!(false);
            return;
        }
    };

    // Make sure everything is equal
    assert_eq!(
        update.package.unwrap().version,
        fetched_update.package.unwrap().version
    );
}

#[tokio::test]
async fn delete_ota_package_success() {
    // Log setup
    setup();

    // Creates temporary in-memory database
    let db: sled::Db = sled::Config::new().temporary(true).open().unwrap();
    let db = ota::init_trees(&db).unwrap();

    // Generate Update
    let update = get_update(1, 0, 3);

    // Test the save_ota_update
    ota::save_ota_update(&db, &update).await.unwrap();

    // Generate the update ID
    let update_id = update.package.unwrap().to_string();

    // Delete the package
    ota::delete_ota_package(&db, &update_id).await.unwrap();
}

#[tokio::test]
async fn delete_all_ota_packages_success() {
    // Log setup
    setup();

    // Creates temporary in-memory database
    let db: sled::Db = sled::Config::new().temporary(true).open().unwrap();
    let db = ota::init_trees(&db).unwrap();

    // Generate Update
    let update = get_update(1, 0, 3);

    // Test the save_ota_update
    ota::save_ota_update(&db, &update).await.unwrap();

    // Delete the package
    ota::delete_all_ota_data(&db).await.unwrap();

    // Image shouldn't be there now..
    let update_id = update.package.clone().unwrap().to_string();
    assert!(ota::get_ota_update(&db, &update_id).is_err());

    // save_ota_update again
    ota::save_ota_update(&db, &update).await.unwrap();
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
}

#[tokio::test]
/// Checks to make sure there's a failure when trying to delete a non-existent file
async fn test_ota_new_package_event() {
    // Log setup
    setup();

    // Creates temporary in-memory database
    let db: sled::Db = sled::Config::new().temporary(true).open().unwrap();
    let db = ota::init_trees(&db).unwrap();

    // Generate Update
    let update = get_update(1, 1, 0);

    // Get update id
    let update_id = update.package.clone().unwrap().to_string();

    // Get the sender/reciever associated with this particular task
    let (sender, _) = unbounded::<Event>();

    // New OTA package event
    let event = Event::OtaNewPackage(update);

    // Process
    ota::process_event(&sender, &db, &event).await;

    // check if it's registered
    assert!(ota::get_ota_update(&db, &update_id).is_ok());
}

#[tokio::test]
/// Checks to make sure there's a failure when trying to delete a non-existent file
async fn test_ota_request_check_event_not_found() {
    // Log setup
    setup();

    // Creates temporary in-memory database
    let db: sled::Db = sled::Config::new().temporary(true).open().unwrap();
    let db = ota::init_trees(&db).unwrap();

    // Get the sender/reciever associated with this particular task
    let (sender, receiver) = unbounded::<Event>();

    // New OTA package event
    let event = Event::OtaRequest {
        device_uid: "1234".to_string(),
        msg: OtaRequest {
            cmd: OtaRequestCmd::Check,
            ..Default::default()
        },
    };

    // Process
    ota::process_event(&sender, &db, &event).await;

    // Get the event sent.
    let event = receiver.recv().unwrap();

    // Package should not be found
    match event {
        Event::OtaResponse(update) => {
            assert_eq!(update.device_uid, Some("1234".to_string()));
            assert!(update.package.is_none());
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

    // Generate Update
    let update = get_update(1, 1, 2);

    // Get update id
    let update_id = update.package.clone().unwrap().to_string();

    // Get the sender/reciever associated with this particular task
    let (sender, _) = unbounded::<Event>();

    // Save update and then try to get it
    let event = Event::OtaNewPackage(update.clone());

    // Process
    ota::process_event(&sender, &db, &event).await;

    // Check to make sure the OTA package is there and is what's expected
    let fetched_update = ota::get_ota_update(&db, &update_id);
    assert!(fetched_update.is_ok());

    // Get the package
    let fetched_update = fetched_update.unwrap();
    let package = fetched_update.package.unwrap();

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

    // Generate Update
    let update = get_update(1, 1, 2);

    // Get update id
    let update_id = update.package.clone().unwrap().to_string();

    // Get the sender/reciever associated with this particular task
    let (sender, _) = unbounded::<Event>();

    // Save update and then try to get it
    let event = Event::OtaNewPackage(update.clone());

    // Process
    ota::process_event(&sender, &db, &event).await;

    // Check to make sure the OTA package is there and is what's expected
    let fetched_update = ota::get_ota_update(&db, &update_id);
    assert!(fetched_update.is_ok());

    // Get the fetched_update
    let fetched_update = fetched_update.unwrap();
    let package = fetched_update.package.unwrap();

    assert_eq!(package.version, update.package.unwrap().version);
}

#[tokio::test]
/// Checks to make sure there's a failure when trying to delete a non-existent file
async fn test_ota_request_assign_and_check() {
    // Log setup
    setup();

    // Creates temporary in-memory database
    let db: sled::Db = sled::Config::new().temporary(true).open().unwrap();
    let db = ota::init_trees(&db).unwrap();

    // Generate Update
    let initial_update = get_update(1, 1, 2);

    // Get update id
    let update_id = initial_update.package.clone().unwrap().to_string();

    // Get the sender/reciever associated with this particular task
    let (sender, receiver) = unbounded::<Event>();

    // Save update and then try to get it
    let event = Event::OtaNewPackage(initial_update.clone());

    // Process
    ota::process_event(&sender, &db, &event).await;

    // Then assign the new image to a device
    let event = Event::OtaLink {
        device_id: Some("1234".to_string()),
        group_id: Some("1".to_string()),
        image_id: Some(update_id.clone()),
    };

    ota::process_event(&sender, &db, &event).await;

    // Get the event sent.
    let event = receiver.recv().unwrap();

    // Package should not be found
    match event {
        Event::OtaResponse(update) => {
            // Make sure this is ok
            assert_eq!(update.device_uid, Some("1234".to_string()));
            assert!(update.package.is_some());

            // Confirm contents of package.
            let package = update.package.unwrap();

            assert_eq!(package.version, initial_update.package.unwrap().version);

            log::debug!("{} - {}", update_id, OTAImageType::Primary);
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

    // Get the sender/reciever associated with this particular task
    let (sender, receiver) = unbounded::<Event>();

    // Save update and then try to get it
    let event = Event::OtaUpdateGroupListRequest();

    // Process
    ota::process_event(&sender, &db, &event).await;

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
    ota::process_event(&sender, &db, &event).await;

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

    // Generate Update
    let initial_update = get_update(1, 1, 3);

    // Get update id
    let update_id = initial_update.package.clone().unwrap().to_string();

    // Get the sender/reciever associated with this particular task
    let (sender, receiver) = unbounded::<Event>();

    // Save update and then try to get it
    let event = Event::OtaNewPackage(initial_update.clone());

    // Process
    ota::process_event(&sender, &db, &event).await;

    // Then assign the new image to a device
    let event = Event::OtaLink {
        device_id: Some("1234".to_string()),
        group_id: Some("1".to_string()),
        image_id: Some(update_id.clone()),
    };

    ota::process_event(&sender, &db, &event).await;

    // Get the event sent.
    receiver.recv().unwrap();

    // Save update and then try to get it
    let event = Event::OtaUpdateGroupListRequest();

    // Process
    ota::process_event(&sender, &db, &event).await;

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
    ota::process_event(&sender, &db, &event).await;

    // Get the event sent.
    let event = receiver.recv().unwrap();

    // Len should be 0
    match event {
        Event::OtaUpdateImageListRequestResponse(r) => {
            assert_eq!(r.images.len(), 1);
            assert_eq!(r.images[0].name, update_id);
        }
        _ => {
            assert!(false, "Unexpected event!")
        }
    };
}

#[tokio::test]
/// Checks to make sure there's a failure when trying to delete a non-existent file
async fn test_ota_request_associate_and_dissociate() {
    // Log setup
    setup();

    // Creates temporary in-memory database
    let db: sled::Db = sled::Config::new().temporary(true).open().unwrap();
    let db = ota::init_trees(&db).unwrap();

    // Generate Update
    let initial_update = get_update(1, 1, 3);

    // Get update id
    let update_id = initial_update.package.clone().unwrap().to_string();

    // Get the sender/reciever associated with this particular task
    let (sender, receiver) = unbounded::<Event>();

    // Save update and then try to get it
    let event = Event::OtaNewPackage(initial_update.clone());

    // Process
    ota::process_event(&sender, &db, &event).await;

    // Then assign the new image to a device
    let event = Event::OtaLink {
        device_id: Some("1234".to_string()),
        group_id: Some("1".to_string()),
        image_id: Some(update_id.clone()),
    };

    ota::process_event(&sender, &db, &event).await;

    // Get the event sent.
    receiver.recv().unwrap();

    // Then assign the new image to a device
    let event = Event::OtaUnlink {
        device_id: Some("1234".to_string()),
        group_id: None,
    };

    ota::process_event(&sender, &db, &event).await;

    assert!(db.devices.get(&"1234".to_string()).unwrap().is_none());
    assert!(db.groups.get(&"1".to_string()).unwrap().is_some());
}
