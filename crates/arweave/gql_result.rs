use serde::Deserialize;
use serde::Serialize;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GQLPageInfoInterface {
  pub has_next_page: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GQLOwnerInterface {
  pub address: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  #[serde(default)]
  pub key: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GQLAmountInterface {
  #[serde(skip_serializing_if = "Option::is_none")]
  #[serde(default)]
  pub winston: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  #[serde(default)]
  pub ar: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GQLMetaDataInterface {
  #[serde(skip_serializing_if = "Option::is_none")]
  #[serde(default)]
  pub size: Option<usize>,
  #[serde(rename = "type")]
  #[serde(skip_serializing_if = "Option::is_none")]
  #[serde(default)]
  pub ty: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GQLTagInterface {
  pub name: String,
  pub value: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GQLBlockInterface {
  pub id: String,
  pub timestamp: usize,
  pub height: usize,
  #[serde(skip_serializing_if = "Option::is_none")]
  #[serde(default)]
  pub previous: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GQLNodeParent {
  pub id: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GQLBundled {
  pub id: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GQLNodeInterface {
  pub id: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  #[serde(default)]
  pub anchor: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  #[serde(default)]
  pub signature: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  #[serde(default)]
  pub recipient: Option<String>,
  pub owner: GQLOwnerInterface,
  #[serde(skip_serializing_if = "Option::is_none")]
  #[serde(default)]
  pub fee: Option<GQLAmountInterface>,
  #[serde(skip_serializing_if = "Option::is_none")]
  #[serde(default)]
  pub quantity: Option<GQLAmountInterface>,
  #[serde(skip_serializing_if = "Option::is_none")]
  #[serde(default)]
  pub data: Option<GQLMetaDataInterface>,
  pub tags: Vec<GQLTagInterface>,
  pub block: GQLBlockInterface,
  #[serde(skip_serializing_if = "Option::is_none")]
  #[serde(default)]
  pub parent: Option<GQLNodeParent>,
  #[serde(skip_serializing_if = "Option::is_none")]
  #[serde(default)]
  pub bundledIn: Option<GQLBundled>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GQLEdgeInterface {
  pub cursor: String,
  pub node: GQLNodeInterface,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GQLTransactionsResultInterface {
  pub page_info: GQLPageInfoInterface,
  pub edges: Vec<GQLEdgeInterface>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GQLDataResultInterface {
  pub transactions: GQLTransactionsResultInterface,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GQLResultInterface {
  pub data: GQLDataResultInterface,
}
