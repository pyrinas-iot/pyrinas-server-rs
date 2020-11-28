use clap::{crate_version, Clap};
use log::error;
use pyrinas_cli::ota;
use pyrinas_cli::OtaAdd;
use pyrinas_shared::settings;
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
      crate::ota::add_ota_from_manifest(&settings, &mut stream, &s);
    }
  }
}
