mod mqtt;

use async_std::os::unix::net::UnixListener;
use async_std::sync::{channel, Receiver, Sender};
use async_std::{prelude::*, task};
use dotenv;
use log::{error, info};
use std::{env, process, str};

async fn sled_run(rx_chan: Receiver<pyrinas_shared::NewOta>) {
    let sled_db = dotenv::var("PYRINAS_SLED_DB").unwrap_or_else(|_| {
        error!("PYRINAS_SLED_DB must be set in environment!");
        process::exit(1);
    });

    // Open the DB
    let tree = sled::open(sled_db).expect("Error opening sled db.");

    // Wait to get something from the channel
    loop {
        let entry: pyrinas_shared::NewOta = rx_chan.recv().await.unwrap();

        // TODO: handle tracking state in here

        info!("Entry: {:?}", entry);

        if tree.contains_key(&entry.uid).unwrap() {
            info!("Has key! {}", entry.uid);
        }

        // Turn entry.package into CBOR
        let res = serde_cbor::ser::to_vec_packed(&entry.package);

        // Write into database
        match res {
            Ok(cbor_data) => {
                if let Err(e) = tree.insert(entry.uid, cbor_data) {
                    error!("Unable to insert into sled. Error: {}", e);
                }

                // Save it to disk
                // TODO: evaluate if this bogs things down..
                if let Err(e) = tree.flush_async().await {
                    error!("Unable to flush tree. Error: {}", e);
                }
            }
            Err(e) => {
                error!("Unable to serialize. Error: {}", e);
            }
        }
    }
}

fn cloud_run() {
    let trust_store_env = dotenv::var("PYRINAS_TRUST_STORE").unwrap_or_else(|_| {
        error!("PYRINAS_TRUST_STORE must be set in environment!");
        process::exit(1);
    });

    let key_store_env = dotenv::var("PYRINAS_KEY_STORE").unwrap_or_else(|_| {
        error!("PYRINAS_KEY_STORE must be set in environment!");
        process::exit(1);
    });

    let private_key_env = dotenv::var("PYRINAS_PRIVATE_KEY").unwrap_or_else(|_| {
        error!("PYRINAS_PRIVATE_KEY must be set in environment!");
        process::exit(1);
    });

    // Let the user override the host, but note the "ssl://" protocol.
    let host = dotenv::var("PYRINAS_HOST").unwrap_or_else(|_| {
        error!("PYRINAS_HOST must be set in environment!");
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

    // Configure MQTTCloud
    let mut cloud = mqtt::MQTTCloud::new(&host);
    cloud.set_certs(
        trust_store.to_str().unwrap(),
        private_key.to_str().unwrap(),
        key_store.to_str().unwrap(),
    );

    cloud.start();
}

async fn sock_run(tx_chan: Sender<pyrinas_shared::NewOta>) {
    let socket_path = dotenv::var("PYRINAS_SOCKET_PATH").unwrap_or_else(|_| {
        error!("PYRINAS_SOCKET_PATH must be set in environment!");
        process::exit(1);
    });

    info!("Removing previous sock!");

    // Remove previous socket
    let _ = std::fs::remove_file(&socket_path);

    // Make connection
    let listener = UnixListener::bind(&socket_path)
        .await
        .expect("Unable to bind!");
    let mut incoming = listener.incoming();

    info!("Created socket listener!");

    while let Some(stream) = incoming.next().await {
        info!("Got stream!");

        // Setup work to make this happen
        let mut stream = stream.unwrap();
        let mut buffer = Vec::new();

        // Read until the stream closes
        stream
            .read_to_end(&mut buffer)
            .await
            .expect("Unable to write to socket.");

        // Get string from the buffer
        let s = str::from_utf8(&buffer).unwrap();
        info!("{}", s);

        // Decode into struct
        let res: pyrinas_shared::NewOta = serde_json::from_str(&s).unwrap();

        // Push this result into a channel so it can be added to sled
        tx_chan.send(res).await;

        // TODO notify the MQTT task that something cool has happened

        // TODO: close stream?
    }
}

fn main() {
    // Initialize the logger from the environment
    env_logger::init();

    // Parse .env file
    dotenv::dotenv().ok();

    // Channels for communication
    let (sock_sled_tx, sock_sled_rx) = channel::<pyrinas_shared::NewOta>(20);
    // let (sock_ota_tx, sock_ota_rx) = channel::<pyrinas_shared::NewOta>(100);

    // TODO: init influx connection

    // TODO: init http service

    // Start sled task
    let _sled_task = task::spawn(sled_run(sock_sled_rx.clone()));

    // Start unix socket task
    let _unix_sock_task = task::spawn(sock_run(sock_sled_tx.clone()));

    // Spawn a new task for the MQTT stuff
    let _cloud_task = task::block_on(async {
        cloud_run();
    });

    // Spawn a task for sending ota bits to devices
    // let ota_process_task = task::spawn(async {
    //     // TODO:check if connected
    // });

    info!("Done!");
}
