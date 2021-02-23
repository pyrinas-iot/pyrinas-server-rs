// Sytem related
use log::{debug, error, warn};
use std::fs::File;
use std::io::Read;
use std::{env, process};

// async related
use std::sync::Arc;
use flume::{unbounded, Sender};

// Shared
use pyrinas_shared::{settings::PyrinasSettings, Event};

// MQTT related
use async_channel;
use rumqttc::{
    self,
    Event::{Incoming, Outgoing},
    EventLoop, MqttOptions, Packet, Publish, QoS, Request, Subscribe,
};

// Master subscription list for debug
#[cfg(debug_assertions)]
const SUBSCRIBE: [&str; 3] = ["d/+/ota/pub", "d/+/tel/pub", "d/+/app/pub/#"];

// Master subscription list for release
#[cfg(not(debug_assertions))]
const SUBSCRIBE: [&str; 3] = ["+/ota/pub", "+/tel/pub", "+/app/pub/#"];

pub async fn setup(settings: Arc<PyrinasSettings>) -> EventLoop {
    // We assume that we are in a valid directory.
    let mut ca_cert = env::current_dir().unwrap();
    ca_cert.push(settings.mqtt.ca_cert.clone());

    let mut server_cert = env::current_dir().unwrap();
    server_cert.push(settings.mqtt.server_cert.clone());

    let mut private_key = env::current_dir().unwrap();
    private_key.push(settings.mqtt.private_key.clone());

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
    let mut opt = MqttOptions::new(
        settings.mqtt.id.clone(),
        settings.mqtt.host.clone(),
        settings.mqtt.port.clone(),
    );
    opt.set_keep_alive(120);
    // TODO: add these back when things are working again...
    // opt.set_ca(ca_cert_buf);
    // opt.set_client_auth(server_cert_buf, private_key_buf);

    EventLoop::new(opt, 10)
}

pub async fn mqtt_run(eventloop: &mut EventLoop, broker_sender: Sender<Event>) {
    // Loop for recieving messages
    loop {
        if let Ok(incoming) = eventloop.poll().await {
            // If we have an actual message
            let msg = match incoming {
                Incoming(i) => i,
                Outgoing(_) => continue,
            };

            // Sort it
            match msg {
                // Incoming::Publish is the main thing we're concerned with here..
                Packet::Publish(msg) => {
                    debug!("Publish = {:?}", msg);

                    // Get the uid and topic
                    let mut topic = msg.topic.trim().split('/');

                    // Removing the d/ prefix
                    #[cfg(debug_assertions)]
                    topic.next();

                    let uid = topic.next().unwrap_or_default();
                    let event_type = topic.next().unwrap_or_default();
                    let pub_sub = topic.next().unwrap_or_default();

                    let targets: Vec<&str> = msg.topic.trim().split(pub_sub).collect();
                    let mut target: &str = "";
                    if let Some(t) = targets.last() {
                        target = t.trim_end_matches('/').trim_start_matches('/');
                    }

                    // Continue if not euql to pub
                    if pub_sub != "pub" {
                        warn!("Pubsub not 'pub'. Value: {}", pub_sub);
                        continue;
                    }

                    match event_type {
                        "ota" => {
                            // Get the telemetry data
                            let res: Result<pyrinas_shared::OtaRequest, serde_cbor::error::Error>;

                            // Get the result
                            res = serde_cbor::from_slice(msg.payload.as_ref());

                            // Match function to handle error
                            match res {
                                Ok(n) => {
                                    debug!("{:?}", n);

                                    // Send message to broker
                                    broker_sender
                                        .send_async(Event::OtaRequest {
                                            uid: uid.to_string(),
                                            msg: n,
                                        })
                                        .await
                                        .unwrap();
                                }
                                Err(e) => error!("OTA decode error: {}", e),
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
                                    debug!("{:?}", n);

                                    // Create query
                                    let query = n
                                        .to_influx_data(uid.to_string())
                                        .to_influx_query("telemetry".to_string());

                                    // Send data to broker
                                    broker_sender
                                        .send_async(Event::InfluxDataSave(query))
                                        .await
                                        .unwrap();
                                }
                                Err(e) => error!("Telemetry decode error: {}", e),
                            }
                        }
                        "app" => {
                            debug!("app: from:{:?}", uid.to_string());

                            // Send data to broker
                            broker_sender
                                .send_async(Event::ApplicationRequest(pyrinas_shared::ApplicationData {
                                    uid: uid.to_string(),
                                    target: target.to_string(),
                                    msg: msg.payload.to_vec(),
                                }))
                                .await
                                .unwrap();
                        }
                        _ => {}
                    }
                }
                _ => {}
            };
        };
    }
}

pub async fn run(tx: &mut async_channel::Sender<Request>, broker_sender: Sender<Event>) {
    // Get the sender/reciever associated with this particular task
    let (sender, reciever) = unbounded::<pyrinas_shared::Event>();

    // Register this task
    broker_sender
        .send_async(Event::NewRunner {
            name: "mqtt".to_string(),
            sender: sender.clone(),
        })
        .await
        .unwrap();

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

    while let Ok(event) = reciever.recv_async().await {
        // Only process OtaNewPackage eventss
        match event {
            Event::ApplicationResponse(data) => {
                debug!("Event::ApplicationResponse");

                // Generate topic (debug)
                #[cfg(debug_assertions)]
                let sub_topic = format!("d/{}/app/sub/{}", data.uid, data.target);

                // Generate topic
                #[cfg(not(debug_assertions))]
                let sub_topic = format!("{}/app/sub/{}", data.uid, data.target);

                // Create a new message
                let out = Publish::new(&sub_topic, QoS::AtLeastOnce, data.msg);

                debug!("Publishing application message to {}", &sub_topic);

                // Publish to the UID in question
                // TODO: wrap this guy up in a separate spawn so it can get back to work.
                if let Err(e) = tx.send(Request::Publish(out)).await {
                    error!("Unable to publish to {}. Error: {}", sub_topic, e);
                } else {
                    debug!("Published..");
                }
            }
            Event::OtaResponse(update) => {
                debug!("mqtt_run: Event::OtaResponse");

                // Serialize this buddy
                let res = serde_cbor::ser::to_vec_packed(&update.package).unwrap();

                // Generate topic (debug)
                #[cfg(debug_assertions)]
                let sub_topic = format!("d/{}/ota/sub", update.uid);

                // Generate topic
                #[cfg(not(debug_assertions))]
                let sub_topic = format!("{}/ota/sub", update.uid);

                // Create a new message
                let msg = Publish::new(&sub_topic, QoS::AtLeastOnce, res);

                debug!("Publishing message to {}", &sub_topic);

                // Publish to the UID in question
                if let Err(e) = tx.send(Request::Publish(msg)).await {
                    error!("Unable to publish to {}. Error: {}", sub_topic, e);
                } else {
                    debug!("Published..");
                }
            }
            _ => (),
        };
    }
}
