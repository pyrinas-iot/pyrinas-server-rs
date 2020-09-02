// Sytem related
use log::{debug, info};
use std::str;

// Config related
use pyrinas_shared::settings::Settings;

// Tokio Related
use tokio::io::AsyncReadExt;
use tokio::net::UnixListener;
use tokio::stream::StreamExt;
use tokio::sync::mpsc::{channel, Sender};

// Local lib related
use pyrinas_shared::Event;

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
      .expect("Unable to write to socket.");

    // Get string from the buffer
    let s = str::from_utf8(&buffer).unwrap();
    info!("{}", s);

    // Decode into struct
    let res: pyrinas_shared::NewOta = serde_json::from_str(&s).unwrap();

    // Send result back to broker
    let _ = broker_sender
      .send(Event::OtaNewPackage {
        uid: res.uid,
        package: res.package,
      })
      .await;
  }
}
