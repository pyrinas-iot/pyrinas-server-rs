use anyhow::anyhow;
use clap::{crate_version, Clap};
use pyrinas_cli::{certs, ota, CertCmd};
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

fn main() -> anyhow::Result<()> {
    // Opts from CLI
    let opts: Opts = Opts::parse();

    // Init logger
    env_logger::init();

    // Get config
    let config = match pyrinas_cli::get_config() {
        Ok(c) => c,
        Err(e) => return Err(anyhow!("Unable to get config. Error: {}", e)),
    };

    // Process command.
    match opts.subcmd {
        SubCommand::Ota(c) => {
            // Get socket
            let mut socket = pyrinas_cli::get_socket(&config)?;

            crate::ota::ota_process(&mut socket, &c.subcmd)?;
        }
        SubCommand::Cert(c) => {
            // Depending on the input, create CA, server or client cert
            match c.subcmd {
                pyrinas_cli::CertSubcommand::Ca => {
                    certs::generate_ca_cert(&config.cert)?;
                }
                pyrinas_cli::CertSubcommand::Server => {
                    certs::generate_server_cert(&config.cert)?;
                }
                pyrinas_cli::CertSubcommand::Device { id } => {
                    certs::generate_device_cert(&config.cert, &id)?;
                }
            }
        }
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
                    if let Err(e) = pyrinas_cli::set_config(&c) {
                        return Err(anyhow!("Unable to set config. Err: {}", e));
                    };

                    println!("Config successfully added!");
                }
            }
        }
    }

    Ok(())
}
