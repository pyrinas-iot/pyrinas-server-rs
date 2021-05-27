use anyhow::anyhow;
use chrono::{Duration, Utc};
use clap::{crate_version, Clap};
use pyrinas_cli::{certs, ota, CertCmd};
use pyrinas_cli::{ConfigCmd, ConfigSubCommand, OtaCmd, OtaSubCommand};

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
    let config = match pyrinas_cli::get_config() {
        Ok(c) => c,
        Err(e) => return Err(anyhow!("Unable to get config. Error: {}", e)),
    };

    // Process command.
    match opts.subcmd {
        SubCommand::Ota(c) => {
            // Get socket
            let mut socket = pyrinas_cli::get_socket(&config)?;

            match c.subcmd {
                OtaSubCommand::Add => {
                    crate::ota::add_ota(&mut socket)?;

                    println!("OTA image successfully uploaded!");
                }
                OtaSubCommand::Remove(r) => {
                    crate::ota::remove_ota(&mut socket, &r.image_id)?;

                    println!("{} successfully removed!", &r.image_id);
                }
                OtaSubCommand::Associate(a) => {
                    crate::ota::associate(&mut socket, &a)?;

                    println!("Associated! {:?}", &a);
                }
                OtaSubCommand::ListGroups => {
                    crate::ota::get_ota_group_list(&mut socket)?;

                    let start = Utc::now();

                    // Get message
                    loop {
                        if Utc::now() > start + Duration::seconds(10) {
                            eprintln!("No response from server!");
                            break;
                        }

                        match socket.read_message() {
                            Ok(msg) => {
                                println!("{:?}", msg);
                                // TODO: do stuff with message
                            }
                            Err(_) => continue,
                        };
                    }
                }
                OtaSubCommand::ListImages => {
                    crate::ota::get_ota_image_list(&mut socket)?;

                    let start = Utc::now();

                    // Get message
                    loop {
                        if Utc::now() > start + Duration::seconds(10) {
                            eprintln!("No response from server!");
                            break;
                        }

                        match socket.read_message() {
                            Ok(msg) => {
                                println!("{:?}", msg);
                                // TODO: do stuff with message
                            }
                            Err(_) => continue,
                        };
                    }
                }
            };
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
