// Cbor
use serde_cbor;

// Std lib
use std::fs::File;
use std::io::{self, prelude::*};

// Websocket
use tungstenite::{client::AutoStream, protocol::WebSocket, Message};

// Error handling
use thiserror::Error;

#[derive(Debug, Error)]
pub enum OtaError {
    #[error("file error: {source}")]
    FileError {
        #[from]
        source: io::Error,
    },

    /// Serde CBOR error
    #[error("serde_cbor error: {source}")]
    CborError {
        #[from]
        source: serde_cbor::Error,
    },

    /// Websocket error
    #[error("websocket error: {source}")]
    WebsocketError {
        #[from]
        source: tungstenite::Error,
    },

    /// Error to indicate repo is dirty
    #[error("repository is dirty. Run --force to override")]
    DirtyError,

    /// Error from CLI portion of code
    #[error("cli error: {source}")]
    CliError {
        #[from]
        source: crate::CliError,
    },
}

/// Adds and OTA image from an included manifest file to the server
pub fn add_ota(stream: &mut WebSocket<AutoStream>, req: &crate::OtaAdd) -> Result<(), OtaError> {
    // Get the current version using 'git describe'
    let ver = crate::get_git_describe()?;

    // Then parse it to get OTAPackageVersion
    let (ver, dirty) = crate::get_ota_package_version(&ver)?;

    // Force error
    if dirty && !req.force {
        return Err(OtaError::DirtyError);
    }

    println!("Adding new update for {}", req.uid);

    // Path for ota
    let path = "./build/zephyr/app_update.bin";

    // Read image in as data
    let mut buf: Vec<u8> = Vec::new();
    let mut file = File::open(&path)?;
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
