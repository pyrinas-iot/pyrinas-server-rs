// Sytem related
use log::{debug, error, warn};

// async Related
use flume::{unbounded, Sender};
use std::sync::Arc;

// Unix listener
cfg_if::cfg_if! {

   if #[cfg(feature = "runtime_tokio")] {
    use tokio::io::AsyncReadExt;
    use tokio::net::UnixListener;
    use tokio_stream::wrappers::UnixListenerStream;
    use tokio_stream::StreamExt;
   } else if #[cfg(feature = "runtime_async_std")] {
    use async_std::os::unix::net::UnixListener;
    use async_std::prelude::*;
   }

}

// Local lib related
use pyrinas_shared::settings::PyrinasSettings;
use pyrinas_shared::Event;

// Cbor
use serde_cbor;

// Only requires a sender. No response necessary here... yet.
pub async fn run(settings: Arc<PyrinasSettings>, broker_sender: Sender<Event>) {
    // Get the sender/reciever associated with this particular task
    let (sender, _) = unbounded::<Event>();

    // Register this task
    broker_sender
        .send_async(Event::NewRunner {
            name: "sock".to_string(),
            sender: sender.clone(),
        })
        .await
        .unwrap();

    debug!("Removing previous sock!");

    // Remove previous socket
    let _ = std::fs::remove_file(&settings.sock.path);

    // Make a connection
    cfg_if::cfg_if! {

       // Make connection using tokio
       if #[cfg(feature = "runtime_tokio")] {
           let mut incoming =
           UnixListenerStream::new(UnixListener::bind(&settings.sock.path).expect("Unable to bind!"));
       // Make connection using async-std
       } else if #[cfg(feature = "runtime_async_std")] {
           let listener = UnixListener::bind(&settings.sock.path)
               .await
               .expect("Unable to bind!");

           let mut incoming = listener.incoming();
       }

    }

    debug!("Created socket listener!");

    while let Some(stream) = incoming.next().await {
        debug!("Got stream!");

        // Setup work to make this happen
        let mut stream = stream.unwrap(); //Box::<UnixStream>::new(stream.unwrap());
        let mut buffer = Vec::new();

        // Read until the stream closes
        stream
            .read_to_end(&mut buffer)
            .await
            .expect("Unable to read from socket.");

        // If no message, then continue
        if buffer.len() <= 0 {
            warn!("No message received over socket");
            continue;
        }

        // First deocde into ManagementRequest struct
        let managment_request: Result<pyrinas_shared::ManagementData, serde_cbor::error::Error> =
            serde_cbor::from_slice(&buffer);

        // Next step in the managment request process
        match managment_request {
            Ok(req) => match req.target.as_str() {
                "add_ota" => {
                    // Dedcode ota update
                    let ota_update: Result<pyrinas_shared::OtaUpdate, serde_cbor::error::Error> =
                        serde_cbor::from_slice(&req.msg);

                    // Send if decode was successful
                    match ota_update {
                        Ok(p) => {
                            let _ = broker_sender.send_async(Event::OtaNewPackage(p)).await;
                        }
                        Err(e) => warn!("Unable to get OtaUpdate. Error: {}", e),
                    }
                }
                // Otherwise send all others to application
                _ => {
                    if let Err(e) = broker_sender
                        .send_async(Event::ApplicationManagementRequest(req))
                        .await
                    {
                        warn!("Unable to send ApplicationManagementRequest. Error: {}", e);
                    }
                }
            },
            Err(e) => error!("Unable to decode ManagementRequest. Error: {}", e),
        }
    }
}

// TODO: (test) try to send an "other" managment_request (gets forwarded to the application.)
// TODO: (test) try to send an "add_ota" command
