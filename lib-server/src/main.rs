use dotenv;
use log::{debug, error, info};
use pyrinas_shared::Event;
use rumqttc::{self, EventLoop, MqttOptions};
use std::fs::File;
use std::{
    collections::hash_map::{Entry, HashMap},
    env,
    io::Read,
    process,
};
use tokio::net::UnixListener;
use tokio::stream::StreamExt;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::task;

async fn sled_run(mut broker_sender: Sender<Event>) {
    // Get the sender/reciever associated with this particular task
    let (sender, mut reciever) = channel::<pyrinas_shared::Event>(20);

    // Register this task
    broker_sender
        .send(Event::NewRunner {
            name: "sled".to_string(),
            sender: sender.clone(),
        })
        .await
        .unwrap();

    let sled_db = dotenv::var("PYRINAS_SLED_DB").unwrap_or_else(|_| {
        error!("PYRINAS_SLED_DB must be set in environment!");
        process::exit(1);
    });

    // Open the DB
    let tree = sled::open(sled_db).expect("Error opening sled db.");

    // TODO: wait for event on reciever
    while let Some(event) = reciever.recv().await {
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

async fn mqtt_run(mut broker_sender: Sender<Event>) {
    let ca_cert_env = dotenv::var("PYRINAS_CA_CERT").unwrap_or_else(|_| {
        error!("PYRINAS_CA_CERT must be set in environment!");
        process::exit(1);
    });

    let server_cert_env = dotenv::var("PYRINAS_SERVER_CERT").unwrap_or_else(|_| {
        error!("PYRINAS_SERVER_CERT must be set in environment!");
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

    // Port
    let port = dotenv::var("PYRINAS_HOST_PORT").unwrap_or_else(|_| {
        error!("PYRINAS_HOST_PORT must be set in environment!");
        process::exit(1);
    });

    // Get the sender/reciever associated with this particular task
    let (sender, mut reciever) = channel::<pyrinas_shared::Event>(20);

    // Register this task
    broker_sender
        .send(Event::NewRunner {
            name: "mqtt".to_string(),
            sender: sender.clone(),
        })
        .await
        .unwrap();

    // We assume that we are in a valid directory.
    let mut ca_cert = env::current_dir().unwrap();
    ca_cert.push(ca_cert_env);

    let mut server_cert = env::current_dir().unwrap();
    server_cert.push(server_cert_env);

    let mut private_key = env::current_dir().unwrap();
    private_key.push(private_key_env);

    if !ca_cert.exists() {
        error!("The trust store file does not exist: {:?}", ca_cert);
        process::exit(1);
    }

    if !server_cert.exists() {
        error!("The key store file does not exist: {:?}", server_cert);
        process::exit(1);
    }

    if !private_key.exists() {
        error!("The key store file does not exist: {:?}", private_key);
        process::exit(1);
    }

    // Read the ca_cert
    let mut file = File::open(ca_cert).expect("Unable to open file!");
    let mut ca_cert_buf = Vec::new();
    file.read_to_end(&mut ca_cert_buf)
        .expect("Unable to read to end");

    // Read the server_cert
    let mut file = File::open(server_cert).expect("Unable to open file!");
    let mut server_cert_buf = Vec::new();
    file.read_to_end(&mut server_cert_buf)
        .expect("Unable to read to end");

    // Read the private_key
    let mut file = File::open(private_key).expect("Unable to open file!");
    let mut private_key_buf = Vec::new();
    file.read_to_end(&mut private_key_buf)
        .expect("Unable to read to end");

    // Create the options for the Mqtt client
    let mut opt = MqttOptions::new("server", host, port.parse::<u16>().unwrap());
    opt.set_keep_alive(5);
    opt.set_ca(ca_cert_buf);
    opt.set_client_auth(server_cert_buf, private_key_buf);

    let mut eventloop = EventLoop::new(opt, 10).await;

    loop {
        let (incoming, outgoing) = eventloop.poll().await.unwrap();
        println!("Incoming = {:?}, Outgoing = {:?}", incoming, outgoing);
    }

    // while let Some(event) = reciever.recv().await {
    //     // Only process NewOtaPackage eventss
    //     match event {
    //         Event::NewOtaPackage { uid, package } => {
    //             info!("mqtt_run: Event::NewOtaPackage");

    //             // Serialize this buddy
    //             let _res = serde_cbor::ser::to_vec_packed(&package).unwrap();

    //             // Generate topic
    //             let sub_topic = format!("{}/ota/sub", uid);

    //             // Create a new message
    //             // let msg = paho_mqtt::Message::new(&sub_topic, res, paho_mqtt::QOS_1);

    //             info!("Publishing message to {}", &sub_topic);

    //             // Publish to the UID in question
    //             // if let Err(e) = client.publish(msg).await {
    //             //     error!("Unable to publish to {}. Error: {}", sub_topic, e);
    //             // } else {
    //             //     info!("Published..");
    //             // }
    //         }
    //         _ => (),
    //     };
    // }

    // TODO: join the above tasks
    // mqtt_recieve_task.join(mqtt_send_task);
}

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

    while let Some(_stream) = incoming.next().await {
        debug!("Got stream!");

        // Setup work to make this happen
        // let mut stream = stream.unwrap(); //Box::<UnixStream>::new(stream.unwrap());
        // let mut buffer = Vec::new();

        // Read until the stream closes
        // stream
        //     .read_to_end(&mut buffer)
        //     .await
        //     .expect("Unable to write to socket.");

        // // Get string from the buffer
        // let s = str::from_utf8(&buffer).unwrap();
        // info!("{}", s);

        // // Decode into struct
        // let res: pyrinas_shared::NewOta = serde_json::from_str(&s).unwrap();

        // // Send result back to broker
        // broker_sender
        //     .send(Event::NewOtaPackage {
        //         uid: res.uid,
        //         package: res.package,
        //     })
        //     .await;
    }
}

async fn broker_run(mut broker_reciever: Receiver<Event>) {
    let mut runners: HashMap<String, Sender<Event>> = HashMap::new();

    // Handle broker events
    while let Some(event) = broker_reciever.recv().await {
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
                    .get_mut("sled")
                    .unwrap()
                    .send(Event::NewOtaPackage {
                        uid: uid.clone(),
                        package: package.clone(),
                    })
                    .await
                    .unwrap();

                // Send to mqtt
                runners
                    .get_mut("mqtt")
                    .unwrap()
                    .send(Event::NewOtaPackage {
                        uid: uid.clone(),
                        package: package.clone(),
                    })
                    .await
                    .unwrap();
            }
            _ => (),
        }
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

    // TODO: init influx connection

    // TODO: init http service

    // Start sled task
    let sled_task = task::spawn(sled_run(broker_sender.clone()));

    // Start unix socket task
    let unix_sock_task = task::spawn(sock_run(broker_sender.clone()));

    // Spawn a new task for the MQTT stuff
    let mqtt_task = task::spawn(mqtt_run(broker_sender.clone()));

    // Spawn the broker task that handles it all!
    let broker_task = task::spawn(broker_run(broker_reciever));

    // Join hands kids
    let _join = tokio::join!(sled_task, unix_sock_task, mqtt_task, broker_task);

    info!("Done!");
}
