use hmac::{Hmac, Mac};
use sha2::Sha256;
use base64::{engine::general_purpose, Engine as _};
use aws_sdk_cognitoidentityprovider::{Client as CognitoClient, types::AuthFlowType};
use actix_web::{web, get, post, Responder, HttpRequest};
use actix_web::cookie::{Cookie, SameSite, time::Duration};
use crate::models::LoginRequest;


type HmacSha256 = Hmac<Sha256>;

fn compute_secret_hash(username: &str, client_id: &str, client_secret: &str) -> String {
    let message = format!("{username}{client_id}");
    let mut mac = HmacSha256::new_from_slice(client_secret.as_bytes())
        .expect("HMAC accepts key of any size");
    mac.update(message.as_bytes());
    general_purpose::STANDARD.encode(mac.finalize().into_bytes())
}

pub async fn login(
    cognito: &CognitoClient,
    client_id: &str,
    client_secret: &str,
    username: &str,
    password: &str,
) -> Result<String, ()> {
    let secret_hash = compute_secret_hash(username, client_id, client_secret);

    let resp = cognito
        .initiate_auth()
        .client_id(client_id)
        .auth_flow(AuthFlowType::UserPasswordAuth)
        .auth_parameters("USERNAME", username)
        .auth_parameters("PASSWORD", password)
        .auth_parameters("SECRET_HASH", secret_hash)
        .send()
        .await
        .map_err(|_| ())?;

    resp.authentication_result()
        .and_then(|r| r.access_token())
        .map(|t| t.to_string())
        .ok_or(())
}


#[derive(serde::Deserialize, Clone)]
pub struct Jwk { pub kid: String, pub n: String, pub e: String }

#[derive(serde::Deserialize, Clone)]
pub struct Jwks { pub keys: Vec<Jwk> }

pub async fn fetch_jwks(region: &str, user_pool_id: &str) -> Result<Jwks, reqwest::Error> {
    let url = format!("https://cognito-idp.{region}.amazonaws.com/{user_pool_id}/.well-known/jwks.json");
    reqwest::get(&url).await?.json::<Jwks>().await
}


use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};

#[derive(serde::Deserialize)]
pub struct Claims {
    pub sub: String,
    pub token_use: String,
    pub client_id: String,
    pub exp: usize,
}

pub fn verify_token(token: &str, jwks: &Jwks, expected_client_id: &str, issuer: &str) -> Result<Claims, ()> {
    let kid = decode_header(token).map_err(|_| ())?.kid.ok_or(())?;
    let jwk = jwks.keys.iter().find(|k| k.kid == kid).ok_or(())?;
    let key = DecodingKey::from_rsa_components(&jwk.n, &jwk.e).map_err(|_| ())?;

    let mut validation = Validation::new(Algorithm::RS256);
    validation.set_issuer(&[issuer]);

    let data = decode::<Claims>(token, &key, &validation).map_err(|_| ())?;

    if data.claims.token_use != "access" || data.claims.client_id != expected_client_id {
        return Err(());
    }
    Ok(data.claims)
}

use actix_web::{middleware::Next, dev::{ServiceRequest, ServiceResponse}, body::{MessageBody, BoxBody}, Error, HttpResponse};

pub async fn require_auth(
    req: ServiceRequest,
    next: Next<impl MessageBody + 'static>,
) -> Result<ServiceResponse<BoxBody>, Error> {
    let Some(token) = req.cookie("session").map(|c| c.value().to_string()) else {
        return Ok(req.into_response(HttpResponse::Unauthorized().finish()).map_into_boxed_body());
    };

    let jwks = req.app_data::<web::Data<Jwks>>().unwrap();
    let client_id = std::env::var("COGNITO_CLIENT_ID").unwrap();
    let region = std::env::var("COGNITO_REGION").unwrap();
    let user_pool_id = std::env::var("COGNITO_USER_POOL_ID").unwrap();
    let issuer = format!("https://cognito-idp.{region}.amazonaws.com/{user_pool_id}");

    match verify_token(&token, jwks, &client_id, &issuer) {
        Ok(_) => next.call(req).await.map(|res| res.map_into_boxed_body()),
        Err(_) => Ok(req.into_response(HttpResponse::Unauthorized().finish()).map_into_boxed_body()),
    }
}

#[post("/login")]
pub async fn login_handler(cognito: web::Data<CognitoClient>, body: web::Json<LoginRequest>) -> impl Responder {
    let client_id = std::env::var("COGNITO_CLIENT_ID").unwrap();
    let client_secret = std::env::var("COGNITO_CLIENT_SECRET").unwrap();

    match login(&cognito, &client_id, &client_secret, &body.username, &body.password).await {
        Ok(access_token) => {
            let cookie = Cookie::build("session", access_token)
                .http_only(true)
                .secure(true)
                .same_site(SameSite::None)
                .path("/")
                .max_age(Duration::days(1))
                .finish();
            HttpResponse::Ok().cookie(cookie).finish()
        }
        Err(_) => HttpResponse::Unauthorized().finish(),
    }
}

#[get("/me")]
pub async fn me_handler(req: HttpRequest, jwks: web::Data<Jwks>) -> impl Responder {
    let Some(token) = req.cookie("session").map(|c| c.value().to_string()) else {
        return HttpResponse::Unauthorized().finish();
    };

    let region = std::env::var("COGNITO_REGION").unwrap();
    let client_id = std::env::var("COGNITO_CLIENT_ID").unwrap();
    let user_pool_id = std::env::var("COGNITO_USER_POOL_ID").unwrap();
    let issuer = format!("https://cognito-idp.{region}.amazonaws.com/{user_pool_id}");

    match verify_token(&token, &jwks, &client_id, &issuer) {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(_) => HttpResponse::Unauthorized().finish(),
    }
}
