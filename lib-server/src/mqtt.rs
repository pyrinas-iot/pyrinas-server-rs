// Sytem related
use log::{debug, error, info, warn};
use std::fs::File;
use std::io::Read;
use std::{env, process};

// Tokio async related
use tokio::sync::mpsc::{channel, Sender};
use tokio::task;

// Config related
use pyrinas_shared::settings::Settings;

// MQTT related
use rumqttc::{self, EventLoop, Incoming, MqttOptions, Publish, QoS, Request, Subscribe};

// Local lib related
use pyrinas_shared::Event;

// Master subscription list
const SUBSCRIBE: [&str; 3] = ["+/ota/pub", "+/tel/pub", "+/app/pub/+"];

pub async fn run(settings: Settings, mut broker_sender: Sender<Event>) {
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
  file
    .read_to_end(&mut ca_cert_buf)
    .expect("Unable to read to end");

  // Read the server_cert
  let mut file = File::open(server_cert).expect("Unable to open file!");
  let mut server_cert_buf = Vec::new();
  file
    .read_to_end(&mut server_cert_buf)
    .expect("Unable to read to end");

  // Read the private_key
  let mut file = File::open(private_key).expect("Unable to open file!");
  let mut private_key_buf = Vec::new();
  file
    .read_to_end(&mut private_key_buf)
    .expect("Unable to read to end");

  // Create the options for the Mqtt client
  let mut opt = MqttOptions::new(
    "server",
    settings.mqtt.host.clone(),
    settings.mqtt.port.parse::<u16>().unwrap(),
  );
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
        Event::ApplicationResponse { uid, target, msg } => {
          info!("mqtt::run Event::ApplicationResponse");

          // Generate topic
          let sub_topic = format!("{}/app/sub/{}", uid, target);

          // Create a new message
          let out = Publish::new(&sub_topic, QoS::AtLeastOnce, msg);

          info!("Publishing application message to {}", &sub_topic);

          // Publish to the UID in question
          // TODO: wrap this guy up in a separate spawn so it can get back to work.
          if let Err(e) = tx.send(Request::Publish(out)).await {
            error!("Unable to publish to {}. Error: {}", sub_topic, e);
          } else {
            info!("Published..");
          }
        }
        Event::OtaResponse(update) => {
          info!("mqtt_run: Event::OtaResponse");

          // Serialize this buddy
          let res = serde_cbor::ser::to_vec_packed(&update.package).unwrap();

          // Generate topic
          let sub_topic = format!("{}/ota/sub", update.uid);

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
            debug!("Publish = {:?}", msg);

            // Get the uid and topic
            let mut topic = msg.topic.split('/');
            let uid = topic.next().unwrap_or_default();
            let event_type = topic.next().unwrap_or_default();
            let pub_sub = topic.next().unwrap_or_default();
            let target = topic.next().unwrap_or_default();

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
                    info!("{:?}", n);

                    // Send message to broker
                    broker_sender
                      .send(Event::OtaRequest {
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
                let res: Result<pyrinas_shared::TelemetryData, serde_cbor::error::Error>;

                // Get the result
                res = serde_cbor::from_slice(msg.payload.as_ref());

                // Match function to handle error
                match res {
                  Ok(n) => {
                    info!("{:?}", n);

                    // Create query
                    let query = n
                      .to_influx_data(uid.to_string())
                      .to_influx_query("telemetry".to_string());

                    // Send data to broker
                    broker_sender
                      .send(Event::InfluxDataSave(query))
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
                  .send(Event::ApplicationRequest {
                    uid: uid.to_string(),
                    target: target.to_string(),
                    msg: msg.payload.to_vec(),
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
