use clap::{crate_version, Clap};
use pyrinas_cli::ota;
use pyrinas_cli::{ConfigCmd, ConfigSubCommand, OtaCmd};
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
}

fn main() {
    // Opts from CLI
    let opts: Opts = Opts::parse();

    // Init logger
    env_logger::init();

    // Get config
    let config = pyrinas_cli::get_config();

    // Process command.
    match opts.subcmd {
        SubCommand::Ota(c) => match c {
            OtaCmd::Add(s) => {
                // Check if config is valid
                let config = match config {
                    Ok(c) => c,
                    Err(_e) => {
                        eprintln!(
                            "Unable to get config. Run \"init\" command before you continue."
                        );
                        return;
                    }
                };

                // Get socket
                let mut socket = match pyrinas_cli::get_socket(&config) {
                    Ok(s) => s,
                    Err(e) => {
                        eprintln!("{}", e);
                        return;
                    }
                };

                // Then process
                if let Err(e) = crate::ota::add_ota(&mut socket, &s) {
                    eprintln!("Err: {}", e);
                    return;
                };

                println!("OTA image successfully uploaded!");
            }
            OtaCmd::Remove(_r) => {
                // Check if config is valid
                let config = match config {
                    Ok(c) => c,
                    Err(_e) => {
                        eprintln!(
                            "Unable to get config. Run \"init\" command before you continue."
                        );
                        return;
                    }
                };

                // Get socket
                let mut _socket = match pyrinas_cli::get_socket(&config) {
                    Ok(s) => s,
                    Err(e) => {
                        eprintln!("{}", e);
                        return;
                    }
                };

                // TODO: run the remove function
            }
        },
        SubCommand::Config(c) => {
            match c.subcmd {
                ConfigSubCommand::Show(_) => {
                    // Check if config is valid
                    let config = match config {
                        Ok(c) => c,
                        Err(_e) => {
                            eprintln!(
                                "Unable to get config. Run \"init\" command before you continue."
                            );
                            return;
                        }
                    };

                    println!("{:?}", config);
                }
                ConfigSubCommand::Install(c) => {
                    // Set the config from init struct
                    if let Err(e) = pyrinas_cli::set_config(&c) {
                        eprintln!("Unable to set config. Err: {}", e);
                        return;
                    };

                    println!("Config successfully added!");
                }
            }
        }
    }
}
