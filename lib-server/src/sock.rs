// Sytem related
use log::{debug, info, error, warn};

// Config related
use pyrinas_shared::settings::Settings;

// Tokio Related
use tokio::io::AsyncReadExt;
use tokio::net::UnixListener;
use tokio::stream::StreamExt;
use tokio::sync::mpsc::{channel, Sender};

// Local lib related
use pyrinas_shared::Event;

// Cbor
use serde_cbor;

// Only requires a sender. No response necessary here... yet.
pub async fn run(settings: Settings, mut broker_sender: Sender<Event>) {
  // Get the sender/reciever associated with this particular task
  let (sender, _) = channel::<pyrinas_shared::Event>(20);

  // Register this task
  broker_sender
    .send(Event::NewRunner {
      name: "sock".to_string(),
      sender: sender.clone(),
    })
    .await
    .unwrap();

  debug!("Removing previous sock!");

  // Remove previous socket
  let _ = std::fs::remove_file(&settings.sock.path);

  // Make connection
  let mut listener = UnixListener::bind(&settings.sock.path).expect("Unable to bind!");
  let mut incoming = listener.incoming();

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
      error!("No message received over socket");
      continue;
    }

    // First deocde into ManagementRequest struct
    let managment_request: Result<pyrinas_shared::ManagementData, serde_cbor::error::Error> =
      serde_cbor::from_slice(&buffer);

    // Next step in the managment request process
    match managment_request {
      Ok(req) =>
        match req.target.as_str() {
          "add_ota" => {

            // Dedcode ota update
            let ota_update: Result<pyrinas_shared::OtaUpdate, serde_cbor::error::Error> =
      serde_cbor::from_slice(&buffer);

            // Send if decode was successful
            match ota_update {
              Ok(p) => {let _ = broker_sender.send(Event::OtaNewPackage(p)).await;}
              Err(e) => warn!("Unable to get OtaUpdate. Error: {}",e)
            }

          }
          // Otherwise send all others to application
          _ => {
            info!("to app");
          }
        }
      Err(e) => error!("Unable to decode ManagementRequest. Error: {}", e)
    }
  }
}
