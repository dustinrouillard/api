use actix_web::{
  delete, get,
  http::Error,
  patch, post,
  web::{self},
  HttpResponse,
};
use chrono::{NaiveDateTime, Utc};
use serde_json::json;

use crate::{
  structs::blog::{BlogPost, BlogPostMutate, BlogPostsQuery},
  ServerState,
};

#[get("/posts")]
async fn get_posts(
  query: Option<web::Query<BlogPostsQuery>>,
  state: web::Data<ServerState>,
) -> Result<HttpResponse, Error> {
  let query: web::Query<BlogPostsQuery> =
    query.unwrap_or(actix_web::web::Query(BlogPostsQuery {
      limit: Some(25),
      offset: Some(0),
    }));

  let posts = sqlx::query_as::<_, BlogPost>(
    "SELECT * FROM blog_posts WHERE visibility = $1 \
     ORDER BY published_at DESC LIMIT $2 OFFSET $3",
  )
  .bind("public")
  .bind(query.limit.unwrap_or(25))
  .bind(query.offset.unwrap_or(0))
  .fetch_all(&state.db)
  .await;

  let posts: Vec<serde_json::Value> = posts
    .unwrap()
    .iter()
    .map(|post| {
      json!({
        "id": post.id,
        "slug": post.slug,
        "title": post.title,
        "description": post.description,
        "image": post.image,
        "visibility": post.visibility,
        "body": post.body,
        "tags": post.tags,
        "published_at": post.published_at,
      })
    })
    .collect();

  Ok(HttpResponse::Ok().json(json!({"posts": posts})))
}

#[get("/posts")]
async fn get_all_posts(
  query: Option<web::Query<BlogPostsQuery>>,
  state: web::Data<ServerState>,
) -> Result<HttpResponse, Error> {
  let query: web::Query<BlogPostsQuery> =
    query.unwrap_or(actix_web::web::Query(BlogPostsQuery {
      limit: Some(25),
      offset: Some(0),
    }));

  let posts = sqlx::query_as::<_, BlogPost>(
    "SELECT * FROM blog_posts ORDER BY created_at DESC LIMIT $1 OFFSET $2",
  )
  .bind(query.limit.unwrap_or(25))
  .bind(query.offset.unwrap_or(0))
  .fetch_all(&state.db)
  .await;

  let posts: Vec<serde_json::Value> = posts
    .unwrap()
    .iter()
    .map(|post| {
      json!({
        "id": post.id,
        "slug": post.slug,
        "title": post.title,
        "description": post.description,
        "image": post.image,
        "visibility": post.visibility,
        "body": post.body,
        "tags": post.tags,
        "created_at": post.created_at,
        "published_at": post.published_at,
      })
    })
    .collect();

  Ok(HttpResponse::Ok().json(json!({"posts": posts})))
}

#[post("/posts")]
async fn create_post(
  body: Option<web::Json<BlogPostMutate>>,
  state: web::Data<ServerState>,
) -> Result<HttpResponse, Error> {
  let body = body.unwrap_or(actix_web::web::Json(BlogPostMutate {
    slug: None,
    title: None,
    description: None,
    image: None,
    visibility: None,
    tags: None,
    body: None,
  }));

  // Omitted columns fall back to the database-side defaults (id_generator(),
  // date_title(), date_slug(), 'draft', '{}').
  let post = sqlx::query_as::<_, BlogPost>(
    "INSERT INTO blog_posts (title, slug, visibility, tags, description, body) \
     VALUES (\
       COALESCE($1, date_title()), \
       COALESCE($2, date_slug()), \
       COALESCE($3, 'draft'), \
       COALESCE($4, '{}'::text[]), \
       $5, $6\
     ) RETURNING *",
  )
  .bind(body.title.clone())
  .bind(body.slug.clone())
  .bind(body.visibility.clone())
  .bind(body.tags.clone())
  .bind(body.description.clone())
  .bind(body.body.clone())
  .fetch_one(&state.db)
  .await;

  match post {
    Ok(post) => Ok(HttpResponse::Created().json(json!({
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
    }))),
    Err(_) => Ok(
      HttpResponse::InternalServerError()
        .json(json!({"code": "uncaught_error_creating_post"})),
    ),
  }
}

#[get("/posts/{id_or_slug}")]
async fn get_post(
  id_or_slug: web::Path<String>,
  state: web::Data<ServerState>,
) -> Result<HttpResponse, Error> {
  let query = if id_or_slug.parse::<f64>().is_ok() {
    "SELECT * FROM blog_posts WHERE id = $1 \
     AND visibility IN ('public', 'unlisted') LIMIT 1"
  } else {
    "SELECT * FROM blog_posts WHERE slug = $1 \
     AND visibility IN ('public', 'unlisted') LIMIT 1"
  };

  let post = sqlx::query_as::<_, BlogPost>(query)
    .bind(id_or_slug.to_string())
    .fetch_optional(&state.db)
    .await;

  match post {
    Ok(Some(post)) => Ok(HttpResponse::Ok().json(json!({
        "post": {
            "id": post.id,
            "title": post.title,
            "slug": post.slug,
            "description": post.description,
            "image": post.image,
            "visibility": post.visibility,
            "tags": post.tags,
            "body": post.body,
            "published_at": post.published_at,
        }
    }))),
    Ok(None) => Ok(
      HttpResponse::NotFound().json(json!({"code": "post_not_found"})),
    ),
    Err(_) => Ok(
      HttpResponse::NotFound().json(json!({"code": "post_not_found"})),
    ),
  }
}

#[patch("/posts/{id}")]
async fn update_post(
  id: web::Path<String>,
  state: web::Data<ServerState>,
  body: web::Json<BlogPostMutate>,
) -> Result<HttpResponse, Error> {
  let existing = sqlx::query_as::<_, BlogPost>(
    "SELECT * FROM blog_posts WHERE id = $1 LIMIT 1",
  )
  .bind(id.to_string())
  .fetch_optional(&state.db)
  .await;

  let post = match existing {
    Ok(Some(post)) => post,
    Ok(None) => {
      return Ok(
        HttpResponse::NotFound().json(json!({"code": "post_not_found"})),
      );
    }
    Err(_) => {
      return Ok(
        HttpResponse::Unauthorized()
          .json(json!({"code": "post_not_found"})),
      );
    }
  };

  let intended_visibility =
    body.visibility.clone().unwrap_or(post.visibility.clone());

  // If the post is being made public for the first time, stamp published_at.
  let mut published_at: Option<NaiveDateTime> = post.published_at;
  if post.published_at.is_none() && intended_visibility == "public" {
    published_at = Some(Utc::now().naive_utc());
  }

  // Resolve each column to its new-or-existing value (only `description` and
  // `body` are set when provided; the rest fall back to the current row).
  let title = body.title.clone().unwrap_or(post.title);
  let slug = body.slug.clone().unwrap_or(post.slug);
  let tags = body.tags.clone().unwrap_or(post.tags);
  let description = body.description.clone().or(post.description);
  let body_text = body.body.clone().or(post.body);

  let updated = sqlx::query_as::<_, BlogPost>(
    "UPDATE blog_posts SET \
       title = $1, slug = $2, visibility = $3, tags = $4, \
       description = $5, body = $6, published_at = $7 \
     WHERE id = $8 RETURNING *",
  )
  .bind(title)
  .bind(slug)
  .bind(intended_visibility)
  .bind(tags)
  .bind(description)
  .bind(body_text)
  .bind(published_at)
  .bind(post.id)
  .fetch_one(&state.db)
  .await;

  match updated {
    Err(_) => Ok(
      HttpResponse::NotFound().json(json!({"code": "post_not_found"})),
    ),
    Ok(post) => Ok(HttpResponse::Ok().json(json!({
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
    }))),
  }
}

#[delete("/posts/{id}")]
async fn delete_post(
  id: web::Path<String>,
  state: web::Data<ServerState>,
) -> Result<HttpResponse, Error> {
  let result =
    sqlx::query("UPDATE blog_posts SET visibility = 'deleted' WHERE id = $1")
      .bind(id.to_string())
      .execute(&state.db)
      .await;

  match result {
    Ok(result) if result.rows_affected() > 0 => {
      Ok(HttpResponse::NoContent().finish())
    }
    _ => Ok(
      HttpResponse::NotFound().json(json!({"code": "post_not_found"})),
    ),
  }
}
