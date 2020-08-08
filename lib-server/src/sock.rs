// Sytem related
use dotenv;
use log::{debug, error, info};
use std::{process, str};

// Tokio Related
use tokio::io::AsyncReadExt;
use tokio::net::UnixListener;
use tokio::stream::StreamExt;
use tokio::sync::mpsc::{channel, Sender};

// Local lib related
use pyrinas_shared::Event;

// Only requires a sender. No response necessary here... yet.
pub async fn run(mut broker_sender: Sender<Event>) {
  let socket_path = dotenv::var("PYRINAS_SOCKET_PATH").unwrap_or_else(|_| {
    error!("PYRINAS_SOCKET_PATH must be set in environment!");
    process::exit(1);
  });

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
  let _ = std::fs::remove_file(&socket_path);

  // Make connection
  let mut listener = UnixListener::bind(&socket_path).expect("Unable to bind!");
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
