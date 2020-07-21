use futures::{executor::block_on, stream::StreamExt};
use paho_mqtt as mqtt;
use std::{process, time::Duration};

const TOPICS: &[&str] = &["+/+/pub"];
const QOS: &[i32] = &[1];

/////////////////////////////////////////////////////////////////////////////

// Struct that can be used elsewhere
pub struct MQTTCloud {
  client: mqtt::AsyncClient,
  trust_store: String,
  key_store: String,
  private_key: String,
}

// Implement the functions related to the cloud
impl MQTTCloud {
  pub fn new(host: &str) -> Self {
    println!("Connecting to host: '{}'", host);

    // Create a client options
    let create_opts = mqtt::CreateOptionsBuilder::new()
      .server_uri(host)
      .client_id("ssl_publish_rs")
      .max_buffered_messages(100)
      .finalize();

    let client = mqtt::AsyncClient::new(create_opts).unwrap_or_else(|e| {
      println!("Error creating the client: {:?}", e);
      process::exit(1);
    });

    Self {
      client: client,
      key_store: "".to_string(),
      private_key: "".to_string(),
      trust_store: "".to_string(),
    }
  }

  pub fn set_certs(&mut self, trust_store: &str, private_key: &str, key_store: &str) {
    self.key_store = key_store.to_string();
    self.private_key = private_key.to_string();
    self.trust_store = trust_store.to_string();
  }

  fn assemble_ota_pkg_response(&self, uid: &str) -> Option<mqtt::Message> {
    // TODO: send serialized CBOR back
    let package = pyrinas_shared::OTAPackage {
      version: "0.1.0".to_string(),
      host: "dreamstars.s3.amazonaws.com".to_string(),
      file: "app_update.bin".to_string(),
      force: true,
    };

    // Serialize this buddy
    let res = serde_cbor::ser::to_vec_packed(&package);

    if res.is_ok() {
      let res = res.unwrap();

      println!("ota response serialized! size: {}", res.len());
      //TODO: Send back to the UUID in question using <UID>/ota/sub/
      let sub_topic = format!("{}/ota/sub", uid);

      // Send payload
      return Some(mqtt::Message::new(sub_topic, res, mqtt::QOS_1));
    }
    None
  }

  // Filters events and processes accordingly
  fn handle_events(&self, msg: &mqtt::message::Message) -> Option<mqtt::Message> {
    println!("{}", msg);

    // TODO: handle telemetry events
    if msg.topic().contains("/tel/pub") {
      // TODO: CBOR deserialize

      // Get the telemetry data
      let res: Result<pyrinas_shared::Telemetry, serde_cbor::error::Error> =
        serde_cbor::from_slice(msg.payload());

      match res {
        Ok(n) => println!("{:?}", n),
        Err(e) => println!("error: {}", e),
      }

    // TODO: serialize into InfluxDB string
    } else if msg.topic().contains("/app/pub") {
      // TODO: handle app events
      // TODO: CBOR deserialize
      // TODO: serialize into InfluxDB string

      println!("application");
    } else if msg.topic().contains("/ota/pub") {
      // TODO: handle OTA events

      // Get the payload
      let payload = msg.payload();

      // Make sure we've gotten only 1 byte
      if payload.len() == 1 {
        // Get the UID from the topic
        let uid = msg.topic().strip_suffix("/ota/pub").unwrap();
        println!("ota uid: {}", uid);

        // Get the command
        let ota_cmd = payload[0];

        // Match the output of the OTA cmd
        match ota_cmd {
          // Status check
          0 => {
            println!("Checking for OTA");
            // TODO: check if OTA is applicable for UID

            // TODO: if so create http endpoint (or do nothing if it already exists)

            // TODO: creat sled entry with UID for file download and UID of device

            // If we have an update, assbmle the package
            // return self.assemble_ota_pkg_response(&uid);
          }
          // Indicate done
          1 => {
            println!("Done with OTA");
            // TODO: mark complete in sled against UID

            // TODO: remove HTTP endpoint
          }
          // All remaining, do nothing
          _ => (),
        }
      }
    }
    None
  }

  // TODO: remove this
  #[allow(dead_code)]
  pub fn publish() {}

  #[allow(dead_code)]
  pub fn disconnect() {}

  pub fn start(&mut self) {
    let ssl_opts = mqtt::SslOptionsBuilder::new()
      .trust_store(self.trust_store.clone())
      .unwrap()
      .key_store(self.key_store.clone())
      .unwrap()
      .private_key(self.private_key.clone())
      .unwrap()
      .finalize();

    let conn_opts = mqtt::ConnectOptionsBuilder::new()
      .ssl_options(ssl_opts)
      .keep_alive_interval(Duration::from_secs(20))
      .automatic_reconnect(Duration::from_secs(20), Duration::from_secs(60))
      .user_name("test")
      .finalize();

    if let Err(err) = block_on(async {
      // Get message stream before connecting.
      let mut strm = self.client.get_stream(25);

      // Connect and wait for it to complete or fail
      println!("Connecting to MQTT broker.");
      self.client.connect(conn_opts).await?;

      println!("Subscribing to topics: {:?}", TOPICS);
      self.client.subscribe_many(TOPICS, QOS).await?;

      // let uid = "352656102545228";
      // if let Some(msg) = self.assemble_ota_pkg_response(&uid) {
      //   println!("publishing ota msg {}", msg);
      //   self.client.publish(msg).await?;
      // }

      while let Some(msg_opt) = strm.next().await {
        if let Some(msg) = msg_opt {
          // If it returns a message, publish it
          if let Some(resp) = self.handle_events(&msg) {
            println!("Sending message");
            self.client.publish(resp).await?;
          }
        }
      }

      // Explicit return type for the async block
      Ok::<(), mqtt::Error>(())
    }) {
      eprintln!("{}", err);
    }

    // let msg = mqtt::MessageBuilder::new()
    //     .topic("352656102545228/ota/sub")
    //     .payload("\0")
    //     .qos(1)
    //     .finalize();

    // let tok = cli.publish(msg);
    // if let Err(e) = tok.wait() {
    //     println!("Error sending message: {:?}", e);
    // }

    // let tok = cli.disconnect(None);
    // let _ = tok.wait();
  }
}
