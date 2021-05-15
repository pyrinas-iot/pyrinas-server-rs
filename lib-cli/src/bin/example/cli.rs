use anyhow::anyhow;
use clap::{crate_version, Clap};
use pyrinas_cli::{certs, ota, CertCmd};
use pyrinas_cli::{ConfigCmd, ConfigSubCommand, OtaCmd, OtaSubCommand};
// use url::Url;

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
    let config = pyrinas_cli::get_config();

    // Process command.
    match opts.subcmd {
        SubCommand::Ota(c) => match c.subcmd {
            OtaSubCommand::Add(s) => {
                // Check if config is valid
                let config = match config {
                    Ok(c) => c,
                    Err(_e) => {
                        return Err(anyhow!(
                            "Unable to get config. Run \"init\" command before you continue."
                        ));
                    }
                };

                // Get socket
                let mut socket = match pyrinas_cli::get_socket(&config) {
                    Ok(s) => s,
                    Err(e) => {
                        return Err(anyhow!("Unable to get socket to Pyrinas! Err: {}", e));
                    }
                };

                // Then process
                if let Err(e) = crate::ota::add_ota(&mut socket, &s) {
                    eprintln!("Err: {}", e);
                    return Err(anyhow!("Unable to add OTA!"));
                };

                println!("OTA image successfully uploaded!");
            }
            OtaSubCommand::Remove(_r) => {
                // Check if config is valid
                let config = match config {
                    Ok(c) => c,
                    Err(_e) => {
                        return Err(anyhow!(
                            "Unable to get config. Run \"init\" command before you continue."
                        ));
                    }
                };

                // Get socket
                let mut _socket = match pyrinas_cli::get_socket(&config) {
                    Ok(s) => s,
                    Err(e) => {
                        eprintln!("{}", e);
                        return Err(anyhow!("Unable to get socket to Pyrinas!"));
                    }
                };

                // TODO: run the remove function
            }
        },
        SubCommand::Cert(c) => {
            // Check if config is valid
            let config = match config {
                Ok(c) => c,
                Err(_e) => {
                    return Err(anyhow!(
                        "Unable to get config. Run \"init\" command before you continue."
                    ))
                }
            };

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
                    // Check if config is valid
                    let config = match config {
                        Ok(c) => c,
                        Err(_e) => {
                            return Err(anyhow!(
                                "Unable to get config. Run \"init\" command before you continue."
                            ));
                        }
                    };

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
