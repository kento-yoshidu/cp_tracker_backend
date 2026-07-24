use actix_web::web;
use aws_sdk_s3::Client;
use serde::{Serialize, de::DeserializeOwned};

use crate::models::DataFile;

pub async fn read_json<T: DeserializeOwned>(
    client: web::Data<Client>,
    data_file: DataFile,
) -> Option<Vec<T>> {
    if std::env::var("USE_LOCAL_FILE").is_ok() {
        let data = std::fs::read_to_string(data_file.filename()).ok()?;

        serde_json::from_str(&data).ok()
    } else {
        let bucket = std::env::var("S3_BUCKET").unwrap();

        let res = client
            .get_object()
            .bucket(&bucket)
            .key(data_file.filename())
            .send()
            .await;

        match res {
            Ok(output) => {
                let bytes = output.body.collect().await.unwrap().into_bytes();
                let problems: Vec<T> = serde_json::from_slice(&bytes).unwrap();
                Some(problems)
            },
            Err(e) => {
                println!("{:?}", e);
                None
            }
        }
    }
}

pub async fn write_json<T: Serialize>(
    client: web::Data<Client>,
    data_file: DataFile,
    problems: &Vec<T>
) -> Option<()> {
    let data = serde_json::to_string_pretty(problems).ok()?;

    if std::env::var("USE_LOCAL_FILE").is_ok() {
        std::fs::write(data_file.filename(), data).ok()
    } else {
        let bucket = std::env::var("S3_BUCKET").unwrap();

        client
            .put_object()
            .bucket(&bucket)
            .key(data_file.filename())
            .body(data.into_bytes().into())
            .send()
            .await
            .ok()
            .map(|_| ())
    }
}
