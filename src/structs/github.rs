use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Data {
  pub user: User,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct User {
  pub pinned_items: PinnedItems,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PinnedItems {
  pub total_count: i64,
  pub edges: Vec<Edge>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Edge {
  pub node: Node,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Node {
  pub owner: Owner,
  pub name: String,
  pub description: String,
  pub stargazer_count: i64,
  pub fork_count: i64,
  pub primary_language: Option<PrimaryLanguage>,
  pub pushed_at: String,
  pub url: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Owner {
  pub login: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PrimaryLanguage {
  pub name: String,
  pub color: String,
}
