// System related
use log::{error, info, warn};

// S3 Bucket related
use awscreds::Credentials;
use s3::bucket::Bucket as aws;

// Config related
use pyrinas_shared::settings::Settings;

// Tokio Related
use tokio::sync::mpsc::{channel, Sender};

// Local lib related
use pyrinas_shared::Event;

pub async fn run(settings: Settings, mut broker_sender: Sender<Event>) {
  // Set up AWS conection
  let credentials = Credentials::new(
    Some(&settings.s3.access_key),
    Some(&settings.s3.secret_key),
    None,
    None,
    None,
  )
  .await
  .unwrap_or_else(|e| {
    error!("Unable to create AWS credentials! {}", e);
    std::process::exit(1);
  });

  // Get the sender/reciever associated with this particular task
  let (sender, mut reciever) = channel::<pyrinas_shared::Event>(20);

  // Register this task
  broker_sender
    .send(Event::NewRunner {
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
    aws::new(&settings.s3.bucket, region, credentials).expect("Unable to create bucket!");

  // Wait for event on reciever
  while let Some(event) = reciever.recv().await {
    match event {
      Event::OtaDeletePackage(update) => {
        info!("bucket_run: OtaDeletePackage");

        // Handle deletion of file from AWS S3
        if let Err(e) = bucket.delete_object(&update.package.file).await {
          warn!("Unable to delete: {}. Error: {}", &update.package.file, e);
        }
      }
      _ => {}
    }
  }
}
