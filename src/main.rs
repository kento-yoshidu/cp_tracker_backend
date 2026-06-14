use actix_web::{get, App, HttpResponse, HttpServer, Responder, web};
use aws_sdk_s3::Client;

#[get("/hello")]
async fn hello() -> impl Responder {
    HttpResponse::Ok().body("Hello World")
}

#[get("/data")]
async fn get_data(client: web::Data<Client>) -> impl Responder {
    let bucket = std::env::var("S3_BUCKET").unwrap();
    
    let resp = client
        .get_object()
        .bucket(&bucket)
        .key("data.json")
        .send()
        .await;

    match resp {
        Ok(output) => {
            let bytes = output.body.collect().await.unwrap().into_bytes();
            HttpResponse::Ok()
                .content_type("application/json")
                .body(bytes)
        }
        Err(_) => HttpResponse::InternalServerError().body("Internal Server Error"),
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenvy::dotenv().ok();

    let config = aws_config::load_from_env().await;
    let client = Client::new(&config);

    let port = std::env::var("PORT").unwrap_or("8080".to_string());

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(client.clone()))
            .service(hello)
            .service(get_data)
    })
    .bind(format!("0.0.0.0:{port}"))?
    .run()
    .await
}
