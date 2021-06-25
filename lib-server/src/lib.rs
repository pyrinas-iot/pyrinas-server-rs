// Lib related
pub mod admin;
pub mod broker;
pub mod influx;
pub mod mqtt;
pub mod ota;
pub mod settings;
pub mod telemetry;

pub use pyrinas_shared::*;

// Influx/Event
use influxdb::{ReadQuery, WriteQuery};

// Async Related
use flume::{Receiver, Sender};
use pyrinas_shared::ota::{v2::OtaUpdate, OtaUpdateVersioned, OtaVersion};
use std::{io, sync::Arc};

// Runtime
use tokio::task;

// MQTT related
use librumqttd::async_locallink::construct_broker;

// Error
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("{source}")]
    SendError {
        #[from]
        source: flume::SendError<Event>,
    },

    // #[error("{source}")]
    // TokioError {
    //     #[from]
    //     source: tokio::Error,
    // },
    #[error("{source}")]
    MqttError {
        #[from]
        source: librumqttd::Error,
    },

    #[error("{source}")]
    MqttLinkError {
        #[from]
        source: librumqttd::async_locallink::LinkError,
    },

    #[error("{source}")]
    CborError {
        #[from]
        source: serde_cbor::Error,
    },

    #[error("{source}")]
    SledError {
        #[from]
        source: sled::Error,
    },

    #[error("{source}")]
    StringConversionError {
        #[from]
        source: std::string::FromUtf8Error,
    },

    #[error("{source}")]
    IoError {
        #[from]
        source: io::Error,
    },

    #[error("{source}")]
    TokioTaskError {
        #[from]
        source: tokio::task::JoinError,
    },

    #[error("err: {0}")]
    CustomError(String),
}

// TODO: conditional use of tokio OR async_std
pub async fn run(
    settings: Arc<settings::PyrinasSettings>,
    broker_sender: Sender<Event>,
    broker_reciever: Receiver<Event>,
) -> Result<(), Error> {
    // Clone these appropriately
    let task_sender = broker_sender.clone();
    let task_settings = settings.clone();

    // Init influx connection
    if settings.clone().influx.is_some() {
        task::spawn(async move {
            influx::run(&task_settings.influx.to_owned().unwrap(), task_sender).await;
        });
    }

    // Ota task
    let task_sender = broker_sender.clone();
    let task_settings = settings.clone();
    task::spawn(async move {
        ota::run(&task_settings.ota, task_sender).await;
    });

    let task_settings = settings.clone();
    task::spawn(async move {
        ota::ota_http_run(&task_settings.ota).await;
    });

    // Clone these appropriately
    let task_sender = broker_sender.clone();
    let task_settings = settings.clone();

    // Start unix socket task
    if task_settings.admin.is_some() {
        task::spawn(async move {
            if let Err(e) = admin::run(&task_settings.admin.to_owned().unwrap(), task_sender).await
            {
                log::error!("Admin runtime error! Err: {}", e);
            };
        });
    }

    // Set up broker
    let (mut router, _, rumqtt_server, builder) = construct_broker(settings.mqtt.rumqtt.clone());

    // Running switch
    task::spawn(async {
        rumqtt_server.await;
    });

    // Spawn router task (needs to be done before anything else or else builder.connect blocks)
    task::spawn_blocking(move || {
        if let Err(e) = router.start() {
            log::error!("mqtt router error. err: {}", e);
        }
    });

    // Get the rx/tx channels
    let (mut tx, mut rx) = builder.connect("localclient", 200).await?;

    // Subscribe
    tx.subscribe(settings.mqtt.topics.clone()).await?;

    // Spawn a new task(s) for the MQTT stuff
    let task_sender = broker_sender.clone();

    // Start server task
    task::spawn(async move {
        mqtt::mqtt_run(&mut rx, task_sender).await;
    });

    // Start mqtt broker task
    let task_sender = broker_sender.clone();
    task::spawn(async move {
        mqtt::run(&mut tx, task_sender).await;
    });

    // Spawn the broker task that handles it all!
    // This blocks this async function from returning.
    // If this returns, the server it shot anyway..
    task::spawn(broker::run(broker_reciever)).await?;

    Ok(())
}

#[derive(Debug, Clone)]
pub enum Event {
    NewRunner {
        name: String,
        sender: Sender<Event>,
    },
    OtaDeletePackage(String),
    OtaNewPackage(OtaUpdate),
    OtaDissociate {
        device_id: Option<String>,
        group_id: Option<String>,
    },
    OtaAssociate {
        device_id: Option<String>,
        group_id: Option<String>,
        image_id: Option<String>,
        ota_version: OtaVersion,
    }, // Associate device with update
    OtaRequest {
        device_id: String,
        msg: OtaRequest,
    },
    OtaResponse(OtaUpdateVersioned),
    OtaUpdateImageListRequest(), // Simple request to get all the firmware image information (id, name, desc, etc)
    OtaUpdateImageListRequestResponse(OtaImageListResponse), // Message sent to show all the avilable OTA updates
    OtaUpdateGroupListRequest(), // Simple request to get a list of all the groups with their memebers
    OtaUpdateGroupListRequestResponse(OtaGroupListResponse), // Message sent to show all the avilable group info
    ApplicationManagementRequest(ManagementData), // Message sent for configuration of application
    ApplicationManagementResponse(ManagementData), // Reponse from application management portion of the app
    ApplicationRequest(ApplicationData),           // Request/event from a device
    ApplicationResponse(ApplicationData),          // Reponse from other parts of the server
    InfluxDataSave(WriteQuery),                    // Takes a pre-prepared query and executes it
    InfluxDataRequest(ReadQuery), // Takes a pre-prepared query to *read* the database
    InfluxDataResponse,           // Is the response to InfluxDataRequest
}
