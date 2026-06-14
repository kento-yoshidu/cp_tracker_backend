use actix_web::{HttpResponse, HttpServer, Responder, get, App};

#[get("/hello")]
async fn hello() -> impl Responder {
    HttpResponse::Ok().body("Hello World")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let port = std::env::var("PORT").unwrap_or("8080".to_string());

    HttpServer::new(|| {
        App::new().service(hello)
    })
    .bind(format!("0.0.0.0:{port}"))?
    .run()
    .await
}
