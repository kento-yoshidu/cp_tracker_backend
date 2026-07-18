use actix_web::{HttpResponse, Responder, get, post, put, web};
use aws_sdk_s3::Client;
use uuid::Uuid;

use crate::{models::{CheckDuplicateRequest, CheckDuplicateResponse, CreateProblemRequest, Problem, UpdateProblemRequest}, store};

#[post("/problems/{id}/ac")]
pub async fn post_ac(
    client: web::Data<Client>,
    path: web::Path<String>
) -> impl Responder {
    let id = path.into_inner();

    let Some(mut problems) = store::read_json(client.clone()).await else {
        return HttpResponse::NotFound().finish();
    };

    let Some(problem) = problems.iter_mut().find(|p| p.id.to_string() == id) else {
        return HttpResponse::NotFound().finish();
    };

    problem.ac_count += 1;
    problem.last_solved_at = Some(chrono::Local::now().format("%Y%m%d").to_string());

    let updated = problem.clone();

    if store::write_json(client, &problems).await.is_none() {
        return HttpResponse::InternalServerError().finish();
    }

    HttpResponse::Ok().json(updated)
}

#[post("/problems")]
pub async  fn create_problem(
    client: web::Data<Client>,
    body: web::Json<CreateProblemRequest>,
) -> impl Responder {
    let req = body.into_inner();

    let Some(mut problems) = store::read_json(client.clone()).await else {
        return HttpResponse::NotFound().finish();
    };

    let new_problem = Problem {
        id: Uuid::new_v4(),
        platform: req.platform,
        url: req.url,
        title: req.title,
        tags: req.tags,
        difficulty: req.difficulty,
        ac_count: 0,
        created_at: Some(chrono::Local::now().to_rfc3339()),
        last_solved_at: None,
    };

    problems.push(new_problem.clone());

    if store::write_json(client, &problems).await.is_none() {
        HttpResponse::InternalServerError().finish();
    }

    HttpResponse::Created().json(new_problem)
}

#[put("/problems/{id}")]
pub async fn update_problem(
    client: web::Data<Client>,
    path: web::Path<String>,
    body: web::Json<UpdateProblemRequest>,
) -> impl Responder {
    let id = path.into_inner();

    let Some(mut problems) = store::read_json(client.clone()).await else {
        return HttpResponse::NotFound().finish();
    };

    let Some(problem) = problems.iter_mut().find(|p| p.id.to_string() == id) else {
        return HttpResponse::NotFound().finish();
    };

    problem.title = body.title.clone();
    problem.url = body.url.clone();
    problem.tags = body.tags.clone();
    problem.difficulty = body.difficulty;

    let updated = problem.clone();

    if store::write_json(client, &problems).await.is_none() {
        return HttpResponse::InternalServerError().finish();
    }

    HttpResponse::Ok().json(updated)
}

#[get("/problems/check-duplicate")]
pub async fn check_duplicate(
    client: web::Data<Client>,
    query: web::Query<CheckDuplicateRequest>,
) -> impl Responder {
    let Some(problems) = store::read_json(client.clone()).await else {
        return HttpResponse::NotFound().finish();
    };

    let input_url = query.url.trim_end_matches('/');

    let exists = problems
        .iter()
        .any(|p| p.url.trim_end_matches('/') == input_url);

    HttpResponse::Ok().json(CheckDuplicateResponse { exists })
}
