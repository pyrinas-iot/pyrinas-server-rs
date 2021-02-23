use crate::OTAManifest;
use chrono::Utc;
use log::error;
use pyrinas_shared::settings;
use s3::{bucket::Bucket,creds::Credentials};
use serde_cbor;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::net::Shutdown;
use std::os::unix::net::UnixStream;
use std::path::Path;

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

  // Upload to AWS
  let ota_info = upload_ota_to_aws(&settings, &manifest.file, &s.uid);

  // FIXME: this can be used in the server..
  // let url = bucket.presign_put(filename, 180).unwrap_or_else(|e| {
  //   error!("File not found! {}", e);
  //   std::process::exit(1);
  // });
  // println!("Presigned url: {}", url);

  // Get host name
  let file_host = format!("https://{}.s3.amazonaws.com/", &settings.s3.bucket);

  // Data structure (from pyrinas_lib_shared)
  let new = pyrinas_shared::OtaUpdate {
    uid: s.uid.clone(),
    package: Some(pyrinas_shared::OTAPackage {
      version: manifest.version,
      host: file_host,
      file: ota_info.server_filename,
      force: manifest.force,
    }),
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

pub fn upload_ota_to_aws(settings: &settings::PyrinasSettings, file: &str, uid: &str) -> OtaInfo {
  // Set up AWS conection
  let credentials = Credentials::new(
    Some(&settings.s3.access_key),
    Some(&settings.s3.secret_key),
    None,
    None,
    None,
  )
  .unwrap_or_else(|e| {
    error!("Unable to create AWS credentials! {}", e);
    std::process::exit(1);
  });

  // Create bucket
  let region = settings
    .s3
    .region
    .parse()
    .expect("Unable to parse AWS region.");
  let bucket =
    Bucket::new(&settings.s3.bucket, region, credentials).expect("Unable to create bucket!");

  // Get current Datetime
  let timestamp = Utc::now().timestamp();

  // Open and upload file
  let path = Path::new(&file);
  if !path.exists() {
    error!("{} not found!", path.to_str().unwrap());
    std::process::exit(1);
  }
  // Get the filename
  let filename = path.file_name().unwrap().to_str().unwrap();

  // Create the target file name
  let server_filename = format!("{}_{}_{}", &uid, timestamp, filename);

  // Open the file
  let mut file = File::open(path).expect("Unable to open file!");

  let mut buffer = Vec::new();
  // read the whole file
  file
    .read_to_end(&mut buffer)
    .expect("Unable to read to end");

  // Upload file to AWS
  let (_, status_code) = bucket
    .put_object_blocking(&server_filename, &buffer)
    .unwrap_or_else(|e| {
      error!("Unable to upload {}! Error: {}", &server_filename, e);
      std::process::exit(1);
    });

  assert_eq!(200, status_code);

  // Return info
  OtaInfo {
    server_filename: server_filename,
  }
}
