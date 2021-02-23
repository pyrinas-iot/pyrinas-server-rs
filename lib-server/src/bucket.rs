// System related
use log::{error, info, warn};

// S3 Bucket related
use s3::{bucket::Bucket,creds::Credentials};

// Config related
use pyrinas_shared::{settings::PyrinasSettings,Event};

// Async Related
use std::sync::Arc;
use flume::{unbounded, Sender};

pub async fn run(settings: Arc<PyrinasSettings>, broker_sender: Sender<Event>) {
  // Set up AWS conection
  let credentials = Credentials::new(
    Some(&settings.s3.access_key),
    Some(&settings.s3.secret_key),
    None,
    None,
    None,
  )
  .unwrap_or_else(|e| {
    error!("Unable to create AWS credentials! {}", e);
    std::process::exit(1);
  });

  // Get the sender/reciever associated with this particular task
  let (sender, reciever) = unbounded::<Event>();

  // Register this task
  broker_sender
    .send_async(Event::NewRunner {
      name: "bucket".to_string(),
      sender: sender.clone(),
    })
    .await
    .unwrap();

  // Parse region
  let region = settings
    .s3
    .region
    .parse()
    .expect("Unable to parse AWS region.");

  // Create bucket
  let bucket =
    Bucket::new(&settings.s3.bucket, region, credentials).expect("Unable to create bucket!");

  // Wait for event on reciever
  while let Ok(event) = reciever.recv_async().await {
    match event {
      Event::OtaDeletePackage(update) => {
        info!("bucket_run: OtaDeletePackage");

        if let Some(package) = update.package {
          // Handle deletion of file from AWS S3
          if let Err(e) = bucket.delete_object(&package.file).await {
            warn!("Unable to delete: {}. Error: {}", &package.file, e);
          }
        }
      }
      _ => {}
    }
  }
}


// TODO: upload a file and then run this portion of code and delete the data