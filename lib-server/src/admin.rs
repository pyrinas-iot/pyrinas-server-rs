// async Related
use flume::{unbounded, Sender};
use futures::StreamExt;

// Unix listener
cfg_if::cfg_if! {

   if #[cfg(feature = "runtime_tokio")] {
    use warp::{ws::WebSocket,Filter};
   } else if #[cfg(feature = "runtime_async_std")] {
    use async_std::os::unix::net::UnixListener;
    use async_std::prelude::*;
    use async_std::task;
   }

}

// Local lib related
use crate::settings;
use crate::Event;
use pyrinas_shared::ManagmentDataType;

// Cbor
use serde_cbor;

// Handle the incoming connection
async fn handle_connection(broker_sender: Sender<Event>, websocket: WebSocket) {
    log::debug!("Got stream!");

    // Make a connection
    let (_, mut incoming) = websocket.split();

    loop {
        // Get the next message
        let msg = match incoming.next().await {
            Some(m) => match m {
                Ok(m) => m,
                Err(_) => break,
            },
            None => break,
        };

        log::debug!("msg size: {}", msg.as_bytes().len());

        // First deocde into ManagementRequest struct
        let req: pyrinas_shared::ManagementData =
            serde_cbor::from_slice(&msg.as_bytes()).expect("Unable to deserialize ManagementData");

        // Next step in the managment request process
        match req.cmd {
            ManagmentDataType::AddOta => {
                // Dedcode ota update
                let ota_update: pyrinas_shared::OtaUpdate =
                    serde_cbor::from_slice(&req.msg).expect("Unable to deserialize OtaUpdate");

                // Send if decode was successful
                let _ = broker_sender
                    .send_async(Event::OtaNewPackage(ota_update))
                    .await
                    .expect("Unable to send OtaNewPackage to broker.");
            }
            ManagmentDataType::RemoveOta => {}
            // Otherwise send all others to application
            ManagmentDataType::Application => {
                broker_sender
                    .send_async(Event::ApplicationManagementRequest(req))
                    .await
                    .expect("Unable to send ApplicationManagementRequest to broker.");
            }
        }
    }
}

// Only requires a sender. No response necessary here... yet.
pub async fn run(settings: &settings::Admin, broker_sender: Sender<Event>) -> anyhow::Result<()> {
    // Get the sender/reciever associated with this particular task
    let (sender, _) = unbounded::<Event>();

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
        .map(move |ws: warp::ws::Ws| {
            log::debug!("Before upgrade..");

            let broker_sender = broker_sender.clone();

            // And then our closure will be called when it completes...
            ws.on_upgrade(|socket| {
                // TODO: handle the connection
                handle_connection(broker_sender, socket)
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
