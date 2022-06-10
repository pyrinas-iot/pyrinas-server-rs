use clap::Parser;
use pyrinas_cli::{ota, CertCmd, Error};
use pyrinas_cli::{ConfigCmd, OtaCmd};

/// Command line utility to communicate with Pyrinas server over
/// a websockets connection.
#[derive(Parser)]
#[clap(version)]
struct Opts {
    #[clap(subcommand)]
    subcmd: SubCommand,
}

#[derive(Parser)]
#[clap(version)]
enum SubCommand {
    Ota(OtaCmd),
    Config(ConfigCmd),
    Cert(CertCmd),
}

fn main() -> Result<(), Error> {
    // Opts from CLI
    let opts: Opts = Opts::parse();

    // Init logger
    env_logger::init();

    // Get config
    let config = match pyrinas_cli::config::get_config() {
        Ok(c) => c,
        Err(e) => {
            match e {
                pyrinas_cli::config::Error::HomeError => eprintln!("Unable to get home path!"),
                pyrinas_cli::config::Error::FileError { source: _ } => {
                    eprintln!("Unable to get config. Run \"init\" command before you continue.")
                }
                pyrinas_cli::config::Error::TomlError { source } => {
                    eprintln!("Error reading config file. Err: {}", source)
                }
            };

            return Ok(());
        }
    };

    // Process command.
    match opts.subcmd {
        // Process OTA commands (needs to be connected)
        SubCommand::Ota(c) => {
            // Get socket
            let mut socket = pyrinas_cli::get_socket(&config)?;

            crate::ota::process(&mut socket, &c.subcmd)?;
        }
        // Depending on the input, create CA, server or client cert
        SubCommand::Cert(c) => pyrinas_cli::certs::process(&config, &c)?,
        // Process config commands
        SubCommand::Config(c) => pyrinas_cli::config::process(&config, &c)?,
    }

    Ok(())
}
