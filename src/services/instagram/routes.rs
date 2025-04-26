use actix_web::{get, http::Error, web, HttpResponse};
use redis::AsyncCommands;
use serde_json::json;

use crate::{
  structs::instagram::{InstagramMe, InstagramOverview},
  ServerState,
};

#[get("/overview")]
pub async fn get_overview(
  state: web::Data<ServerState>,
) -> Result<HttpResponse, Error> {
  let redis = &mut state.valkey.clone();
  let cached = redis.cm.get::<_, String>("instagram/overview").await;
  if let Ok(overview) = cached {
    return Ok(HttpResponse::Ok().json(
      serde_json::from_str::<InstagramOverview>(&overview).unwrap(),
    ));
  }

  let access_token =
    redis.cm.get::<_, String>("instagram/access_token").await;
  if let Ok(token) = access_token {
    let client = reqwest::Client::new();
    let url = format!(
        "https://graph.instagram.com/me?fields=media_count,follows_count,media{{id,caption,media_type,media_product_type,comments_count,like_count,media_url,thumbnail_url,permalink,timestamp}}&access_token={}",
        token
    );

    let res = client.get(&url).send().await;
    if res.is_err() {
      eprintln!("failed to fetch instagram graph {:?}", res.unwrap_err());
      return Ok(
        HttpResponse::Ok()
          .json(json!({ "error": "failed_to_fetch_ig_graph" })),
      );
    }
    let res = res.unwrap();
    let instagram_me = res.json().await;
    if instagram_me.is_err() {
      eprintln!(
        "failed to fetch instagram graph {:?}",
        instagram_me.unwrap_err()
      );
      return Ok(
        HttpResponse::Ok()
          .json(json!({ "error": "failed_to_fetch_ig_graph" })),
      );
    }

    let instagram_me: InstagramMe = instagram_me.unwrap();
    let overview = InstagramOverview::from(instagram_me);

    redis
      .cm
      .set_ex::<_, _, i32>(
        "instagram/overview",
        serde_json::to_string::<InstagramOverview>(&overview).unwrap(),
        300,
      )
      .await
      .ok();

    Ok(HttpResponse::Ok().json(overview))
  } else {
    Ok(
      HttpResponse::Ok()
        .json(json!({ "error": "failed_to_fetch_ig_graph" })),
    )
  }
}
