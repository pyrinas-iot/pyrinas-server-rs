// System related
use dotenv;
use log::{error, info, warn};

// S3 Bucket related
use awscreds::Credentials;
use s3::bucket::Bucket;

// Tokio Related
use tokio::sync::mpsc::{channel, Sender};

// Local lib related
use pyrinas_shared::Event;

pub async fn run(mut broker_sender: Sender<Event>) {
  let aws_access_key = dotenv::var("PYRINAS_AWS_ACCESS_KEY").unwrap_or_else(|_| {
    error!("PYRINAS_AWS_ACCESS_KEY must be set in environment!");
    std::process::exit(1);
  });

  let aws_secret_key = dotenv::var("PYRINAS_AWS_SECRET_KEY").unwrap_or_else(|_| {
    error!("PYRINAS_AWS_SECRET_KEY must be set in environment!");
    std::process::exit(1);
  });

  let aws_region = dotenv::var("PYRINAS_AWS_REGION").unwrap_or_else(|_| {
    error!("PYRINAS_AWS_REGION must be set in environment!");
    std::process::exit(1);
  });

  let aws_bucket = dotenv::var("PYRINAS_AWS_BUCKET").unwrap_or_else(|_| {
    error!("PYRINAS_AWS_BUCKET must be set in environment!");
    std::process::exit(1);
  });

  // Set up AWS conection
  let region = aws_region.parse().expect("Unable to parse AWS region.");
  let credentials = Credentials::new(
    Some(&aws_access_key),
    Some(&aws_secret_key),
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

  // Create bucket
  let bucket = Bucket::new(&aws_bucket, region, credentials).expect("Unable to create bucket!");

  // Wait for event on reciever
  while let Some(event) = reciever.recv().await {
    match event {
      Event::OtaDeletePackage { uid: _, package } => {
        info!("bucket_run: OtaDeletePackage");

        // Handle deletion of file from AWS S3
        if let Err(e) = bucket.delete_object(&package.file).await {
          warn!("Unable to delete: {}. Error: {}", &package.file, e);
        }
      }
      _ => {}
    }
  }
}
