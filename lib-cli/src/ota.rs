use chrono::{DateTime, Duration, Local, Utc};

// Pyrinas
use pyrinas_shared::ota::v2::{OTAImageData, OTAImageType, OTAPackage, OTAUpdate};
use pyrinas_shared::{
    ManagementData, ManagmentDataType, OtaGroupListResponse, OtaImageListResponse,
};

// Cbor
use minicbor;

// Std lib
use std::fs::File;
use std::io::{self, prelude::*};
use std::net::TcpStream;

// Websocket
use tungstenite::{protocol::WebSocket, stream::MaybeTlsStream, Message};

// Error handling
use thiserror::Error;

use crate::{git, OtaLink, OtaSubCommand};

#[derive(Debug, Error)]
pub enum Error {
    #[error("file error: {source}")]
    FileError {
        #[from]
        source: io::Error,
    },

    #[error("err: {0}")]
    CborError(String),

    /// Websocket error
    #[error("websocket error: {source}")]
    WebsocketError {
        #[from]
        source: tungstenite::Error,
    },

    /// Error to indicate repo is dirty
    #[error("repository is dirty. Run --force to override")]
    DirtyError,

    /// Error to indicate file(s) not found
    #[error("update not found")]
    UpdateNotFoundError,

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
            let image_id = crate::ota::add_ota(socket, &a.bin, a.force)?;

            println!("{} image successfully uploaded!", &image_id);

            // Do association
            match &a.device_id {
                Some(device_id) => {
                    let a = OtaLink {
                        device_id: Some(device_id.clone()),
                        group_id: Some(device_id.to_string()),
                        image_id: Some(image_id),
                    };

                    crate::ota::link(socket, &a)?;

                    println!("OTA Linked! {:?}", &a);
                }
                None => (),
            };
        }
        OtaSubCommand::Remove(r) => {
            crate::ota::remove_ota(socket, &r.image_id)?;

            println!("{} successfully removed!", &r.image_id);
        }
        OtaSubCommand::Unlink(a) => {
            crate::ota::unlink(socket, a)?;

            println!("OTA Unlinked! {:?}", a);
        }
        OtaSubCommand::Link(a) => {
            crate::ota::link(socket, a)?;

            println!("OTA Linked! {:?}", &a);
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

                        let list: OtaGroupListResponse = match minicbor::decode(&data) {
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

                        let list: OtaImageListResponse = match minicbor::decode(&data) {
                            Ok(m) => m,
                            Err(e) => {
                                eprintln!("Unable to get image list! Error: {}", e);
                                break;
                            }
                        };

                        for image in list.images.iter() {
                            let date_added =
                                DateTime::parse_from_rfc2822(&image.package.date_added)
                                    .unwrap()
                                    .with_timezone(&Local);

                            // Print out the entry
                            println!("{} {}", image.name, date_added);
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
    file_path: &Option<String>,
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
    let mut paths = vec![
        "./build/zephyr/app_update.bin",
        "./build/zephyr/zephyr.signed.bin",
    ];

    if let Some(path) = file_path {
        paths.clear();
        paths.push(path);
    }

    // Read image in as data
    let mut buf: Vec<u8> = Vec::new();
    let mut file: Option<File> = None;
    for entry in paths {
        if let Ok(f) = File::open(&entry) {
            file = Some(f);
        }
    }

    // Return error if not found
    let mut file = match file {
        Some(f) => f,
        None => return Err(Error::UpdateNotFoundError),
    };

    let size = file.read_to_end(&mut buf)?;

    println!("Reading {} bytes from firmware update binary.", size);

    // Data structure (from pyrinas_lib_shared)
    let new = OTAUpdate {
        device_uid: None,
        package: Some(OTAPackage {
            id: package_version.to_string(),
            version: package_version.clone(),
            file: Some(OTAImageData {
                data: buf,
                image_type: OTAImageType::Primary,
            }),
            size,
            date_added: Utc::now().to_string().to_string(),
        }),
    };

    // Serialize to cbor
    let data = match minicbor::to_vec(&new) {
        Ok(u) => u,
        Err(e) => return Err(Error::CborError(e.to_string())),
    };

    // Then configure the outer data
    let msg = ManagementData {
        cmd: ManagmentDataType::AddOta,
        target: None,
        msg: data,
    };

    // If second encode looks good send it off
    let data = match minicbor::to_vec(&msg) {
        Ok(u) => u,
        Err(e) => return Err(Error::CborError(e.to_string())),
    };

    // Send over socket
    stream.write_message(Message::binary(data))?;

    Ok(package_version.to_string())
}

pub fn unlink(
    stream: &mut WebSocket<MaybeTlsStream<TcpStream>>,
    link: &OtaLink,
) -> Result<(), Error> {
    // Then configure the outer data
    let msg = ManagementData {
        cmd: ManagmentDataType::UnlinkOta,
        target: None,
        msg: match minicbor::to_vec(link) {
            Ok(u) => u,
            Err(e) => return Err(Error::CborError(e.to_string())),
        },
    };

    // If second encode looks good send it off
    let data = match minicbor::to_vec(&msg) {
        Ok(u) => u,
        Err(e) => return Err(Error::CborError(e.to_string())),
    };

    // Send over socket
    stream.write_message(Message::binary(data))?;

    Ok(())
}

pub fn link(
    stream: &mut WebSocket<MaybeTlsStream<TcpStream>>,
    link: &OtaLink,
) -> Result<(), Error> {
    // Then configure the outer data
    let msg = ManagementData {
        cmd: ManagmentDataType::LinkOta,
        target: None,
        msg: match minicbor::to_vec(link) {
            Ok(u) => u,
            Err(e) => return Err(Error::CborError(e.to_string())),
        },
    };

    // If second encode looks good send it off
    let data = match minicbor::to_vec(&msg) {
        Ok(u) => u,
        Err(e) => return Err(Error::CborError(e.to_string())),
    };

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
    let data = match minicbor::to_vec(&msg) {
        Ok(u) => u,
        Err(e) => return Err(Error::CborError(e.to_string())),
    };

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
    let data = match minicbor::to_vec(&msg) {
        Ok(u) => u,
        Err(e) => return Err(Error::CborError(e.to_string())),
    };

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
    let data = match minicbor::to_vec(&msg) {
        Ok(u) => u,
        Err(e) => return Err(Error::CborError(e.to_string())),
    };

    // Send over socket
    stream.write_message(Message::binary(data))?;

    Ok(())
}
