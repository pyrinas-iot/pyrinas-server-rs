use crate::OTAManifest;
use pyrinas_shared::settings;
use serde_cbor;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::net::Shutdown;
use std::os::unix::net::UnixStream;

pub struct OtaInfo {
    pub server_filename: String,
}

pub fn add_ota_from_manifest(
    settings: &settings::PyrinasSettings,
    stream: &mut UnixStream,
    s: &crate::OtaAdd,
) {
    // Open the file in read-only mode with buffer.
    let file = File::open(&s.manifest).expect("Unable to open manifest file!");
    let reader = BufReader::new(file);

    // Read manifest file
    let manifest: OTAManifest =
        serde_json::from_reader(reader).expect("Unable to deserialze JSON manifest file!");

    println!(
        "Adding new update for: {} on device: {}",
        manifest.file, s.uid
    );

    // Read image in as data
    let mut buf: Vec<u8> = Vec::new();
    let mut file = File::open(&manifest.file).expect("Unable to open firmware update binary.");
    let size = file
        .read_to_end(&mut buf)
        .expect("Error reading from binary file.");

    println!("Reading {} bytes from firmware update binary.", size);

    // Data structure (from pyrinas_lib_shared)
    let new = pyrinas_shared::OtaUpdate {
        uid: s.uid.clone(),
        package: Some(pyrinas_shared::OTAPackage {
            version: manifest.version,
            host: format!("{}/images/", &settings.ota.url),
            file: manifest.file,
            force: manifest.force,
        }),
        image: Some(buf),
    };

    // Serialize to cbor
    if let Ok(data) = serde_cbor::to_vec(&new) {
        let msg = pyrinas_shared::ManagementData {
            target: "add_ota".to_string(),
            msg: data,
        };

        // If second encode looks good send it off
        if let Ok(data) = serde_cbor::to_vec(&msg) {
            // Send over socket
            stream.write_all(&data).unwrap_or_else(|_| {
                println!("Unable to write to {}.", &settings.sock.path);
                std::process::exit(1);
            });
        }
    }

    // Close socket
    stream
        .shutdown(Shutdown::Both)
        .expect("shutdown function failed");
}
