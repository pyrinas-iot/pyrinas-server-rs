use dotenv;
use log::{debug, error, info, warn};
use std::fs::File;
use std::{
    collections::hash_map::{Entry, HashMap},
    env,
    io::Read,
    process, str,
};

// Tokio Related
use tokio::io::AsyncReadExt;
use tokio::net::UnixListener;
use tokio::stream::StreamExt;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::task;
use tokio::time::{delay_for, Duration};

// Influx Related
use influxdb::{Client, InfluxDbWriteable};

// MQTT related
use rumqttc::{self, EventLoop, Incoming, MqttOptions, Publish, QoS, Request, Subscribe};

// Local lib related
use pyrinas_shared::{Event, OtaRequestCmd};

// Master subscription list
const SUBSCRIBE: [&str; 3] = ["+/ota/pub", "+/tel/pub", "+/app/pub"];

async fn sled_run(mut broker_sender: Sender<Event>) {
    // Get the sender/reciever associated with this particular task
    let (mut sender, mut reciever) = channel::<pyrinas_shared::Event>(20);

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

    // TODO: smarter way to do this?
    tokio::spawn(async move {
        loop {
            // Delay
            delay_for(Duration::from_secs(10)).await;

            // Flush the database
            sender.send(Event::SledFlush).await.unwrap();
        }
    });

    // TODO: wait for event on reciever
    while let Some(event) = reciever.recv().await {
        match event {
            Event::SledFlush => {
                debug!("sled_run: Event::SledFlush");

                // Save it to disk
                if let Err(e) = tree.flush_async().await {
                    error!("Unable to flush tree. Error: {}", e);
                }
            }
            // Process OtaRequests
            Event::OtaRequest { uid, msg } => {
                info!("sled_run: Event::OtaRequest");

                // Do something different depending on the situation
                match msg.cmd {
                    OtaRequestCmd::Done => {
                        info!("Done!");

                        // TODO: mark update as complete
                    }
                    OtaRequestCmd::Check => {
                        info!("Check!");

                        // TODO: check if there's a package available and ready
                    }
                }
            }
            // Pprocess OtaNewPackage events
            Event::OtaNewPackage { uid, package } => {
                info!("sled_run: Event::OtaNewPackage");

                // Turn entry.package into CBOR
                let res = serde_cbor::ser::to_vec_packed(&package);

                // Write into database
                match res {
                    Ok(cbor_data) => {
                        if let Err(e) = tree.insert(uid, cbor_data) {
                            error!("Unable to insert into sled. Error: {}", e);
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
    let host = dotenv::var("PYRINAS_MQTT_HOST").unwrap_or_else(|_| {
        error!("PYRINAS_MQTT_HOST must be set in environment!");
        process::exit(1);
    });

    // Port
    let port = dotenv::var("PYRINAS_MQTT_HOST_PORT").unwrap_or_else(|_| {
        error!("PYRINAS_MQTT_HOST_PORT must be set in environment!");
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
    opt.set_keep_alive(120);
    // TODO: add these back when things are working again...
    // opt.set_ca(ca_cert_buf);
    // opt.set_client_auth(server_cert_buf, private_key_buf);

    let mut eventloop = EventLoop::new(opt, 10).await;
    let tx = eventloop.handle();

    // Loop for sending messages from main broker
    let _ = task::spawn(async move {
        // TODO: handle cases were the sesion is not maintained..

        // Iterate though all potential subscriptions
        for item in SUBSCRIBE.iter() {
            // Set up subscription
            let subscription = Subscribe::new(*item, QoS::AtMostOnce);
            tx.send(Request::Subscribe(subscription))
                .await
                .unwrap_or_else(|e| {
                    println!("Unable to subscribe! Error: {}", e);
                    process::exit(1);
                });
        }

        while let Some(event) = reciever.recv().await {
            // Only process OtaNewPackage eventss
            match event {
                Event::OtaNewPackage { uid, package } => {
                    info!("mqtt_run: Event::OtaNewPackage");

                    // Serialize this buddy
                    let res = serde_cbor::ser::to_vec_packed(&package).unwrap();

                    // Generate topic
                    let sub_topic = format!("{}/ota/sub", uid);

                    // Create a new message
                    let msg = Publish::new(&sub_topic, QoS::AtLeastOnce, res);

                    info!("Publishing message to {}", &sub_topic);

                    // Publish to the UID in question
                    // TODO: wrap this guy up in a separate spawn so it can get back to work.
                    if let Err(e) = tx.send(Request::Publish(msg)).await {
                        error!("Unable to publish to {}. Error: {}", sub_topic, e);
                    } else {
                        info!("Published..");
                    }
                }
                _ => (),
            };
        }
    });

    // Loop for recieving messages
    loop {
        if let Ok((incoming, _)) = eventloop.poll().await {
            // If we have an actual message
            if incoming.is_some() {
                // Get the message
                let msg = incoming.unwrap();

                // Sort it
                match msg {
                    // Incoming::Publish is the main thing we're concerned with here..
                    Incoming::Publish(msg) => {
                        println!("Publish = {:?}", msg);

                        // Get the uid and topic
                        let mut topic = msg.topic.split('/');
                        let uid = topic.next().unwrap_or_default();
                        let event_type = topic.next().unwrap_or_default();
                        let pub_sub = topic.next().unwrap_or_default();

                        // Continue if not euql to pub
                        if pub_sub != "pub" {
                            warn!("Pubsub not 'pub'. Value: {}", pub_sub);
                            continue;
                        }

                        match event_type {
                            "ota" => {
                                // Get the telemetry data
                                let res: Result<
                                    pyrinas_shared::OtaRequest,
                                    serde_cbor::error::Error,
                                >;

                                // Get the result
                                res = serde_cbor::from_slice(msg.payload.as_ref());

                                // Match function to handle error
                                match res {
                                    Ok(n) => {
                                        println!("{:?}", n);

                                        // Send message to broker
                                        broker_sender
                                            .send(Event::OtaRequest {
                                                uid: uid.to_string(),
                                                msg: n,
                                            })
                                            .await
                                            .unwrap();
                                    }
                                    Err(e) => println!("Decode error: {}", e),
                                }
                            }
                            "tel" => {
                                // Get the telemetry data
                                let res: Result<
                                    pyrinas_shared::TelemetryData,
                                    serde_cbor::error::Error,
                                >;

                                // Get the result
                                res = serde_cbor::from_slice(msg.payload.as_ref());

                                // Match function to handle error
                                match res {
                                    Ok(n) => {
                                        println!("{:?}", n);
                                        // Send data to broker
                                        broker_sender
                                            .send(Event::TelemetryData {
                                                uid: uid.to_string(),
                                                msg: n,
                                            })
                                            .await
                                            .unwrap();
                                    }
                                    Err(e) => println!("Decode error: {}", e),
                                }
                            }
                            "app" => {
                                // TODO: Deserialize data?

                                // Send data to broker
                                broker_sender
                                    .send(Event::ApplicationData {
                                        uid: uid.to_string(),
                                        msg: msg.payload,
                                    })
                                    .await
                                    .unwrap();
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                };
            }
        };
    }
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

async fn broker_run(mut broker_reciever: Receiver<Event>) {
    let mut runners: HashMap<String, Sender<Event>> = HashMap::new();

    // Handle broker events
    while let Some(event) = broker_reciever.recv().await {
        match event.clone() {
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
            // Handle OtaNewPackage generated by sock_run
            Event::OtaNewPackage { uid: _, package: _ } => {
                info!("broker_run: Event::OtaNewPackage");

                // Send to sled
                runners
                    .get_mut("sled")
                    .unwrap()
                    .send(event.clone())
                    .await
                    .unwrap();

                // Send to mqtt
                runners
                    .get_mut("mqtt")
                    .unwrap()
                    .send(event.clone())
                    .await
                    .unwrap();
            }
            Event::OtaRequest { uid: _, msg: _ } => {
                info!("broker_run: OtaRequest");

                // Send to sled
                runners
                    .get_mut("sled")
                    .unwrap()
                    .send(event.clone())
                    .await
                    .unwrap();
            }
            Event::TelemetryData { uid: _, msg: _ } => {
                info!("broker_run: TelemetryData");

                // Send to influx
                runners
                    .get_mut("influx")
                    .unwrap()
                    .send(event.clone())
                    .await
                    .unwrap();
            }

            _ => (),
        }
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
    let sled_task = task::spawn(sled_run(broker_sender.clone()));

    // Start unix socket task
    let unix_sock_task = task::spawn(sock_run(broker_sender.clone()));

    // Spawn a new task for the MQTT stuff
    let mqtt_task = task::spawn(mqtt_run(broker_sender.clone()));

    // Spawn the broker task that handles it all!
    let broker_task = task::spawn(broker_run(broker_reciever));

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
