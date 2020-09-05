use awscreds::Credentials;
use chrono::Utc;
use log::error;
use pyrinas_shared::settings;
use s3::bucket::Bucket;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;

pub struct OtaInfo {
  pub server_filename: String,
}

pub fn upload_ota_to_aws(settings: &settings::PyrinasSettings, file: &str, uid: &str) -> OtaInfo {
  // Set up AWS conection
  let credentials = Credentials::new_blocking(
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
    .put_object_blocking(&server_filename, &buffer, "application/octet-stream")
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
