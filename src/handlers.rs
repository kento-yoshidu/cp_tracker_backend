use std::collections::HashMap;

use actix_web::{
    HttpResponse,
    Responder,
    web,
    get,
    post,
    put,
    delete,
};
use aws_sdk_s3::Client;
use uuid::Uuid;

use crate::{
    models::{
        Activity, CheckDuplicateRequest, CheckDuplicateResponse, CreateProblemRequest, DailyActivity, DataFile, Problem, UpdateProblemRequest
    }, store,
};

#[get("/problems")]
async fn get_problems(client: web::Data<Client>) -> impl Responder {
    match store::read_json::<Problem>(client, DataFile::Problems).await {
        Some(mut problems) => {
            problems.sort_by(|a, b| b.created_at.cmp(&a.created_at));

            HttpResponse::Ok()
                .content_type("application/json")
                .body(serde_json::to_string(&problems).unwrap())
        },
        None => HttpResponse::InternalServerError().finish(),
    }
}

#[get("/activities")]
async fn get_activities(client: web::Data<Client>) -> impl Responder {
    let Some(activities) = store::read_json::<Activity>(client, DataFile::Activities).await else {
        return HttpResponse::InternalServerError().finish();
    };

    let mut map = HashMap::new();

    for activity in activities {
        let entry = map.entry(activity.date).or_insert((0, 0));
        entry.0 += 1;
        entry.1 = entry.1.max(activity.difficulty);
    }

    let mut daily: Vec<DailyActivity> = map
        .into_iter()
        .map(|(date, (ac_count, max_difficulty))| DailyActivity { date, ac_count, max_difficulty })
        .collect();

    daily.sort_by(|a, b| a.date.cmp(&b.date));

    HttpResponse::Ok().json(daily)
}

#[post("/problems/{id}/ac")]
pub async fn post_ac(
    client: web::Data<Client>,
    path: web::Path<String>
) -> impl Responder {
    let id = path.into_inner();

    let Some(mut problems) = store::read_json::<Problem>(client.clone(), DataFile::Problems).await else {
        return HttpResponse::NotFound().finish();
    };

    let Some(problem) = problems.iter_mut().find(|p| p.id.to_string() == id) else {
        return HttpResponse::NotFound().finish();
    };

    let today = chrono::Local::now().format("%Y%m%d").to_string();

    problem.ac_count += 1;
    problem.last_solved_at = Some(today.clone());

    let updated = problem.clone();

    if store::write_json(client.clone(), DataFile::Problems, &problems).await.is_none() {
        return HttpResponse::InternalServerError().finish();
    }

    let Some(mut activities) = store::read_json::<Activity>(client.clone(), DataFile::Activities).await else {
        return HttpResponse::InternalServerError().finish();
    };

    activities.push(Activity {
        problem_id: updated.id,
        difficulty: updated.difficulty,
        date: today,
    });

    if store::write_json(client, DataFile::Activities, &activities).await.is_none() {
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

    let Some(mut problems) = store::read_json(client.clone(), DataFile::Problems).await else {
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

    if store::write_json(client, DataFile::Problems, &problems).await.is_none() {
        return  HttpResponse::InternalServerError().finish();
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

    let Some(mut problems) = store::read_json::<Problem>(client.clone(), DataFile::Problems).await else {
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

    if store::write_json(client, DataFile::Problems, &problems).await.is_none() {
        return HttpResponse::InternalServerError().finish();
    }

    HttpResponse::Ok().json(updated)
}

#[delete("/problems/{id}")]
pub async fn delete_problem(
    client: web::Data<Client>,
    path: web::Path<uuid::Uuid>,
) -> impl Responder {
    let path_id = path.into_inner();

    let Some(problems) = store::read_json::<Problem>(client.clone(), DataFile::Problems).await else {
        return HttpResponse::NotFound().finish();
    };

    if !problems.iter().any(|problem| problem.id == path_id) {
        return HttpResponse::NotFound().finish();
    }

    let new_problems: Vec<Problem> = problems
        .into_iter()
        .filter(|problem| problem.id != path_id)
        .collect();

    if store::write_json::<Problem>(client, DataFile::Problems, &new_problems).await.is_none() {
        return HttpResponse::InternalServerError().finish();
    }

    HttpResponse::Ok().finish()
}

#[get("/problems/check-duplicate")]
pub async fn check_duplicate(
    client: web::Data<Client>,
    query: web::Query<CheckDuplicateRequest>,
) -> impl Responder {
    let Some(problems) = store::read_json::<Problem>(client.clone(), DataFile::Problems).await else {
        return HttpResponse::NotFound().finish();
    };

    let input_url = query.url.trim_end_matches('/');

    let exists = problems
        .iter()
        .any(|p| p.url.trim_end_matches('/') == input_url);

    HttpResponse::Ok().json(CheckDuplicateResponse { exists })
}
