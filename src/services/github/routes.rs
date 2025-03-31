use std::collections::HashMap;

use actix_web::{get, http::Error, web, HttpResponse};
use envconfig::Envconfig;
use gql_client::Client;
use redis::{aio::ConnectionManager, AsyncCommands, RedisError};
use serde_json::json;

use crate::{
  config::Config,
  structs::github::{ContributionsData, GithubPinned},
  ServerState,
};

#[get("/pinned")]
async fn github_pinned(
  state: web::Data<ServerState>,
) -> Result<HttpResponse, Error> {
  let query = "
    query GithubUserPins {
      user(login: \"dustinrouillard\") {
        pinnedItems(first: 6, types: [REPOSITORY]) {
          totalCount
          edges {
            node {
              ... on Repository {
                owner {
                  login
                }
                name
                description
                stargazerCount
                forkCount
                primaryLanguage {
                  name
                  color
                }
                pushedAt
                url
              }
            }
          }
        }
      }
    }
  ";

  let valkey = &mut state.valkey.clone();

  let cached = redis::cmd("GET")
    .arg("cache/github/pinned")
    .query_async::<ConnectionManager, String>(&mut valkey.cm)
    .await;

  let response: Vec<serde_json::Value> = if let Err(_) = cached {
    let config = Config::init_from_env().unwrap();

    let mut headers = HashMap::new();
    headers.insert("user-agent", "rest.dstn.to/2.0".to_string());
    headers
      .insert("authorization", format!("token {}", config.github_pat));

    let client =
      Client::new_with_headers("https://api.github.com/graphql", headers);

    let data = client.query::<GithubPinned>(query).await.unwrap();

    let response = data
      .unwrap()
      .user
      .pinned_items
      .edges
      .iter()
      .map(|edge| {
        json!({
          "owner": edge.node.owner.login,
          "name": edge.node.name,
          "description": edge.node.description,
          "stars": edge.node.stargazer_count,
          "forks": edge.node.fork_count,
          "language": edge.node.primary_language,
          "pushed_at": edge.node.pushed_at,
          "url": edge.node.url,
        })
      })
      .collect();

    let _: Result<String, RedisError> = valkey
      .cm
      .set_ex("cache/github/pinned", json!(response).to_string(), 1800)
      .await;

    response
  } else {
    serde_json::from_str(&cached.unwrap()).unwrap()
  };

  Ok(HttpResponse::Ok().json(json!({"repositories": response})))
}

#[get("/contributions")]
async fn github_contributions(
  state: web::Data<ServerState>,
) -> Result<HttpResponse, Error> {
  let query = "
    query GithubContributions {
      viewer {
        contributionsCollection {
          contributionCalendar {
            totalContributions
            weeks {
              contributionDays {
                contributionCount
                date
              }
            }
          }
        }
      }
    }
  ";

  let valkey = &mut state.valkey.clone();

  let cached = redis::cmd("GET")
    .arg("cache/github/contributions")
    .query_async::<ConnectionManager, String>(&mut valkey.cm)
    .await;

  let response: serde_json::Value = if let Err(_) = cached {
    let config = Config::init_from_env().unwrap();

    let mut headers = HashMap::new();
    headers.insert("user-agent", "rest.dstn.to/2.0".to_string());
    headers
      .insert("authorization", format!("token {}", config.github_pat));

    let client =
      Client::new_with_headers("https://api.github.com/graphql", headers);

    let data = client
      .query::<ContributionsData>(query)
      .await
      .unwrap()
      .unwrap();
    let graph = data
      .viewer
      .contributions_collection
      .contribution_calendar
      .weeks
      .into_iter()
      .map(|week| {
        week
          .contribution_days
          .into_iter()
          .map(|day| {
            json!({
              "count": day.contribution_count,
              "date": day.date,
            })
          })
          .collect::<Vec<_>>()
      })
      .collect::<Vec<_>>();

    let response = json!({
      "graph": graph,
      "total_contributions": data.viewer.contributions_collection.contribution_calendar.total_contributions,
    });

    let _: Result<String, RedisError> = valkey
      .cm
      .set_ex("cache/github/contributions", response.to_string(), 1800)
      .await;

    response
  } else {
    serde_json::from_str(&cached.unwrap()).unwrap()
  };

  Ok(HttpResponse::Ok().json(response))
}
