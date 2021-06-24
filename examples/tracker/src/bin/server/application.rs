use influxdb::InfluxDbWriteable;
// Pyrinas related
use pyrinas_server::{settings::PyrinasSettings, Event};

// async Related
use flume::{unbounded, Sender};
// use std::str;
use std::str;
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
            // let payload = match str::from_utf8(&req.msg) {
            //     Ok(p) => p,
            //     Err(_e) => {
            //         log::error!("Unable to get json string!");
            //         continue;
            //     }
            // };

            log::info!("target: {}", req.target);

            match req.target.as_str() {
                // Handle a certain event
                "data" => {
                    // Handle deserialization of JSON from Asset Tracker V2
                    let payload: crate::structures::data::TrackerPayload =
                        match serde_json::from_slice(&req.msg) {
                            Ok(p) => p,
                            Err(e) => {
                                log::error!("Unable to deserialize data! Err: {}", e);
                                continue;
                            }
                        };

                    // Convert to TrackerStateReport
                    let payload = payload.state.reported;

                    log::info!("deserialized data: {:?}", payload);

                    // Pubish GPS to Influx
                    if let Some(r) = payload.gps {
                        let query = r.to_influx(&req.uid).into_query("gps");

                        // Send the query
                        if let Err(e) = broker_sender
                            .send_async(pyrinas_server::Event::InfluxDataSave(query))
                            .await
                        {
                            log::error!("Unable to publish query to Influx! {:?}", e);
                        }
                    };

                    // Publish Cellular Data to Influx
                    if let Some(r) = payload.roam {
                        let query = r.to_influx(&req.uid).into_query("roam");

                        // Send the query
                        if let Err(e) = broker_sender
                            .send_async(pyrinas_server::Event::InfluxDataSave(query))
                            .await
                        {
                            log::error!("Unable to publish query to Influx! {:?}", e);
                        }
                    };

                    // Publish Battery Data to Influx
                    if let Some(r) = payload.bat {
                        let query = r.to_influx(&req.uid).into_query("batt");

                        // Send the query
                        if let Err(e) = broker_sender
                            .send_async(pyrinas_server::Event::InfluxDataSave(query))
                            .await
                        {
                            log::error!("Unable to publish query to Influx! {:?}", e);
                        }
                    };
                }
                /* TODO: decode other entries besides accel */
                "batch" => match str::from_utf8(&req.msg) {
                    // TrackerBulkReport
                    Ok(m) => {
                        log::info!("batch: {}", m);

                        // Handle deserialization of JSON from Asset Tracker V2
                        let payload: crate::structures::data::TrackerBulkReport =
                            match serde_json::from_slice(&req.msg) {
                                Ok(p) => p,
                                Err(e) => {
                                    log::error!("Unable to deserialize data! Err: {}", e);
                                    continue;
                                }
                            };

                        // Submit each
                        for val in payload.acc {
                            let query = val.to_influx(&req.uid).into_query("accel");

                            // Send the query
                            if let Err(e) = broker_sender
                                .send_async(pyrinas_server::Event::InfluxDataSave(query))
                                .await
                            {
                                log::error!("Unable to publish query to Influx! {:?}", e);
                            }
                        }
                    }
                    Err(e) => log::error!("Error: {}", e),
                },
                "env" => {
                    // TODO: handle deserialization of JSON from Asset Tracker V2
                }
                _ => {}
            };
        }
    }
}
