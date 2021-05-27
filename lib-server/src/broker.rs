// System related
use log::debug;
use std::collections::hash_map::{Entry, HashMap};

// Channels
use flume::{Receiver, Sender};

// Error
use anyhow::{anyhow, Result};

// Local lib related
use crate::Event;

pub async fn run(broker_reciever: Receiver<Event>) {
    let mut runners: HashMap<String, Sender<Event>> = HashMap::new();

    // Handle broker events
    while let Ok(event) = broker_reciever.recv_async().await {
        match event.clone() {
            // Upon creating a new server thread, the thread has to register with the broker.
            Event::NewRunner { name, sender } => {
                // Check to see if the runner is already in the HashMap
                match runners.entry(name.clone()) {
                    Entry::Occupied(..) => (),
                    Entry::Vacant(entry) => {
                        // Inserts the Sender<event> into the HashMap
                        debug!("Adding {} to broker.", name);
                        entry.insert(sender);
                    }
                }
            }
            Event::InfluxDataSave(_query) => {
                debug!("broker_run: InfluxDataSave");

                // Send to influx
                if let Err(e) = send("influx", &event, &mut runners).await {
                    log::error!("{}", e);
                }
            }
            Event::ApplicationRequest(_) | Event::ApplicationManagementRequest(_) => {
                debug!("broker_run: ApplicationManagementRequest");

                // Send to app handler
                if let Err(e) = send("app", &event, &mut runners).await {
                    log::error!("{}", e);
                }
            }
            Event::ApplicationManagementResponse(_data) => {
                debug!("broker_run: ApplicationManagementResponse");

                // Send to app handler
                if let Err(e) = send("sock", &event, &mut runners).await {
                    log::error!("{}", e);
                }
            }
            Event::ApplicationResponse(_) | Event::OtaResponse(_) => {
                debug!("broker_run: ApplicationResponse");
                // Send to mqtt handler
                if let Err(e) = send("mqtt", &event, &mut runners).await {
                    log::error!("{}", e);
                }
            }
            Event::OtaDissociate { .. }
            | Event::OtaAssociate { .. }
            | Event::OtaUpdateImageListRequest()
            | Event::OtaUpdateGroupListRequest()
            | Event::OtaDeletePackage(_)
            | Event::OtaNewPackage(_)
            | Event::OtaRequest { .. } => {
                // Send to ota task
                if let Err(e) = send("ota", &event, &mut runners).await {
                    log::error!("{}", e);
                }
            }
            Event::OtaUpdateImageListRequestResponse(_)
            | Event::OtaUpdateGroupListRequestResponse(_) => {
                // Send to app handler
                if let Err(e) = send("sock", &event, &mut runners).await {
                    log::error!("{}", e);
                }
            }
            _ => (),
        }
    }
}

/// Local only function to search for and find the corresponding sender
async fn send(
    task_name: &str,
    event: &Event,
    runners: &mut HashMap<String, Sender<Event>>,
) -> Result<()> {
    match runners.get_mut(task_name) {
        Some(sender) => {
            sender.send_async(event.clone()).await?;
            Ok(())
        }
        None => Err(anyhow!("{} broker task not registered!", task_name)),
    }
}
