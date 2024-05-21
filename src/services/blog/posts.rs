use actix_web::{delete, get, http::Error, patch, post, web, HttpResponse};
use serde_json::json;

use crate::{structs::blog::BlogPost, ServerState};

#[get("/posts")]
async fn get_posts() -> Result<HttpResponse, Error> {
    Ok(HttpResponse::NotImplemented().finish())
}

#[post("/posts")]
async fn create_post(state: web::Data<ServerState>) -> Result<HttpResponse, Error> {
    let postgres = &mut &state.postgres;

    let post_record = sqlx::query_as::<_, BlogPost>(
        "INSERT INTO blog_posts DEFAULT VALUES RETURNING id, slug, title, description, visibility, created_at",
    )
    .fetch_one(&postgres.pool)
    .await;

    match post_record {
        Ok(post) => Ok(HttpResponse::Created()
            .append_header(("Content-type", "application/json"))
            .body(
                json!({
                    "post": {
                        "id": post.id,
                        "title": post.title,
                        "slug": post.slug,
                        "description": post.description,
                        "image": post.image,
                        "visibility": post.visibility,
                        "tags": post.tags,
                        "body": post.body,
                        "created_at": post.created_at,
                        "published_at": post.published_at,
                    }
                })
                .to_string(),
            )),
        Err(_) => {
            return Ok(HttpResponse::Unauthorized()
                .append_header(("Content-type", "application/json"))
                .body(json!({"code": "uncaught_error_creating_post"}).to_string()));
        }
    }
}

#[get("/posts/{id}")]
async fn get_post() -> Result<HttpResponse, Error> {
    Ok(HttpResponse::NotImplemented().finish())
}

#[patch("/posts/{id}")]
async fn update_post() -> Result<HttpResponse, Error> {
    Ok(HttpResponse::NotImplemented().finish())
}

#[delete("/posts/{id}")]
async fn delete_post() -> Result<HttpResponse, Error> {
    Ok(HttpResponse::NotImplemented().finish())
}
