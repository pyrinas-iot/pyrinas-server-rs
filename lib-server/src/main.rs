use async_std::os::unix::net::UnixListener;
use async_std::sync::{channel, Receiver, Sender};
use async_std::{prelude::*, task};
use dotenv;
use log::{debug, error, info};
use paho_mqtt;
use pyrinas_shared::Event;
use std::{
    collections::hash_map::{Entry, HashMap},
    env, process, str,
    sync::RwLock,
    thread,
    time::Duration,
};

const TOPICS: &[&str] = &["+/+/pub"];
const QOS: &[i32] = &[1];

async fn sled_run(broker_sender: Sender<Event>) {
    // Get the sender/reciever associated with this particular task
    let (sender, mut reciever) = channel::<pyrinas_shared::Event>(20);

    // Register this task
    broker_sender
        .send(Event::NewRunner {
            name: "sled".to_string(),
            sender,
        })
        .await;

    let sled_db = dotenv::var("PYRINAS_SLED_DB").unwrap_or_else(|_| {
        error!("PYRINAS_SLED_DB must be set in environment!");
        process::exit(1);
    });

    // Open the DB
    let tree = sled::open(sled_db).expect("Error opening sled db.");

    // TODO: wait for event on reciever
    while let Some(event) = reciever.next().await {
        // Only process NewOtaPackage events
        match event {
            Event::NewOtaPackage { uid, package } => {
                info!("sled_run: Event::NewOtaPackage");

                // Turn entry.package into CBOR
                let res = serde_cbor::ser::to_vec_packed(&package);

                // Write into database
                match res {
                    Ok(cbor_data) => {
                        if let Err(e) = tree.insert(uid, cbor_data) {
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
            _ => (),
        }
    }
}

type UserTopics = RwLock<Vec<String>>;

// fn on_connect_success(cli: &paho_mqtt::AsyncClient, _msgid: u16) {
//     println!("Connection succeeded");
//     let data = cli.user_data().unwrap();

//     if let Some(lock) = data.downcast_ref::<UserTopics>() {
//         let topics = lock.read().unwrap();
//         println!("Subscribing to topics: {:?}", topics);

//         // Create a QoS vector, same len as # topics
//         let qos = vec![QOS; topics.len()];
//         // Subscribe to the desired topic(s).
//         cli.subscribe_many(&topics, &qos);
//         // TODO: This doesn't yet handle a failed subscription.
//     }
// }

// fn on_connect_failure(cli: &paho_mqtt::AsyncClient, _msgid: u16, rc: i32) {
//     println!("Connection attempt failed with error code {}.\n", rc);
//     thread::sleep(Duration::from_millis(2500));
//     cli.reconnect_with_callbacks(on_connect_success, on_connect_failure);
// }

async fn mqtt_run(broker_sender: Sender<Event>) {
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

    // Get the sender/reciever associated with this particular task
    let (sender, mut reciever) = channel::<pyrinas_shared::Event>(20);

    // Register this task
    broker_sender
        .send(Event::NewRunner {
            name: "mqtt".to_string(),
            sender,
        })
        .await;

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

    // let topics: Vec<String> = TOPICS.iter().map(|s| s.to_string()).collect();

    // Create a client options
    let create_opts = paho_mqtt::CreateOptionsBuilder::new()
        .server_uri(host)
        .client_id("ssl_publish_rs")
        .max_buffered_messages(100)
        .finalize();

    let mut client = paho_mqtt::AsyncClient::new(create_opts).unwrap_or_else(|e| {
        println!("Error creating the client: {:?}", e);
        process::exit(1);
    });

    let ssl_opts = paho_mqtt::SslOptionsBuilder::new()
        .trust_store(trust_store)
        .unwrap()
        .key_store(key_store)
        .unwrap()
        .private_key(private_key)
        .unwrap()
        .finalize();

    // client.set_message_callback(|_client, msg| {
    //     if let Some(msg) = msg {
    //         let topic = msg.topic();
    //         let _payload_str = msg.payload_str();

    //         info!("Recieved! {}", topic);
    //     }
    // });

    // info!("Connecting to MQTT broker.");
    // client.connect_with_callbacks(conn_opts, on_connect_success, on_connect_failure);

    // let mqtt_recieve_task = task::spawn(async move {
    //     let conn_opts = paho_mqtt::ConnectOptionsBuilder::new()
    //         .ssl_options(ssl_opts)
    //         .keep_alive_interval(Duration::from_secs(20))
    //         .automatic_reconnect(Duration::from_secs(20), Duration::from_secs(60))
    //         .user_name("test")
    //         .finalize();

    //     // Get message stream before connecting.
    //     let strm = client.get_stream(25);

    //     // Connect and wait for it to complete or fail
    //     println!("Connecting to MQTT broker.");
    //     client.connect(conn_opts).await;

    //     println!("Subscribing to topics: {:?}", TOPICS);
    //     client.subscribe_many(TOPICS, QOS).await;

    //     while let Some(msg_opt) = strm.next().await {
    //         if let Some(msg) = msg_opt {
    //             info!("Got mqtt message. {}", msg);
    //         }
    //     }
    // });

    // Make clone
    while let Some(event) = reciever.next().await {
        if client.is_connected() {
            info!("connected to client..");
        } else {
            error!("client not connected..");
            continue;
        }

        // Only process NewOtaPackage eventss
        match event {
            Event::NewOtaPackage { uid, package } => {
                info!("mqtt_run: Event::NewOtaPackage");

                // Serialize this buddy
                let res = serde_cbor::ser::to_vec_packed(&package).unwrap();

                // Generate topic
                let sub_topic = format!("{}/ota/sub", uid);

                // Create a new message
                let msg = paho_mqtt::Message::new(&sub_topic, res, paho_mqtt::QOS_1);

                info!("Publishing message to {}", &sub_topic);

                // Publish to the UID in question
                // if let Err(e) = client.publish(msg).await {
                //     error!("Unable to publish to {}. Error: {}", sub_topic, e);
                // } else {
                //     info!("Published..");
                // }
            }
            _ => (),
        };
    }

    // TODO: join the above tasks
    // mqtt_recieve_task.join(mqtt_send_task);
}

// Only requires a sender. No response necessary here... yet.
async fn sock_run(broker_sender: Sender<Event>) {
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
            sender,
        })
        .await;

    debug!("Removing previous sock!");

    // Remove previous socket
    let _ = std::fs::remove_file(&socket_path);

    // Make connection
    let listener = UnixListener::bind(&socket_path)
        .await
        .expect("Unable to bind!");
    let mut incoming = listener.incoming();

    debug!("Created socket listener!");

    while let Some(stream) = incoming.next().await {
        debug!("Got stream!");

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

        // Send result back to broker
        broker_sender
            .send(Event::NewOtaPackage {
                uid: res.uid,
                package: res.package,
            })
            .await;
    }
}

async fn broker_run(mut broker_reciever: Receiver<Event>) {
    let mut runners: HashMap<String, Sender<Event>> = HashMap::new();

    // Handle broker events
    while let Some(event) = broker_reciever.next().await {
        match event {
            // Upon creating a new server thread, the thread has to register with the broker.
            Event::NewRunner { name, sender } => {
                // Check to see if the runner is already in the HashMap
                match runners.entry(name.clone()) {
                    Entry::Occupied(..) => (),
                    Entry::Vacant(entry) => {
                        // Inserts the Sender<event> into the HashMap
                        info!("Adding {} to broker.", name);
                        entry.insert(sender);
                    }
                }
            }
            // Handle NewOtaPackage generated by sock_run
            Event::NewOtaPackage { uid, package } => {
                info!("broker_run: Event::NewOtaPackage");

                // Send to sled
                runners
                    .get("sled")
                    .unwrap()
                    .send(Event::NewOtaPackage {
                        uid: uid.clone(),
                        package: package.clone(),
                    })
                    .await;

                // Send to mqtt
                runners
                    .get("mqtt")
                    .unwrap()
                    .send(Event::NewOtaPackage {
                        uid: uid.clone(),
                        package: package.clone(),
                    })
                    .await;
            }
            _ => (),
        }
    }
}

fn main() {
    // Initialize the logger from the environment
    env_logger::init();

    // Parse .env file
    dotenv::dotenv().ok();

    // Channels for communication
    let (broker_sender, broker_reciever) = channel::<pyrinas_shared::Event>(100);

    // TODO: init influx connection

    // TODO: init http service

    // Start sled task
    let _sled_task = task::spawn(sled_run(broker_sender.clone()));

    // Start unix socket task
    let _unix_sock_task = task::spawn(sock_run(broker_sender.clone()));

    // Spawn a new task for the MQTT stuff
    let _mqtt_task = task::spawn(mqtt_run(broker_sender.clone()));

    // Spawn the broker task that handles it all!
    let _broker_task = task::block_on(broker_run(broker_reciever));

    info!("Done!");
}
