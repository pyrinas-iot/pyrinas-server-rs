// Sytem related
use log::{debug, error, warn};

// async related
use flume::{unbounded, Sender};
use pyrinas_shared::OtaUpdate;

// Shared
use crate::telemetry;
use crate::Event;

// Mqttd
use librumqttd::async_locallink::{AsyncLinkRx, AsyncLinkTx};

pub async fn mqtt_run(rx: &mut AsyncLinkRx, broker_sender: Sender<Event>) {
    // Loop for recieving messages
    loop {
        let msg = match rx.recv().await {
            Ok(m) => m,
            Err(e) => {
                log::warn!("mqtt error: {}", e);
                continue;
            }
        };

        // Get the uid and topic
        let mut topic = msg.topic.split('/');

        let device_id = match topic.next() {
            Some(t) => t,
            None => {
                log::warn!("Unable get uid.");
                continue;
            }
        };

        let event_type = match topic.next() {
            Some(t) => t,
            None => {
                log::warn!("Unable event type.");
                continue;
            }
        };

        let pub_sub = match topic.next() {
            Some(t) => t,
            None => {
                log::warn!("Unable to get pub/sub.");
                continue;
            }
        };

        // Get the topic string after pub or sub
        let targets: Vec<&str> = msg.topic.trim().split(pub_sub).collect();
        let mut target: &str = "";
        if let Some(t) = targets.last() {
            target = t.trim_end_matches('/').trim_start_matches('/');
        }

        // Continue if pub. 'sub' are sent to clients
        if pub_sub != "pub" {
            warn!("Pubsub not 'pub'. Value: {}", pub_sub);
            continue;
        }

        // Go over each payload
        for payload in msg.payload {
            match event_type {
                "ota" => {
                    // Get the telemetry data
                    let res: Result<pyrinas_shared::OtaRequest, serde_cbor::error::Error>;

                    // Get the result
                    res = serde_cbor::from_slice(&payload);

                    // Match function to handle error
                    match res {
                        Ok(n) => {
                            debug!("{:?}", n);

                            // Send message to broker
                            broker_sender
                                .send_async(Event::OtaRequest {
                                    device_id: device_id.to_string(),
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
                    let res: Result<telemetry::TelemetryData, serde_cbor::error::Error>;

                    // Get the result
                    res = serde_cbor::from_slice(&payload);

                    // Match function to handle error
                    match res {
                        Ok(n) => {
                            debug!("{:?}", n);

                            // Create query
                            let query = n
                                .to_influx_data(device_id.to_string())
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
                    debug!("app: from:{:?}", device_id.to_string());

                    // Send data to broker
                    broker_sender
                        .send_async(Event::ApplicationRequest(pyrinas_shared::ApplicationData {
                            uid: device_id.to_string(),
                            target: target.to_string(),
                            msg: payload.to_vec(),
                        }))
                        .await
                        .unwrap();
                }
                _ => {}
            }
        }
    }
}

pub async fn run(tx: &mut AsyncLinkTx, broker_sender: Sender<Event>) {
    // Get the sender/reciever associated with this particular task
    let (sender, reciever) = unbounded::<Event>();

    // Register this task
    broker_sender
        .send_async(Event::NewRunner {
            name: "mqtt".to_string(),
            sender: sender.clone(),
        })
        .await
        .unwrap();

    while let Ok(event) = reciever.recv_async().await {
        // Only process OtaNewPackage eventss
        match event {
            Event::ApplicationResponse(data) => {
                debug!("Event::ApplicationResponse");

                // Generate topic
                // TODO: make these configurable
                let sub_topic = format!("{}/app/sub/{}", data.uid, data.target);

                // Publish to the UID in question
                // TODO: wrap this guy up in a separate spawn so it can get back to work.
                if let Err(e) = tx.publish(&sub_topic, false, data.msg).await {
                    error!("Unable to publish to {}. Error: {}", sub_topic, e);
                } else {
                    debug!("Published to {}", sub_topic);
                }
            }
            Event::OtaResponse(update) => {
                debug!("mqtt_run: Event::OtaResponse");

                // Device id
                let device_id = match update.uid {
                    Some(id) => id,
                    None => {
                        log::warn!("No device id!");
                        continue;
                    }
                };

                // Get the package
                let package = match update.package {
                    Some(p) => p,
                    None => {
                        log::warn!("No package!");
                        continue;
                    }
                };

                // Serialize this buddy
                let res = serde_cbor::ser::to_vec_packed(&package).unwrap();

                // Generate topic
                // TODO: make these configurable
                let sub_topic = format!("{}/ota/sub", device_id);

                debug!("Publishing message to {}", &sub_topic);

                // Publish to the UID in question
                if let Err(e) = tx.publish(&sub_topic, false, res).await {
                    error!("Unable to publish to {}. Error: {}", sub_topic, e);
                } else {
                    debug!("Published to {}", sub_topic);
                }
            }
            _ => (),
        };
    }
}
