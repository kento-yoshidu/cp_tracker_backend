use actix_web::{get, App, HttpResponse, HttpServer, Responder, web};
use aws_sdk_s3::Client;
use handlers::{post_ac, check_duplicate};
use handlers::create_problem;

mod models;
mod store;
mod handlers;

#[get("/hello")]
async fn hello() -> impl Responder {
    HttpResponse::Ok().body("Hello World")
}

#[get("/problems")]
async fn get_problems(client: web::Data<Client>) -> impl Responder {
    match store::read_json(client).await {
        Some(problems) => HttpResponse::Ok()
            .content_type("application/json")
            .body(serde_json::to_string(&problems).unwrap()),
        None => HttpResponse::InternalServerError().finish(),
    }
}

#[get("/data")]
async fn get_data(client: web::Data<Client>) -> impl Responder {
    let bucket = std::env::var("S3_BUCKET").unwrap();

    let resp = client
        .get_object()
        .bucket(&bucket)
        .key("problems.json")
        .send()
        .await;

    match resp {
        Ok(output) => {
            let bytes = output.body.collect().await.unwrap().into_bytes();
            HttpResponse::Ok()
                .content_type("application/json")
                .body(bytes)
        }
        Err(e) => HttpResponse::InternalServerError().body(format!("{e:#?}")),
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
            .service(get_problems)
            .service(create_problem)
            .service(check_duplicate)
            .service(post_ac)
    })
    .bind(format!("0.0.0.0:{port}"))?
    .run()
    .await
}
