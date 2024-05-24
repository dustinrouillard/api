use std::vec;

use actix_web::{
  delete, get,
  http::Error,
  patch, post,
  web::{self},
  HttpResponse,
};
use prisma_client_rust::Direction;
use serde_json::json;

use crate::{
  connectivity::prisma::blog_posts,
  structs::blog::{BlogPostMutate, BlogPostsQuery},
  ServerState,
};

#[get("/posts")]
async fn get_posts(
  query: Option<web::Query<BlogPostsQuery>>,
  state: web::Data<ServerState>,
) -> Result<HttpResponse, Error> {
  let prisma = &mut &state.prisma;

  let query: web::Query<BlogPostsQuery> =
    query.unwrap_or(actix_web::web::Query(BlogPostsQuery {
      limit: Some(25),
      offset: Some(0),
    }));

  let posts = prisma
    .blog_posts()
    .find_many(vec![blog_posts::visibility::equals("public".to_string())])
    .take(query.limit.unwrap_or(25))
    .skip(query.offset.unwrap_or(0))
    .order_by(blog_posts::published_at::order(Direction::Desc))
    .exec()
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

  Ok(
    HttpResponse::Ok()
      .append_header(("Content-type", "application/json"))
      .body(json!({"posts": posts}).to_string()),
  )
}

#[get("/posts")]
async fn get_all_posts(
  query: Option<web::Query<BlogPostsQuery>>,
  state: web::Data<ServerState>,
) -> Result<HttpResponse, Error> {
  let prisma = &mut &state.prisma;

  let query: web::Query<BlogPostsQuery> =
    query.unwrap_or(actix_web::web::Query(BlogPostsQuery {
      limit: Some(25),
      offset: Some(0),
    }));

  let posts = prisma
    .blog_posts()
    .find_many(vec![])
    .take(query.limit.unwrap_or(25))
    .skip(query.offset.unwrap_or(0))
    .order_by(blog_posts::created_at::order(Direction::Desc))
    .exec()
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

  Ok(
    HttpResponse::Ok()
      .append_header(("Content-type", "application/json"))
      .body(json!({"posts": posts}).to_string()),
  )
}

#[post("/posts")]
async fn create_post(
  body: Option<web::Json<BlogPostMutate>>,
  state: web::Data<ServerState>,
) -> Result<HttpResponse, Error> {
  let prisma = &mut &state.prisma;

  // Trigger public post stuff if the updated visibility is public and there is no published_at before hand.

  let body = body.unwrap_or(actix_web::web::Json(BlogPostMutate {
    slug: None,
    title: None,
    description: None,
    image: None,
    visibility: None,
    tags: None,
    body: None,
  }));

  let post_params: Vec<blog_posts::SetParam> = vec![
    body.title.clone().map(blog_posts::title::set),
    body.slug.clone().map(blog_posts::slug::set),
    body.visibility.clone().map(blog_posts::visibility::set),
    body.tags.clone().map(blog_posts::tags::set),
    body.description.clone().map(|value: std::string::String| {
      blog_posts::description::set(Some(value))
    }),
    body.body.clone().map(|value: std::string::String| {
      blog_posts::body::set(Some(value))
    }),
  ]
  .into_iter()
  .flatten()
  .collect();

  let post = prisma.blog_posts().create(post_params).exec().await;

  match post {
    Ok(post) => Ok(
      HttpResponse::Created()
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
        ),
    ),
    Err(_) => {
      return Ok(
        HttpResponse::InternalServerError()
          .append_header(("Content-type", "application/json"))
          .body(
            json!({"code": "uncaught_error_creating_post"}).to_string(),
          ),
      );
    }
  }
}

#[get("/posts/{id}")]
async fn get_post(
  id: web::Path<String>,
  state: web::Data<ServerState>,
) -> Result<HttpResponse, Error> {
  let prisma = &mut &state.prisma;
  let post = prisma
    .blog_posts()
    .find_first(vec![blog_posts::id::equals(id.to_string())])
    .exec()
    .await;

  match post {
    Ok(post) => match post {
      Some(post) => Ok(
        HttpResponse::Ok()
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
                    "published_at": post.published_at,
                }
            })
            .to_string(),
          ),
      ),
      None => Ok(
        HttpResponse::NotFound()
          .append_header(("Content-type", "application/json"))
          .body(json!({"code": "post_not_found"}).to_string()),
      ),
    },
    Err(_) => {
      return Ok(
        HttpResponse::NotFound()
          .append_header(("Content-type", "application/json"))
          .body(json!({"code": "post_not_found"}).to_string()),
      );
    }
  }
}

#[patch("/posts/{id}")]
async fn update_post(
  id: web::Path<String>,
  state: web::Data<ServerState>,
  body: web::Json<BlogPostMutate>,
) -> Result<HttpResponse, Error> {
  let prisma = &mut &state.prisma;
  let post = prisma
    .blog_posts()
    .find_first(vec![blog_posts::id::equals(id.to_string())])
    .exec()
    .await;

  match post {
    Ok(post) => {
      let post = match post {
        Some(post) => post,
        None => {
          return Ok(
            HttpResponse::NotFound()
              .append_header(("Content-type", "application/json"))
              .body(json!({"code": "post_not_found"}).to_string()),
          );
        }
      };

      let body = body.clone();

      let post_params: Vec<blog_posts::SetParam> = vec![
        body.title.clone().map(blog_posts::title::set),
        body.slug.clone().map(blog_posts::slug::set),
        body.visibility.clone().map(blog_posts::visibility::set),
        body.tags.clone().map(blog_posts::tags::set),
        body.description.clone().map(|value: std::string::String| {
          blog_posts::description::set(Some(value))
        }),
        body.body.clone().map(|value: std::string::String| {
          blog_posts::body::set(Some(value))
        }),
      ]
      .into_iter()
      .flatten()
      .collect();

      let post_update = prisma
        .blog_posts()
        .update(blog_posts::id::equals(post.id), post_params)
        .exec()
        .await;

      match post_update {
        Err(_) => Ok(
          HttpResponse::NotFound()
            .append_header(("Content-type", "application/json"))
            .body(json!({"code": "post_not_found"}).to_string()),
        ),
        Ok(post) => Ok(
          HttpResponse::Ok()
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
            ),
        ),
      }
    }
    Err(_) => Ok(
      HttpResponse::Unauthorized()
        .append_header(("Content-type", "application/json"))
        .body(json!({"code": "post_not_found"}).to_string()),
    ),
  }
}

#[delete("/posts/{id}")]
async fn delete_post(
  id: web::Path<String>,
  state: web::Data<ServerState>,
) -> Result<HttpResponse, Error> {
  let prisma = &mut &state.prisma;
  let post = prisma
    .blog_posts()
    .find_first(vec![blog_posts::id::equals(id.to_string())])
    .exec()
    .await;

  match post {
    Ok(post) => {
      let post = match post {
        Some(post) => post,
        None => {
          return Ok(
            HttpResponse::NotFound()
              .append_header(("Content-type", "application/json"))
              .body(json!({"code": "post_not_found"}).to_string()),
          );
        }
      };

      let _ = prisma
        .blog_posts()
        .update(
          blog_posts::id::equals(post.id),
          vec![blog_posts::visibility::set("deleted".to_string())],
        )
        .exec()
        .await;

      Ok(HttpResponse::NoContent().finish())
    }
    Err(_) => {
      return Ok(
        HttpResponse::NotFound()
          .append_header(("Content-type", "application/json"))
          .body(json!({"code": "post_not_found"}).to_string()),
      );
    }
  }
}
