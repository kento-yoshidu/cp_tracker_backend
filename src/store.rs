use actix_web::web;
use aws_sdk_s3::Client;

use crate::models::Problem;

pub async fn read_json(client: web::Data<Client>) -> Option<Vec<Problem>> {
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
