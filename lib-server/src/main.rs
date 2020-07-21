mod mqtt;

use dotenv;
use log::{error, info};
use std::{env, process};

fn main() {
    // Parse .env file
    dotenv::dotenv().ok();

    let trust_store_env = dotenv::var("TRUST_STORE").unwrap_or_else(|_| {
        error!("TRUST_STORE must be set in environment!");
        process::exit(1);
    });

    let key_store_env = dotenv::var("KEY_STORE").unwrap_or_else(|_| {
        error!("KEY_STORE must be set in environment!");
        process::exit(1);
    });

    let private_key_env = dotenv::var("PRIVATE_KEY").unwrap_or_else(|_| {
        error!("PRIVATE_KEY must be set in environment!");
        process::exit(1);
    });

    // We assume that we are in a valid directory.
    let mut trust_store = env::current_dir().unwrap();
    trust_store.push(trust_store_env);

    let mut key_store = env::current_dir().unwrap();
    key_store.push(key_store_env);

    let mut private_key = env::current_dir().unwrap();
    private_key.push(private_key_env);

    if !trust_store.exists() {
        error!("The trust store file does not exist: {:?}", trust_store);
        process::exit(1);
    }

    if !key_store.exists() {
        error!("The key store file does not exist: {:?}", key_store);
        process::exit(1);
    }

    if !private_key.exists() {
        error!("The key store file does not exist: {:?}", private_key);
        process::exit(1);
    }

    // Initialize the logger from the environment
    env_logger::init();

    // TODO: init influx connection

    // TODO: init http service

    // Let the user override the host, but note the "ssl://" protocol.
    let host = dotenv::var("HOST").unwrap_or_else(|_| {
        error!("HOST must be set in environment!");
        process::exit(1);
    });

    // ? Start mqtt thread(?)
    let mut cloud = mqtt::MQTTCloud::new(&host);
    cloud.set_certs(
        trust_store.to_str().unwrap(),
        private_key.to_str().unwrap(),
        key_store.to_str().unwrap(),
    );
    cloud.start();

    info!("Done!");
}
