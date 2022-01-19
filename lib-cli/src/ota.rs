use chrono::{Duration, Local, Utc};

// Pyrinas
use pyrinas_shared::ota::v2::{OTAImageData, OTAImageType, OTAPackage, OtaUpdate};
use pyrinas_shared::{
    ManagementData, ManagmentDataType, OtaGroupListResponse, OtaImageListResponse,
};

// Cbor
use serde_cbor;

// Std lib
use std::fs::File;
use std::io::{self, prelude::*};
use std::net::TcpStream;

// Websocket
use tungstenite::{protocol::WebSocket, stream::MaybeTlsStream, Message};

// Error handling
use thiserror::Error;

use crate::{git, OtaAssociate, OtaSubCommand};

#[derive(Debug, Error)]
pub enum Error {
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

    /// Error for git related commands
    #[error("{source}")]
    GitError {
        #[from]
        source: git::GitError,
    },
}

/// Functon for processing all incoming OTA commands.
pub fn process(
    socket: &mut WebSocket<MaybeTlsStream<TcpStream>>,
    cmd: &OtaSubCommand,
) -> Result<(), Error> {
    match cmd {
        OtaSubCommand::Add(a) => {
            let image_id = crate::ota::add_ota(socket, a.force)?;

            println!("{} image successfully uploaded!", &image_id);

            // Do association
            match &a.device_id {
                Some(device_id) => {
                    let a = OtaAssociate {
                        device_id: Some(device_id.clone()),
                        group_id: Some(device_id.to_string()),
                        image_id: Some(image_id),
                        ota_version: a.ota_version,
                    };

                    crate::ota::associate(socket, &a)?;

                    println!("Associated! {:?}", &a);
                }
                None => (),
            };
        }
        OtaSubCommand::Remove(r) => {
            crate::ota::remove_ota(socket, &r.image_id)?;

            println!("{} successfully removed!", &r.image_id);
        }
        OtaSubCommand::Associate(a) => {
            crate::ota::associate(socket, &a)?;

            println!("Associated! {:?}", &a);
        }
        OtaSubCommand::ListGroups => {
            crate::ota::get_ota_group_list(socket)?;

            let start = Utc::now();

            // Get message
            loop {
                if Utc::now() > start + Duration::seconds(10) {
                    eprintln!("No response from server!");
                    break;
                }

                match socket.read_message() {
                    Ok(msg) => {
                        let data = match msg {
                            tungstenite::Message::Binary(b) => b,
                            _ => {
                                eprintln!("Unexpected WS message!");
                                break;
                            }
                        };

                        let list: OtaGroupListResponse = match serde_cbor::from_slice(&data) {
                            Ok(m) => m,
                            Err(e) => {
                                eprintln!("Unable to get image list! Error: {}", e);
                                break;
                            }
                        };

                        for name in list.groups.iter() {
                            // Print out the entry
                            println!("{}", name);
                        }

                        break;
                    }
                    Err(_) => continue,
                };
            }
        }
        OtaSubCommand::ListImages => {
            crate::ota::get_ota_image_list(socket)?;

            let start = Utc::now();

            // Get message
            loop {
                if Utc::now() > start + Duration::seconds(10) {
                    eprintln!("No response from server!");
                    break;
                }

                match socket.read_message() {
                    Ok(msg) => {
                        let data = match msg {
                            tungstenite::Message::Binary(b) => b,
                            _ => {
                                eprintln!("Unexpected WS message!");
                                break;
                            }
                        };

                        let list: OtaImageListResponse = match serde_cbor::from_slice(&data) {
                            Ok(m) => m,
                            Err(e) => {
                                eprintln!("Unable to get image list! Error: {}", e);
                                break;
                            }
                        };

                        for (name, package) in list.images.iter() {
                            // Get the date
                            let date = match package.date_added {
                                Some(d) => d.with_timezone(&Local).to_string(),
                                None => "".to_string(),
                            };

                            // Print out the entry
                            println!("{} {}", name, date);
                        }

                        break;
                    }
                    Err(_) => continue,
                };
            }
        }
    };

    Ok(())
}

/// Adds and OTA image from an included manifest file to the server
pub fn add_ota(
    stream: &mut WebSocket<MaybeTlsStream<TcpStream>>,
    force: bool,
) -> Result<String, Error> {
    // Get the current version using 'git describe'
    let ver = crate::git::get_git_describe()?;

    // Then parse it to get OTAPackageVersion
    let (package_version, dirty) = crate::git::get_ota_package_version(&ver)?;

    // Force error
    if dirty && !force {
        return Err(Error::DirtyError);
    }

    // Path for ota
    let path = "./build/zephyr/app_update.bin";

    // Read image in as data
    let mut buf: Vec<u8> = Vec::new();
    let mut file = File::open(&path)?;
    let size = file.read_to_end(&mut buf)?;

    println!("Reading {} bytes from firmware update binary.", size);

    // Data structure (from pyrinas_lib_shared)
    let new = OtaUpdate {
        uid: None,
        package: Some(OTAPackage {
            version: package_version.clone(),
            files: Vec::new(),
            date_added: Some(Utc::now()),
        }),
        images: Some(
            [OTAImageData {
                data: buf,
                image_type: OTAImageType::Primary,
            }]
            .to_vec(),
        ),
    };

    // Serialize to cbor
    let data = serde_cbor::to_vec(&new)?;

    // Then configure the outer data
    let msg = ManagementData {
        cmd: ManagmentDataType::AddOta,
        target: None,
        msg: data,
    };

    // If second encode looks good send it off
    let data = serde_cbor::to_vec(&msg)?;

    // Send over socket
    stream.write_message(Message::binary(data))?;

    Ok(package_version.to_string())
}

pub fn associate(
    stream: &mut WebSocket<MaybeTlsStream<TcpStream>>,
    associate: &OtaAssociate,
) -> Result<(), Error> {
    // Then configure the outer data
    let msg = ManagementData {
        cmd: ManagmentDataType::Associate,
        target: None,
        msg: serde_cbor::to_vec(associate)?,
    };

    // If second encode looks good send it off
    let data = serde_cbor::to_vec(&msg)?;

    // Send over socket
    stream.write_message(Message::binary(data))?;

    Ok(())
}

/// Adds and OTA image from an included manifest file to the server
pub fn remove_ota(
    stream: &mut WebSocket<MaybeTlsStream<TcpStream>>,
    image_id: &str,
) -> Result<(), Error> {
    // Then configure the outer data
    let msg = ManagementData {
        cmd: ManagmentDataType::RemoveOta,
        target: None,
        msg: image_id.as_bytes().to_vec(),
    };

    // If second encode looks good send it off
    let data = serde_cbor::to_vec(&msg)?;

    // Send over socket
    stream.write_message(Message::binary(data))?;

    Ok(())
}

pub fn get_ota_group_list(stream: &mut WebSocket<MaybeTlsStream<TcpStream>>) -> Result<(), Error> {
    // Then configure the outer data
    let msg = ManagementData {
        cmd: ManagmentDataType::GetGroupList,
        target: None,
        msg: [].to_vec(),
    };

    // If second encode looks good send it off
    let data = serde_cbor::to_vec(&msg)?;

    // Send over socket
    stream.write_message(Message::binary(data))?;

    Ok(())
}

pub fn get_ota_image_list(stream: &mut WebSocket<MaybeTlsStream<TcpStream>>) -> Result<(), Error> {
    // Then configure the outer data
    let msg = ManagementData {
        cmd: ManagmentDataType::GetImageList,
        target: None,
        msg: [].to_vec(),
    };

    // If second encode looks good send it off
    let data = serde_cbor::to_vec(&msg)?;

    // Send over socket
    stream.write_message(Message::binary(data))?;

    Ok(())
}
