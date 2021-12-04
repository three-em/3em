use crate::arweave_get_tag::get_tag;
use crate::cache::ArweaveCache;
use crate::gql_result::{
  GQLEdgeInterface, GQLNodeParent, GQLResultInterface,
  GQLTransactionsResultInterface,
};
use crate::miscellaneous::{get_contract_type, ContractType};
use crate::utils::decode_base_64;
use deno_core::futures::stream;
use deno_core::futures::StreamExt;
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

#[derive(Deserialize, Serialize, Default, Clone)]
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
  pub min_fee: String,
  pub contract_transaction: TransactionData,
}

pub static MAX_REQUEST: usize = 100;

impl Arweave {
  pub fn new(port: i32, host: String) -> Arweave {
    return Arweave {
      port,
      host,
      protocol: ArweaveProtocol::HTTPS,
      client: Client::new(),
    };
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
  ) -> Vec<GQLEdgeInterface> {
    let variables = self
      .get_default_gql_variables(contract_id.to_owned(), height.to_owned())
      .await;

    let mut transactions =
      self.get_next_interaction_page(variables.clone()).await;

    let mut tx_infos = transactions.edges.clone();

    let variables = self
      .get_default_gql_variables(contract_id.to_owned(), height.to_owned())
      .await;

    enum State {
      Next(String, InteractionVariables),
      End,
    }

    let mut cursor = String::from("");
    let maybe_edge = transactions.edges.get(MAX_REQUEST - 1);

    if let Some(data) = maybe_edge {
      cursor = data.cursor.to_owned();
    } else {
      cursor = String::from("null");
    }

    let results =
      stream::unfold(State::Next(cursor, variables), |state| async move {
        match state {
          State::End => return None,
          State::Next(cursor, variables) => {
            let mut new_variables: InteractionVariables = variables.clone();

            if !(cursor.eq("null")) {
              new_variables.after = Some(cursor);
            }

            let tx = self.get_next_interaction_page(new_variables).await;

            if !tx.page_info.has_next_page {
              return None;
            } else {
              let edge = tx.edges.get(MAX_REQUEST - 1).unwrap();
              let cursor = (&edge.cursor).to_owned();
              return Some((tx, State::Next(cursor, variables)));
            }
          }
        }
      })
      .collect::<Vec<GQLTransactionsResultInterface>>()
      .await;

    for result in results {
      let mut new_tx_infos = result.edges.clone();
      tx_infos.append(&mut new_tx_infos);
    }

    let filtered: Vec<GQLEdgeInterface> = tx_infos
      .into_iter()
      .filter(|p| {
        (p.node.parent.is_none())
          || (p
            .node
            .parent
            .as_ref()
            .unwrap_or_else(|| &GQLNodeParent { id: None }))
          .id
          .is_none()
      })
      .collect();

    filtered
  }

  async fn get_next_interaction_page(
    &self,
    variables: InteractionVariables,
  ) -> GQLTransactionsResultInterface {
    let query = r#"query Transactions($tags: [TagFilter!]!, $blockFilter: BlockFilter!, $first: Int!, $after: String) {
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
  }"#;

    let query_str = String::from(query);

    let graphql_query = GraphqlQuery {
      query: query_str,
      variables,
    };

    let req_url = format!("{}/graphql", self.get_host());
    let result = self
      .client
      .post(req_url)
      .json(&graphql_query)
      .send()
      .await
      .unwrap();

    let data = result.json::<GQLResultInterface>().await.unwrap();
    data.data.transactions
  }

  pub async fn load_contract(
    &self,
    contract_id: String,
    contract_src_tx_id: Option<String>,
    contract_type: Option<String>,
    cache: bool,
  ) -> LoadedContract {
    let arweave_cache = ArweaveCache::new().await;

    let mut result: Option<LoadedContract> = None;

    if cache {
      let cached_contract =
        arweave_cache.find_contract(contract_id.to_owned()).await;
      if let Some(contract) = cached_contract {
        result = Some(contract);
      }
    }

    if result.is_some() {
      result.unwrap()
    } else {
      // TODO: DON'T PANIC
      let contract_transaction =
        self.get_transaction(&contract_id).await.unwrap();

      let contract_src = contract_src_tx_id
        .unwrap_or_else(|| get_tag(&contract_transaction, "Contract-Src"));

      if contract_src.len() <= 0 {
        panic!("Contract contains invalid information for tag 'Contract-Src'");
      }

      let min_fee = get_tag(&contract_transaction, "Min-Fee");

      // TODO: Don't panic
      let contract_src_tx = self.get_transaction(&contract_src).await.unwrap();

      let contract_src_data =
        self.get_transaction_data(&contract_src_tx.id).await;

      let mut state = String::from("");

      let init_state_tag = get_tag(&contract_transaction, "Init-State");
      if init_state_tag.len() >= 1 {
        state = init_state_tag;
      } else {
        let init_state_tag_txid =
          get_tag(&contract_transaction, "Init-State-TX");
        if init_state_tag_txid.len() >= 1 {
          let init_state_tx =
            self.get_transaction(&init_state_tag_txid).await.unwrap();
          state = decode_base_64(init_state_tx.data);
        } else {
          state = decode_base_64(contract_transaction.data.to_owned());

          if state.len() <= 0 {
            state = String::from_utf8(
              self.get_transaction_data(&contract_transaction.id).await,
            )
            .unwrap();
          }
        }
      }

      if state.len() <= 0 {
        panic!("Contract does not have an initial state or an error has occurred while reading it.");
      }

      let contract_type = get_contract_type(
        contract_type,
        &contract_transaction,
        &contract_src_tx,
      );

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
        arweave_cache.cache_contract(&final_result).await;
      }

      final_result
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
    height: Option<usize>,
  ) -> InteractionVariables {
    let app_name_tag: TagFilter = TagFilter {
      name: "App-Name".to_owned(),
      values: vec!["SmartWeaveAction".to_owned()],
    };

    let contract_tag: TagFilter = TagFilter {
      name: "Contract".to_owned(),
      values: vec![contract_id],
    };

    let new_height = match height {
      Some(size) => size,
      None => self.get_network_info().await.height,
    };

    let variables: InteractionVariables = InteractionVariables {
      tags: vec![app_name_tag, contract_tag],
      block_filter: BlockFilter { max: new_height },
      first: MAX_REQUEST,
      after: None,
    };

    variables
  }
}
