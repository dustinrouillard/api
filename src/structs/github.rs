use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct ContributionsData {
  pub viewer: ViewerWithContributions,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ViewerWithContributions {
  pub contributions_collection: ContributionsCollectionClass,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContributionsCollectionClass {
  pub contribution_calendar: ContributionCalendar,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContributionCalendar {
  pub total_contributions: i64,
  pub weeks: Vec<Week>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Week {
  pub contribution_days: Vec<ContributionDay>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContributionDay {
  pub contribution_count: i64,
  pub date: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GithubPinned {
  pub user: UserWithPins,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UserWithPins {
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
