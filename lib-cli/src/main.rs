use clap::{crate_version, Clap};
use log::error;
use pyrinas_cli::ota;
use pyrinas_shared::settings;
use std::io::prelude::*;
use std::net::Shutdown;
use std::os::unix::net::UnixStream;

/// This doc string acts as a help message when the user runs '--help'
/// as do all doc strings on fields
#[derive(Clap)]
#[clap(version = crate_version!())]
struct Opts {
  #[clap(short, long, default_value = "config.toml")]
  config: String,
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
  // Opts from CLI
  let opts: Opts = Opts::parse();

  // Init logger
  env_logger::init();

  // Parse config file
  let settings = settings::PyrinasSettings::new(opts.config.clone()).unwrap_or_else(|e| {
    error!("Unable to parse config at: {}. Error: {}", &opts.config, e);
    std::process::exit(1);
  });

  // Connect to unix socket
  let mut stream = UnixStream::connect(&settings.sock.path).unwrap_or_else(|_| {
    println!(
      "Unable to connect to {}. Server started?",
      &settings.sock.path
    );
    std::process::exit(1);
  });

  match opts.subcmd {
    SubCommand::Add(s) => {
      println!("Adding new update for: {} on device: {}", s.file, s.uid);

      // Upload to AWS
      let ota_info = ota::upload_ota_to_aws(&settings, &s.file, &s.uid);

      // TODO: this can be used in the server..
      // let url = bucket.presign_put(filename, 180).unwrap_or_else(|e| {
      //   error!("File not found! {}", e);
      //   std::process::exit(1);
      // });
      // println!("Presigned url: {}", url);

      // Get host name
      let file_host = format!("https://{}.s3.amazonaws.com/", &settings.s3.bucket);

      // Data structure (from pyrinas_shared)
      let new = pyrinas_shared::OtaUpdate {
        uid: s.uid,
        package: pyrinas_shared::OTAPackage {
          version: s.version,
          host: file_host,
          file: ota_info.server_filename,
          force: s.force,
        },
      };

      // serialize to json
      let j = serde_json::to_string(&new).expect("Unable to encode NewOTA struct");

      println!("Serialzed: {}", j);

      // Send over socket
      stream.write_all(&j.into_bytes()).unwrap_or_else(|_| {
        println!("Unable to write to {}.", &settings.sock.path);
        std::process::exit(1);
      });

      // Close socket
      stream
        .shutdown(Shutdown::Both)
        .expect("shutdown function failed");
    }
  }
}
