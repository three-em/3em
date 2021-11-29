use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GQLPageInfoInterface {
  pub has_next_page: bool,
}

#[derive(Deserialize, Debug, Clone)]
pub struct GQLOwnerInterface {
  pub address: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  #[serde(default)]
  pub key: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct GQLAmountInterface {
  pub winston: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  #[serde(default)]
  pub ar: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct GQLMetaDataInterface {
  pub size: usize,
  #[serde(rename = "type")]
  pub ty: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct GQLTagInterface {
  pub name: String,
  pub value: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct GQLBlockInterface {
  pub id: String,
  pub timestamp: usize,
  pub height: usize,
  #[serde(skip_serializing_if = "Option::is_none")]
  #[serde(default)]
  pub previous: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct GQLNodeParent {
  pub id: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct GQLNodeInterface {
  pub id: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  #[serde(default)]
  pub anchor: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  #[serde(default)]
  pub signature: Option<String>,
  pub recipient: String,
  pub owner: GQLOwnerInterface,
  pub fee: GQLAmountInterface,
  pub quantity: GQLAmountInterface,
  #[serde(skip_serializing_if = "Option::is_none")]
  #[serde(default)]
  pub data: Option<GQLMetaDataInterface>,
  pub tags: Vec<GQLTagInterface>,
  pub block: GQLBlockInterface,
  pub parent: Option<GQLNodeParent>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct GQLEdgeInterface {
  pub cursor: String,
  pub node: GQLNodeInterface,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GQLTransactionsResultInterface {
  pub page_info: GQLPageInfoInterface,
  pub edges: Vec<GQLEdgeInterface>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct GQLDataResultInterface {
  pub transactions: GQLTransactionsResultInterface,
}

#[derive(Deserialize, Debug, Clone)]
pub struct GQLResultInterface {
  pub data: GQLDataResultInterface,
}
