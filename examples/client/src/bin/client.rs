use pyrinas_codec_example::EnvironmentData;
use rumqttc::{self, AsyncClient, MqttOptions, QoS};
use std::time::Duration;

// Command line parsing
use clap::Parser;

#[derive(Parser)]
#[clap(version)]
struct Opts {
    address: String,
    port: u16,
}

#[tokio::main()]
async fn main() {
    // Get the config path
    let opts: Opts = Opts::parse();

    // Set up client
    let mut mqttoptions = MqttOptions::new("pyrinas-client-example", opts.address, opts.port);
    mqttoptions.set_keep_alive(Duration::from_secs(5));

    // Connect
    let (client, mut eventloop) = AsyncClient::new(mqttoptions, 10);

    // Data payload
    let raw_data = EnvironmentData {
        temperature: 4000,
        humidity: 3200,
    };

    // Serialize and send CBOR data
    let data = serde_cbor::to_vec(&raw_data).unwrap();

    // Handle requests
    tokio::spawn(async move {
        loop {
            client
                .publish("1234/app/p/env", QoS::AtLeastOnce, false, data.clone())
                .await
                .unwrap();
            tokio::time::sleep(Duration::from_secs(3)).await;
        }
    });

    // Iterate to poll the eventloop for connection progress
    loop {
        let _ = eventloop.poll().await;
    }
}
