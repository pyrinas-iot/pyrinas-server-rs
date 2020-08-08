use dotenv;
use log::{debug, error, info};
use std::{collections::hash_map::HashMap, process, str};

// Tokio Related
use tokio::io::AsyncReadExt;
use tokio::net::UnixListener;
use tokio::stream::StreamExt;
use tokio::sync::mpsc::{channel, Sender};
use tokio::task;

// Influx Related
use influxdb::{Client, InfluxDbWriteable};

// Local lib related
mod broker;
mod mqtt;
mod sled;
use pyrinas_shared::Event;

// Only requires a sender. No response necessary here... yet.
async fn sock_run(mut broker_sender: Sender<Event>) {
    let socket_path = dotenv::var("PYRINAS_SOCKET_PATH").unwrap_or_else(|_| {
        error!("PYRINAS_SOCKET_PATH must be set in environment!");
        process::exit(1);
    });

    // Get the sender/reciever associated with this particular task
    let (sender, _) = channel::<pyrinas_shared::Event>(20);

    // Register this task
    broker_sender
        .send(Event::NewRunner {
            name: "sock".to_string(),
            sender: sender.clone(),
        })
        .await
        .unwrap();

    debug!("Removing previous sock!");

    // Remove previous socket
    let _ = std::fs::remove_file(&socket_path);

    // Make connection
    let mut listener = UnixListener::bind(&socket_path).expect("Unable to bind!");
    let mut incoming = listener.incoming();

    debug!("Created socket listener!");

    while let Some(stream) = incoming.next().await {
        debug!("Got stream!");

        // Setup work to make this happen
        let mut stream = stream.unwrap(); //Box::<UnixStream>::new(stream.unwrap());
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

        // Send result back to broker
        let _ = broker_sender
            .send(Event::OtaNewPackage {
                uid: res.uid,
                package: res.package,
            })
            .await;
    }
}

async fn influx_run(mut broker_sender: Sender<Event>) {
    // All the vars involved
    let env_vars = vec![
        String::from("PYRINAS_INFLUX_HOST"),
        String::from("PYRINAS_INFLUX_HOST_PORT"),
        String::from("PYRINAS_INFLUX_DB"),
        String::from("PYRINAS_INFLUX_USER"),
        String::from("PYRINAS_INFLUX_PASSWORD"),
    ];

    // Used for storing temporary array of input params
    let mut params = HashMap::new();

    // Iterate and get each of the environment variables
    for item in env_vars.iter() {
        let ret = dotenv::var(item).unwrap_or_else(|_| {
            error!("{} must be set in environment!", item);
            process::exit(1);
        });

        // Insert ret into map
        params.insert(item, ret);
    }

    // Get the sender/reciever associated with this particular task
    let (sender, mut reciever) = channel::<pyrinas_shared::Event>(20);

    // Register this task
    broker_sender
        .send(Event::NewRunner {
            name: "influx".to_string(),
            sender: sender.clone(),
        })
        .await
        .unwrap();

    // Set up the URL
    let host = params.get(&env_vars[0]).unwrap();
    let port = params.get(&env_vars[1]).unwrap();
    let url = format!("http://{}:{}", host, port);

    // Get the db params
    let db_name = params.get(&env_vars[2]).unwrap();
    let user = params.get(&env_vars[3]).unwrap();
    let password = params.get(&env_vars[4]).unwrap();

    // Create the client
    let client = Client::new(url, db_name).with_auth(user, password);

    // Process putting new data away
    while let Some(event) = reciever.recv().await {
        // Process telemetry and app data
        match event {
            Event::TelemetryData { uid, msg } => {
                info!("influx_run: TelemetryData");

                // Convert to data used by influx
                let data = msg.to_influx_data(uid);

                // Query
                let query = data.into_query("telemetry");

                // Create the query. Shows error if it fails
                if let Err(e) = client.query(&query).await {
                    error!("Unable to write query. Error: {}", e);
                }
            }
            Event::ApplicationData { uid: _, msg: _ } => {
                info!("influx_run: ApplicationData");
            }
            _ => (),
        };
    }
}

#[tokio::main()]
async fn main() {
    // Initialize the logger from the environment
    env_logger::init();

    // Parse .env file
    dotenv::dotenv().ok();

    // Channels for communication
    let (broker_sender, broker_reciever) = channel::<pyrinas_shared::Event>(100);

    // Init influx connection
    let influx_task = task::spawn(influx_run(broker_sender.clone()));

    // TODO: init http service

    // Start sled task
    let sled_task = task::spawn(sled::run(broker_sender.clone()));

    // Start unix socket task
    let unix_sock_task = task::spawn(sock_run(broker_sender.clone()));

    // Spawn a new task for the MQTT stuff
    let mqtt_task = task::spawn(mqtt::run(broker_sender.clone()));

    // Spawn the broker task that handles it all!
    let broker_task = task::spawn(broker::run(broker_reciever));

    // Join hands kids
    let _join = tokio::join!(
        sled_task,
        influx_task,
        unix_sock_task,
        mqtt_task,
        broker_task
    );

    info!("Done!");
}
