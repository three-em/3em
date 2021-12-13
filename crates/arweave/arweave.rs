use crate::cache::ArweaveCache;
use crate::gql_result::GQLEdgeInterface;
use crate::gql_result::GQLNodeParent;
use crate::gql_result::GQLResultInterface;
use crate::gql_result::GQLTransactionsResultInterface;
use crate::miscellaneous::get_contract_type;
use crate::miscellaneous::ContractType;
use crate::utils::decode_base_64;
use deno_core::error::AnyError;
use deno_core::futures::stream;
use deno_core::futures::StreamExt;
use lazy_static::lazy_static;
use reqwest::Client;
use serde::Deserialize;
use serde::Serialize;

#[derive(Deserialize, Serialize, Clone)]
pub struct NetworkInfo {
  pub network: String,
  pub version: usize,
  pub release: usize,
  pub height: usize,
  pub current: String,
  pub blocks: usize,
  pub peers: usize,
  pub queue_length: usize,
  pub node_state_latency: usize,
}

#[derive(Deserialize, Serialize, Default, Clone, Debug)]
pub struct Tag {
  pub name: String,
  pub value: String,
}

#[derive(Deserialize, Serialize, Default, Clone)]
pub struct TransactionData {
  pub format: usize,
  pub id: String,
  pub last_tx: String,
  pub owner: String,
  pub tags: Vec<Tag>,
  pub target: String,
  pub quantity: String,
  pub data: String,
  pub reward: String,
  pub signature: String,
  pub data_size: String,
  pub data_root: String,
}

impl TransactionData {
  pub fn get_tag(&self, tag: &str) -> Result<String, AnyError> {
    // Encodes the tag instead of decoding the keys.
    let encoded_tag = base64::encode_config(tag, base64::URL_SAFE_NO_PAD);
    self
      .tags
      .iter()
      .find(|t| t.name == encoded_tag)
      .map(|t| Ok(String::from_utf8(base64::decode(&t.value)?)?))
      .ok_or_else(|| AnyError::msg(format!("{} tag not found", tag)))?
  }
}

#[derive(Clone)]
pub enum ArweaveProtocol {
  HTTP,
  HTTPS,
}

#[derive(Clone)]
pub struct Arweave {
  host: String,
  port: i32,
  protocol: ArweaveProtocol,
  client: Client,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct TagFilter {
  name: String,
  values: Vec<String>,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct BlockFilter {
  max: usize,
}

#[derive(Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct InteractionVariables {
  tags: Vec<TagFilter>,
  block_filter: BlockFilter,
  first: usize,
  #[serde(skip_serializing_if = "Option::is_none")]
  #[serde(default)]
  after: Option<String>,
}

#[derive(Deserialize, Serialize)]
pub struct GraphqlQuery {
  query: String,
  variables: InteractionVariables,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct LoadedContract {
  pub id: String,
  pub contract_src_tx_id: String,
  pub contract_src: Vec<u8>,
  pub contract_type: ContractType,
  pub init_state: String,
  pub min_fee: Option<String>,
  pub contract_transaction: TransactionData,
}

enum State {
  Next(Option<String>, InteractionVariables),
  #[allow(dead_code)]
  End,
}

pub static MAX_REQUEST: usize = 100;

lazy_static! {
  pub static ref ARWEAVE_CACHE: ArweaveCache = ArweaveCache::new();
}

impl Arweave {
  pub fn new(port: i32, host: String) -> Arweave {
    Arweave {
      port,
      host,
      protocol: ArweaveProtocol::HTTPS,
      client: Client::new(),
    }
  }

  pub async fn get_transaction(
    &self,
    transaction_id: &str,
  ) -> reqwest::Result<TransactionData> {
    let request = self
      .client
      .get(format!("{}/tx/{}", self.get_host(), transaction_id))
      .send()
      .await
      .unwrap();
    let transaction = request.json::<TransactionData>().await;
    transaction
  }

  pub async fn get_transaction_data(&self, transaction_id: &str) -> Vec<u8> {
    let request = self
      .client
      .get(format!("{}/{}", self.get_host(), transaction_id))
      .send()
      .await
      .unwrap();
    request.bytes().await.unwrap().to_vec()
  }

  pub async fn get_network_info(&self) -> NetworkInfo {
    let info = self
      .client
      .get(format!("{}/info", self.get_host()))
      .send()
      .await
      .unwrap()
      .json::<NetworkInfo>()
      .await
      .unwrap();
    info
  }

  pub async fn get_interactions(
    &self,
    contract_id: String,
    height: Option<usize>,
    cache: bool,
  ) -> Result<(Vec<GQLEdgeInterface>, usize, bool), AnyError> {
    let mut interactions: Option<Vec<GQLEdgeInterface>> = None;

    let height_result = match height {
      Some(size) => size,
      None => self.get_network_info().await.height,
    };

    let variables = self
      .get_default_gql_variables(contract_id.to_owned(), height_result)
      .await;

    if cache {
      if let Some(cache_interactions) = ARWEAVE_CACHE
        .find_interactions(contract_id.to_owned())
        .await
      {
        if !cache_interactions.is_empty() {
          interactions = Some(cache_interactions);
        }
      }
    }

    let mut final_result: Vec<GQLEdgeInterface> = Vec::new();
    let mut new_transactions = false;
    let mut new_interactions_index: usize = 0;

    if let Some(mut cache_interactions) = interactions {
      let last_transaction_edge = cache_interactions.last().unwrap();
      let has_more_from_last_interaction = self
        .has_more(&variables, last_transaction_edge.cursor.to_owned())
        .await?;

      if has_more_from_last_interaction {
        // Start from what's going to be the next interaction. if doing len - 1, that would mean we will also include the last interaction cached: not ideal.
        new_interactions_index = cache_interactions.len();
        let fetch_more_interactions = self
          .stream_interactions(
            Some(last_transaction_edge.cursor.to_owned()),
            variables.to_owned(),
          )
          .await;

        for result in fetch_more_interactions {
          let mut new_tx_infos = result.edges.clone();
          cache_interactions.append(&mut new_tx_infos);
        }
        new_transactions = true;
      }

      final_result.append(&mut cache_interactions);
    } else {
      let transactions = self
        .get_next_interaction_page(variables.clone(), false, None)
        .await?;

      let mut tx_infos = transactions.edges.clone();

      let mut cursor: Option<String> = None;
      let max_edge = self.get_max_edges(&transactions.edges);
      let maybe_edge = transactions.edges.get(max_edge);

      if let Some(data) = maybe_edge {
        let owned = data;
        cursor = Some(owned.cursor.to_owned());
      }

      let results = self.stream_interactions(cursor, variables).await;

      for result in results {
        let mut new_tx_infos = result.edges.clone();
        tx_infos.append(&mut new_tx_infos);
      }

      final_result.append(&mut tx_infos);
      new_transactions = true;
    }

    let to_return: Vec<GQLEdgeInterface>;

    if new_transactions {
      let filtered: Vec<GQLEdgeInterface> = final_result
        .into_iter()
        .filter(|p| {
          (p.node.parent.is_none())
            || p
              .node
              .parent
              .as_ref()
              .unwrap_or(&GQLNodeParent { id: None })
              .id
              .is_none()
        })
        .collect();

      if cache {
        ARWEAVE_CACHE
          .cache_interactions(contract_id, &filtered)
          .await;
      }

      to_return = filtered;
    } else {
      to_return = final_result;
    }

    let are_there_new_interactions = cache && new_transactions;
    Ok((
      to_return,
      new_interactions_index,
      are_there_new_interactions,
    ))
  }

  async fn get_next_interaction_page(
    &self,
    mut variables: InteractionVariables,
    from_last_page: bool,
    max_results: Option<usize>,
  ) -> Result<GQLTransactionsResultInterface, AnyError> {
    let mut query = String::from(
      r#"query Transactions($tags: [TagFilter!]!, $blockFilter: BlockFilter!, $first: Int!, $after: String) {
    transactions(tags: $tags, block: $blockFilter, first: $first, sort: HEIGHT_ASC, after: $after) {
      pageInfo {
        hasNextPage
      }
      edges {
        node {
          id
          owner { address }
          recipient
          tags {
            name
            value
          }
          block {
            height
            id
            timestamp
          }
          fee { winston }
          quantity { winston }
          parent { id }
        }
        cursor
      }
    }
  }"#,
    );

    if from_last_page {
      query = query.replace("HEIGHT_ASC", "HEIGHT_DESC");
      variables.first = max_results.unwrap_or(100);
    }

    let graphql_query = GraphqlQuery { query, variables };

    let req_url = format!("{}/graphql", self.get_host());
    let result = self
      .client
      .post(req_url)
      .json(&graphql_query)
      .send()
      .await
      .unwrap();

    let data = result.json::<GQLResultInterface>().await?;

    Ok(data.data.transactions)
  }

  pub async fn load_contract(
    &self,
    contract_id: String,
    contract_src_tx_id: Option<String>,
    contract_type: Option<String>,
    cache: bool,
  ) -> Result<LoadedContract, AnyError> {
    let mut result: Option<LoadedContract> = None;

    if cache {
      result = ARWEAVE_CACHE.find_contract(contract_id.to_owned()).await;
    }

    if result.is_some() {
      Ok(result.unwrap())
    } else {
      let contract_transaction = self.get_transaction(&contract_id).await?;

      let contract_src = contract_src_tx_id
        .or_else(|| contract_transaction.get_tag("Contract-Src").ok())
        .ok_or_else(|| {
          AnyError::msg("Contract-Src tag not found in transaction")
        })?;

      let min_fee = contract_transaction.get_tag("Min-Fee").ok();

      let contract_src_tx = self.get_transaction(&contract_src).await?;

      let contract_src_data =
        self.get_transaction_data(&contract_src_tx.id).await;

      let mut state: String;

      if let Ok(init_state_tag) = contract_transaction.get_tag("Init-State") {
        state = init_state_tag;
      } else if let Ok(init_state_tag_txid) =
        contract_transaction.get_tag("Init-State-TX")
      {
        let init_state_tx = self.get_transaction(&init_state_tag_txid).await?;
        state = decode_base_64(init_state_tx.data);
      } else {
        state = decode_base_64(contract_transaction.data.to_owned());

        if state.is_empty() {
          state = String::from_utf8(
            self.get_transaction_data(&contract_transaction.id).await,
          )
          .unwrap();
        }
      }

      let contract_type = get_contract_type(
        contract_type,
        &contract_transaction,
        &contract_src_tx,
      )?;

      let final_result = LoadedContract {
        id: contract_id,
        contract_src_tx_id: contract_src,
        contract_src: contract_src_data,
        contract_type,
        init_state: state,
        min_fee,
        contract_transaction,
      };

      if cache {
        ARWEAVE_CACHE.cache_contract(&final_result).await;
      }

      Ok(final_result)
    }
  }

  fn get_host(&self) -> String {
    let protocol = match self.protocol {
      ArweaveProtocol::HTTP => "http",
      ArweaveProtocol::HTTPS => "https",
    };

    if self.port == 80 {
      format!("{}://{}", protocol, self.host)
    } else {
      format!("{}://{}:{}", protocol, self.host, self.port)
    }
  }

  async fn get_default_gql_variables(
    &self,
    contract_id: String,
    height: usize,
  ) -> InteractionVariables {
    let app_name_tag: TagFilter = TagFilter {
      name: "App-Name".to_owned(),
      values: vec!["SmartWeaveAction".to_owned()],
    };

    let contract_tag: TagFilter = TagFilter {
      name: "Contract".to_owned(),
      values: vec![contract_id],
    };

    let variables: InteractionVariables = InteractionVariables {
      tags: vec![app_name_tag, contract_tag],
      block_filter: BlockFilter { max: height },
      first: MAX_REQUEST,
      after: None,
    };

    variables
  }

  async fn stream_interactions(
    &self,
    cursor: Option<String>,
    variables: InteractionVariables,
  ) -> Vec<GQLTransactionsResultInterface> {
    stream::unfold(State::Next(cursor, variables), |state| async move {
      match state {
        State::End => None,
        State::Next(cursor, variables) => {
          let mut new_variables: InteractionVariables = variables.clone();

          new_variables.after = cursor;

          let tx = self
            .get_next_interaction_page(new_variables, false, None)
            .await
            .unwrap();

          if tx.edges.is_empty() {
            None
          } else {
            let max_requests = self.get_max_edges(&tx.edges);

            let edge = tx.edges.get(max_requests);

            if let Some(result_edge) = edge {
              let cursor = (&result_edge.cursor).to_owned();
              Some((tx, State::Next(Some(cursor), variables)))
            } else {
              None
            }
          }
        }
      }
    })
    .collect::<Vec<GQLTransactionsResultInterface>>()
    .await
  }

  fn get_max_edges(&self, data: &[GQLEdgeInterface]) -> usize {
    let len = data.len();
    if len == MAX_REQUEST {
      MAX_REQUEST - 1
    } else if len == 0 {
      len
    } else {
      len - 1
    }
  }

  async fn has_more(
    &self,
    variables: &InteractionVariables,
    cursor: String,
  ) -> Result<bool, AnyError> {
    let mut variables = variables.to_owned();
    variables.after = Some(cursor);
    variables.first = 1;

    let load_transactions = self
      .get_next_interaction_page(variables, false, None)
      .await?;

    Ok(!load_transactions.edges.is_empty())
  }
}
