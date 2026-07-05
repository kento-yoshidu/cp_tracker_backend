use actix_web::web;
use aws_sdk_s3::Client;

use crate::models::Problem;

pub async fn read_json(client: web::Data<Client>) -> Option<Vec<Problem>> {
    if std::env::var("USE_LOCAL_FILE").is_ok() {
        let data = std::fs::read_to_string("problems.json").ok()?;
        serde_json::from_str(&data).ok()
    } else {
        let bucket = std::env::var("S3_BUCKET").unwrap();

        let res = client
            .get_object()
            .bucket(&bucket)
            .key("problems.json")
            .send()
            .await;

        match res {
            Ok(output) => {
                let bytes = output.body.collect().await.unwrap().into_bytes();
                let problems: Vec<Problem> = serde_json::from_slice(&bytes).unwrap();
                Some(problems)
            },
            Err(e) => {
                println!("{:?}", e);
                None
            }
        }
    }
}

pub async fn write_json(client: web::Data<Client>, problems: &Vec<Problem>) -> Option<()> {
    let data = serde_json::to_string_pretty(problems).ok()?;

    if std::env::var("USE_LOCAL_FILE").is_ok() {
        std::fs::write("problems.json", data).ok()
    } else {
        let bucket = std::env::var("S3_BUCKET").unwrap();

        client
            .put_object()
            .bucket(&bucket)
            .key("problems.json")
            .body(data.into_bytes().into())
            .send()
            .await
            .ok()
            .map(|_| ())
    }
}
