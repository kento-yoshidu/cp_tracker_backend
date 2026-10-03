use actix_web::{App, HttpServer, web, middleware::from_fn};
use aws_sdk_s3::Client;
use aws_sdk_cognitoidentityprovider::Client as CognitoClient;
use handlers::{
    get_problems,
    get_activities,
    create_problem,
    update_problem,
    post_ac,
    delete_problem,
    check_duplicate,
    archive,
    get_archives,
    delete_archive,
    restore_archive,
};
use auth::{login_handler, me_handler, require_auth, fetch_jwks};

mod models;
mod store;
mod handlers;
mod auth;
mod time;

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
            .service(get_problems)
            .service(get_activities)
            .service(check_duplicate)
            .service(login_handler)
            .service(me_handler)
            .service(get_archives)
            .service(
                web::scope("")
                    .wrap(from_fn(require_auth))
                    .service(create_problem)
                    .service(update_problem)
                    .service(delete_problem)
                    .service(post_ac)
                    .service(archive)
                    .service(delete_archive)
                    .service(restore_archive)
            )
    })
    .bind(format!("0.0.0.0:{port}"))?
    .run()
    .await
}
