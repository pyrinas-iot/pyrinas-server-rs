use influxdb::InfluxDbWriteable;
// Pyrinas related
use pyrinas_server::{settings::PyrinasSettings, Event};

// async Related
use flume::{unbounded, Sender};
use std::sync::Arc;

pub async fn run(_settings: Arc<PyrinasSettings>, broker_sender: Sender<Event>) {
    // Get the sender/reciever associated with this particular task
    let (sender, reciever) = unbounded::<Event>();

    // Register this task
    broker_sender
        .send_async(Event::NewRunner {
            name: "app".to_string(),
            sender: sender.clone(),
        })
        .await
        .unwrap();

    // Wait for event on reciever
    while let Ok(event) = reciever.recv_async().await {
        log::debug!("{:?}", event);

        // Match the event. The only one we're interested in is the `ApplicationRequest`
        if let Event::ApplicationRequest(req) = event {
            log::info!("target: {}", req.target);

            match req.target.as_str() {
                // Handle a certain event
                "gps" => {
                    // Handle deserialization of CBOR
                    let payload: crate::structures::data::TrackerGpsReport =
                        match serde_cbor::from_slice(&req.msg) {
                            Ok(p) => p,
                            Err(e) => {
                                log::error!("Unable to deserialize data! Err: {}", e);
                                continue;
                            }
                        };

                    log::info!("gps data: {:?}", payload);

                    // Pubish GPS to Influx
                    let query = payload.to_influx(&req.uid).into_query("gps");

                    // Send the query
                    if let Err(e) = broker_sender
                        .send_async(pyrinas_server::Event::InfluxDataSave(query))
                        .await
                    {
                        log::error!("Unable to publish query to Influx! {:?}", e);
                    }
                }
                "boot" => {
                    // Handle deserialization of CBOR
                    let payload: crate::structures::data::TrackerDeviceReport =
                        match serde_cbor::from_slice(&req.msg) {
                            Ok(p) => p,
                            Err(e) => {
                                log::error!("Unable to deserialize data! Err: {}", e);
                                continue;
                            }
                        };

                    log::info!("boot data: {:?}", payload);

                    // Pubish boot data to influx
                    let query = payload.to_influx(&req.uid).into_query("boot");

                    // Send the query
                    if let Err(e) = broker_sender
                        .send_async(pyrinas_server::Event::InfluxDataSave(query))
                        .await
                    {
                        log::error!("Unable to publish query to Influx! {:?}", e);
                    }
                }
                "motion" => {
                    // Handle deserialization of CBOR
                    let payload: crate::structures::data::TrackerAccelReport =
                        match serde_cbor::from_slice(&req.msg) {
                            Ok(p) => p,
                            Err(e) => {
                                log::error!("Unable to deserialize data! Err: {}", e);
                                continue;
                            }
                        };

                    log::info!("accel data: {:?}", payload);

                    // Pubish GPS to Influx
                    let query = payload.to_influx(&req.uid).into_query("accel");

                    // Send the query
                    if let Err(e) = broker_sender
                        .send_async(pyrinas_server::Event::InfluxDataSave(query))
                        .await
                    {
                        log::error!("Unable to publish query to Influx! {:?}", e);
                    }
                }
                _ => {}
            };
        }
    }
}
