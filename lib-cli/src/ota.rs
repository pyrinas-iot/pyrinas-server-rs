// Anyhow
use anyhow::anyhow;

// Cbor
use serde_cbor;

// Std lib
use std::fs::File;
use std::io::prelude::*;

// Websocket
use tungstenite::{client::AutoStream, protocol::WebSocket, Message};

/// Adds and OTA image from an included manifest file to the server
pub fn add_ota(stream: &mut WebSocket<AutoStream>, req: &crate::OtaAdd) -> anyhow::Result<()> {
    // Get the current version using 'git describe'
    let ver = crate::get_git_describe()?;

    // Then parse it to get OTAPackageVersion
    let (ver, dirty) = crate::get_ota_package_version(&ver)?;

    // Force error
    if dirty && !req.force {
        return Err(anyhow!("Repository is dirty. Run --force to override."));
    }

    println!("Adding new update for {}", req.uid);

    // Path for ota
    let path = "./build/zephyr/app_update.bin";

    // Read image in as data
    let mut buf: Vec<u8> = Vec::new();
    let mut file = match File::open(&path) {
        Ok(f) => f,
        Err(_) => return Err(anyhow!("Unable to find firmware file in {}", path)),
    };
    let size = file.read_to_end(&mut buf)?;

    println!("Reading {} bytes from firmware update binary.", size);

    // Data structure (from pyrinas_lib_shared)
    let new = pyrinas_shared::OtaUpdate {
        uid: req.uid.clone(),
        package: Some(pyrinas_shared::OTAPackage {
            version: ver,
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
        cmd: pyrinas_shared::ManagmentDataType::AddOta,
        target: None,
        msg: data,
    };

    // If second encode looks good send it off
    let data = serde_cbor::to_vec(&msg)?;

    // Send over socket
    stream.write_message(Message::binary(data))?;

    Ok(())
}

// TODO: remove command..
