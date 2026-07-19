use actix_web::{get, App, HttpResponse, HttpServer, Responder, web, middleware::from_fn};
use aws_sdk_s3::Client;
use aws_sdk_cognitoidentityprovider::Client as CognitoClient;
use handlers::{
    get_problems,
    create_problem,
    update_problem,
    post_ac,
    delete_problem,
    check_duplicate,
};
use auth::{login_handler, me_handler, require_auth, fetch_jwks};

mod models;
mod store;
mod handlers;
mod auth;

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
    let cognito_client = CognitoClient::new(&config);

    let region = std::env::var("COGNITO_REGION").unwrap();
    let user_pool_id = std::env::var("COGNITO_USER_POOL_ID").unwrap();
    let jwks = fetch_jwks(&region, &user_pool_id)
        .await
        .expect("failed to fetch Cognito JWKS");

    let port = std::env::var("PORT").unwrap_or("8080".to_string());

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(client.clone()))
            .app_data(web::Data::new(cognito_client.clone()))
            .app_data(web::Data::new(jwks.clone()))
            .service(hello)
            .service(get_data)
            .service(get_problems)
            .service(check_duplicate)
            .service(login_handler)
            .service(me_handler)
            .service(
                web::scope("")
                    .wrap(from_fn(require_auth))
                    .service(create_problem)
                    .service(update_problem)
                    .service(delete_problem)
                    .service(post_ac)
            )
    })
    .bind(format!("0.0.0.0:{port}"))?
    .run()
    .await
}
