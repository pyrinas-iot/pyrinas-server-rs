use awscreds::Credentials;
use chrono::Utc;
use clap::{crate_version, Clap};
use dotenv;
use log::error;
use s3::bucket::Bucket;
use std::fs::File;
use std::io::prelude::*;
use std::net::Shutdown;
use std::os::unix::net::UnixStream;
use std::path::Path;

/// This doc string acts as a help message when the user runs '--help'
/// as do all doc strings on fields
#[derive(Clap)]
#[clap(version = crate_version!())]
struct Opts {
  #[clap(subcommand)]
  subcmd: SubCommand,
}

#[derive(Clap)]
#[clap(version = crate_version!())]
enum SubCommand {
  Add(OtaAdd),
}

/// Add a new OTA package to the sever
#[derive(Clap, Debug)]
#[clap(version = crate_version!())]
struct OtaAdd {
  /// UID for the device being targeted
  #[clap(long)]
  uid: String,
  /// Version string
  #[clap(long)]
  version: String,
  /// File path
  #[clap(long)]
  file: String,
  /// Force update on same version
  #[clap(long)]
  force: bool,
}

fn main() {
  let opts: Opts = Opts::parse();

  // Init logger
  env_logger::init();

  // Parse .env file
  dotenv::dotenv().unwrap_or_else(|e| {
    error!("dotnev parsing failed! {}", e);
    std::process::exit(1);
  });

  let socket = dotenv::var("PYRINAS_SOCKET_PATH").unwrap_or_else(|_| {
    error!("PYRINAS_SOCKET_PATH must be set in environment!");
    std::process::exit(1);
  });

  let file_host = dotenv::var("PYRINAS_FILE_HOST").unwrap_or_else(|_| {
    error!("PYRINAS_FILE_HOST must be set in environment!");
    std::process::exit(1);
  });

  let aws_access_key = dotenv::var("PYRINAS_AWS_ACCESS_KEY").unwrap_or_else(|_| {
    error!("PYRINAS_AWS_ACCESS_KEY must be set in environment!");
    std::process::exit(1);
  });

  let aws_secret_key = dotenv::var("PYRINAS_AWS_SECRET_KEY").unwrap_or_else(|_| {
    error!("PYRINAS_AWS_SECRET_KEY must be set in environment!");
    std::process::exit(1);
  });

  let aws_region = dotenv::var("PYRINAS_AWS_REGION").unwrap_or_else(|_| {
    error!("PYRINAS_AWS_REGION must be set in environment!");
    std::process::exit(1);
  });

  let aws_bucket = dotenv::var("PYRINAS_AWS_BUCKET").unwrap_or_else(|_| {
    error!("PYRINAS_AWS_BUCKET must be set in environment!");
    std::process::exit(1);
  });

  // Set up AWS conection
  let region = aws_region.parse().expect("Unable to parse AWS region.");
  let credentials = Credentials::new_blocking(
    Some(&aws_access_key),
    Some(&aws_secret_key),
    None,
    None,
    None,
  )
  .unwrap_or_else(|e| {
    error!("Unable to create AWS credentials! {}", e);
    std::process::exit(1);
  });

  // Create bucket
  let bucket = Bucket::new(&aws_bucket, region, credentials).expect("Unable to create bucket!");

  // Connect to unix socket
  let mut stream = UnixStream::connect(&socket).unwrap_or_else(|_| {
    println!("Unable to connect to {}. Server started?", socket);
    std::process::exit(1);
  });

  match opts.subcmd {
    SubCommand::Add(s) => {
      println!("Adding new update for: {} on device: {}", s.file, s.uid);

      // Get current Datetime
      let timestamp = Utc::now().timestamp();

      // Open and upload file
      let path = Path::new(&s.file);
      if !path.exists() {
        error!("{} not found!", path.to_str().unwrap());
        std::process::exit(1);
      }
      // Get the filename
      let filename = path.file_name().unwrap().to_str().unwrap();

      // Create the target file name
      let server_filename = format!("{}_{}_{}", s.uid, timestamp, filename);

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

      // TODO: this can be used in the server..
      // let url = bucket.presign_put(filename, 180).unwrap_or_else(|e| {
      //   error!("File not found! {}", e);
      //   std::process::exit(1);
      // });
      // println!("Presigned url: {}", url);

      // Data structure (from pyrinas_shared)
      let new = pyrinas_shared::NewOta {
        uid: s.uid,
        package: pyrinas_shared::OTAPackage {
          version: s.version,
          host: file_host,
          file: server_filename,
          force: s.force,
        },
      };

      // serialize to json
      let j = serde_json::to_string(&new).expect("Unable to encode NewOTA struct");

      println!("Serialzed: {}", j);

      // Send over socket
      stream.write_all(&j.into_bytes()).unwrap_or_else(|_| {
        println!("Unable to write to {}.", socket);
        std::process::exit(1);
      });

      // Close socket
      stream
        .shutdown(Shutdown::Both)
        .expect("shutdown function failed");
    }
  }
}
