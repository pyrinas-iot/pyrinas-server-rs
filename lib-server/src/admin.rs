use std::sync::Arc;

// async Related
use flume::{unbounded, Sender};
use futures::{FutureExt, StreamExt};
use tokio::sync::Mutex;
use warp::ws::Message;

// Unix listener
use warp::{ws::WebSocket, Filter};

// Local lib related
use crate::settings;
use crate::Event;
use pyrinas_shared::ManagmentDataType;

// Cbor
use serde_cbor;

// Error
use crate::Error;

pub type AdminClient = Arc<Mutex<Option<Sender<Result<Message, warp::Error>>>>>;

// Handle the incoming connection
async fn handle_connection(
    broker_sender: Sender<Event>,
    websocket: WebSocket,
    client: AdminClient,
) {
    log::debug!("Got stream!");

    // Ensure only one admin connection
    if client.lock().await.is_some() {
        log::warn!("Already connected to admin client!");
        return;
    }

    // Make a connection
    let (ws_tx, mut ws_rx) = websocket.split();

    // Use an unbounded channel to handle buffering and flushing of messages
    // to the websocket...
    let (tx, rx) = unbounded();
    tokio::task::spawn(rx.into_stream().forward(ws_tx).map(|result| {
        if let Err(e) = result {
            eprintln!("websocket send error: {}", e);
        }
    }));

    // Handle tx portion of things..
    {
        *client.lock().await = Some(tx);
    }

    while let Some(Ok(msg)) = ws_rx.next().await {
        log::debug!("msg size: {}", msg.as_bytes().len());

        // First deocde into ManagementRequest struct
        let req: pyrinas_shared::ManagementData =
            serde_cbor::from_slice(msg.as_bytes()).expect("Unable to deserialize ManagementData");

        // Next step in the managment request process
        match req.cmd {
            ManagmentDataType::AddOta => {
                // Dedcode ota update
                let ota_update: pyrinas_shared::ota::v2::OTAUpdate =
                    serde_cbor::from_slice(&req.msg).expect("Unable to deserialize OtaUpdate");

                // Send if decode was successful
                let _ = broker_sender
                    .send_async(Event::OtaNewPackage(ota_update))
                    .await
                    .expect("Unable to send OtaNewPackage to broker.");
            }
            ManagmentDataType::LinkOta => {
                // Dedcode ota update
                let a: pyrinas_shared::OtaLink =
                    serde_cbor::from_slice(&req.msg).expect("Unable to deserialize OtaLink");

                // Send if decode was successful
                let _ = broker_sender
                    .send_async(Event::OtaLink {
                        device_id: a.device_id,
                        group_id: a.group_id,
                        image_id: a.image_id,
                    })
                    .await
                    .expect("Unable to send OtaNewPackage to broker.");
            }
            ManagmentDataType::RemoveOta => {
                // Dedcode ota update
                let image_id = match String::from_utf8(req.msg) {
                    Ok(id) => id,
                    Err(_) => {
                        log::warn!("Unable to get image_id!");
                        continue;
                    }
                };

                // Send if decode was successful
                let _ = broker_sender
                    .send_async(Event::OtaDeletePackage(image_id))
                    .await
                    .expect("Unable to send OtaNewPackage to broker.");
            }
            // Otherwise send all others to application
            ManagmentDataType::Application => {
                broker_sender
                    .send_async(Event::ApplicationManagementRequest(req))
                    .await
                    .expect("Unable to send ApplicationManagementRequest to broker.");
            }
            ManagmentDataType::UnlinkOta => {
                // Dedcode ota update
                let a: pyrinas_shared::OtaLink =
                    serde_cbor::from_slice(&req.msg).expect("Unable to deserialize OtaLink");

                // Send if decode was successful
                let _ = broker_sender
                    .send_async(Event::OtaUnlink {
                        device_id: a.device_id,
                        group_id: a.group_id,
                    })
                    .await
                    .expect("Unable to send OtaNewPackage to broker.");
            }
            ManagmentDataType::GetGroupList => {
                broker_sender
                    .send_async(Event::OtaUpdateGroupListRequest())
                    .await
                    .expect("Unable to send ApplicationManagementRequest to broker.");
            }
            ManagmentDataType::GetImageList => {
                broker_sender
                    .send_async(Event::OtaUpdateImageListRequest())
                    .await
                    .expect("Unable to send ApplicationManagementRequest to broker.");
            }
        }
    }

    // Handle tx portion of things..
    {
        *client.lock().await = None;
    }
}

// Only requires a sender. No response necessary here... yet.
pub async fn run(settings: &settings::Admin, broker_sender: Sender<Event>) -> Result<(), Error> {
    // Get the sender/reciever associated with this particular task
    let (sender, receiver) = unbounded::<Event>();

    // Handle reciever end
    let client: AdminClient = Default::default();
    let from_broker_client = client.clone();

    // Client filter
    let client_filter = warp::any().map(move || client.clone());

    tokio::task::spawn(async move {
        let c = from_broker_client;

        while let Ok(event) = receiver.recv_async().await {
            let data = match event {
                Event::OtaUpdateImageListRequestResponse(r) => match serde_cbor::to_vec(&r) {
                    Ok(v) => v,
                    Err(_) => {
                        log::warn!("Unable to serialize image list!");
                        continue;
                    }
                },
                Event::OtaUpdateGroupListRequestResponse(r) => match serde_cbor::to_vec(&r) {
                    Ok(v) => v,
                    Err(_) => {
                        log::warn!("Unable to serialize group list!");
                        continue;
                    }
                },
                Event::ApplicationManagementResponse(r) => match serde_cbor::to_vec(&r) {
                    Ok(v) => v,
                    Err(_) => {
                        log::warn!("Unable to application management response!");
                        continue;
                    }
                },
                _ => {
                    log::warn!("Unhandled command sent to admin!");
                    continue;
                }
            };

            if let Some(c) = c.lock().await.as_ref() {
                if let Err(e) = c.send_async(Ok(Message::binary(data))).await {
                    log::error!("Unabe to send message to admin! Err: {}", e);
                };
            }
        }
    });

    // Register this task
    broker_sender
        .send_async(Event::NewRunner {
            name: "sock".to_string(),
            sender: sender.clone(),
        })
        .await?;

    // ! Important: this leaks the api_key into the exact function. As long as this is only called once
    // it's NBD.
    let stream = warp::get()
        .and(warp::path("socket"))
        .and(warp::header::exact(
            "ApiKey",
            Box::leak(settings.api_key.clone().into_boxed_str()),
        ))
        .and(warp::ws())
        .and(client_filter)
        .map(move |ws: warp::ws::Ws, client: AdminClient| {
            log::debug!("Before upgrade..");

            let broker_sender = broker_sender.clone();

            // And then our closure will be called when it completes...
            ws.on_upgrade(|socket| {
                // handle the connection
                handle_connection(broker_sender, socket, client)
            })
        });

    // Run the `warp` server
    warp::serve(stream)
        .run(([127, 0, 0, 1], settings.port))
        .await;

    Ok(())
}

// TODO: (test) try to send an "other" managment_request (gets forwarded to the application.)
// TODO: (test) try to send an "add_ota" command
