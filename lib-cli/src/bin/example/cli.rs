use clap::{crate_version, Clap};
use pyrinas_cli::{ota, CertCmd, CliError};
use pyrinas_cli::{ConfigCmd, ConfigSubCommand, OtaCmd};

/// Command line utility to communicate with Pyrinas server over
/// a websockets connection.
#[derive(Clap)]
#[clap(version = crate_version!())]
struct Opts {
    #[clap(subcommand)]
    subcmd: SubCommand,
}

#[derive(Clap)]
#[clap(version = crate_version!())]
enum SubCommand {
    Ota(OtaCmd),
    Config(ConfigCmd),
    Cert(CertCmd),
}

fn main() -> Result<(), CliError> {
    // Opts from CLI
    let opts: Opts = Opts::parse();

    // Init logger
    env_logger::init();

    // Get config
    let config = match pyrinas_cli::get_config() {
        Ok(c) => c,
        Err(_e) => {
            return Err(CliError::CustomError(
                "Unable to get config. Run \"init\" command before you continue.".to_string(),
            ))
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
        SubCommand::Config(c) => {
            match c.subcmd {
                ConfigSubCommand::Show(_) => {
                    println!("{:?}", config);
                }
                ConfigSubCommand::Init => {
                    // Default config (blank)
                    let c = Default::default();

                    // TODO: migrate config on update..

                    // Set the config from init struct
                    pyrinas_cli::set_config(&c)?;

                    println!("Config successfully added!");
                }
            }
        }
    }

    Ok(())
}
