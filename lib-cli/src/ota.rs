use crate::OTAManifest;

// Cbor
use serde_cbor;

// Std lib
use std::fs::File;
use std::io::{prelude::*, BufReader};

// Websocket
use tungstenite::{client::AutoStream, protocol::WebSocket, Message};

/// Adds and OTA image from an included manifest file to the server
pub fn add_ota_from_manifest(
    stream: &mut WebSocket<AutoStream>,
    req: &crate::OtaAdd,
) -> anyhow::Result<()> {
    // Open the file in read-only mode with buffer.
    let file = File::open(&req.manifest)?;
    let reader = BufReader::new(file);

    // Read manifest file
    let manifest: OTAManifest = serde_json::from_reader(reader)?;

    println!(
        "Adding new update for: {} on device: {}",
        manifest.file, req.uid
    );

    // Read image in as data
    let mut buf: Vec<u8> = Vec::new();
    let mut file = File::open(&manifest.file)?;
    let size = file.read_to_end(&mut buf)?;

    println!("Reading {} bytes from firmware update binary.", size);

    // Data structure (from pyrinas_lib_shared)
    let new = pyrinas_shared::OtaUpdate {
        uid: req.uid.clone(),
        package: Some(pyrinas_shared::OTAPackage {
            version: manifest.version,
            host: "".to_string(),
            file: "".to_string(),
            force: req.force,
        }),
        image: Some(buf),
    };

    // Serialize to cbor
    let data = serde_cbor::to_vec(&new)?;

    // Then configure the outer data
    let msg = pyrinas_shared::ManagementData {
        target: pyrinas_shared::ManagmentDataType::AddOta,
        msg: data,
    };

    // If second encode looks good send it off
    let data = serde_cbor::to_vec(&msg)?;

    // Send over socket
    stream.write_message(Message::binary(data))?;

    Ok(())
}

// TODO: remove command..
