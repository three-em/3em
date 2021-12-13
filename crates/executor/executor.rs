use crate::{get_input_from_interaction, nop_cost_fn};
use deno_core::serde_json;
use deno_core::serde_json::Value;
use std::collections::HashMap;
use three_em_arweave::arweave::Arweave;
use three_em_arweave::arweave::LoadedContract;
use three_em_arweave::arweave::ARWEAVE_CACHE;
use three_em_arweave::gql_result::GQLEdgeInterface;
use three_em_arweave::miscellaneous::ContractType;
use three_em_evm::{ExecutionState, Machine, Storage};
use three_em_js::Runtime;
use three_em_smartweave::{ContractBlock, ContractInfo};
use three_em_wasm::WasmRuntime;

pub type ValidityTable = HashMap<String, bool>;
pub type CachedState = Option<Value>;

pub enum ExecuteResult {
  V8(Value, ValidityTable),
  Evm(Storage, Vec<u8>, ValidityTable),
}

pub type OnCached = dyn Fn() -> ExecuteResult;

pub async fn raw_execute_contract<
  CachedCallBack: FnOnce(ValidityTable, CachedState) -> ExecuteResult,
>(
  contract_id: String,
  loaded_contract: LoadedContract,
  interactions: Vec<GQLEdgeInterface>,
  mut validity: HashMap<String, bool>,
  cache_state: Option<Value>,
  needs_processing: bool,
  on_cached: CachedCallBack,
  shared_client: Arweave,
) -> ExecuteResult {
  let transaction = (&loaded_contract.contract_transaction).to_owned();
  let contract_info = ContractInfo {
    transaction,
    block: ContractBlock {
      height: 0,
      indep_hash: String::from(""),
      timestamp: String::from(""),
    },
  };

  let cache = cache_state.is_some();

  match loaded_contract.contract_type {
    ContractType::JAVASCRIPT => {
      if needs_processing {
        let state: Value = cache_state.unwrap_or_else(|| {
          deno_core::serde_json::from_str(&loaded_contract.init_state).unwrap()
        });

        let mut rt = Runtime::new(
          &(String::from_utf8(loaded_contract.contract_src).unwrap()),
          state,
          contract_info,
        )
        .await
        .unwrap();

        for interaction in interactions {
          let tx = interaction.node;
          let input = get_input_from_interaction(&tx);

          // TODO: has_multiple_interactions
          // https://github.com/ArweaveTeam/SmartWeave/blob/4d09c66d832091805f583ba73e8da96cde2c0190/src/contract-read.ts#L68
          let js_input: Value = deno_core::serde_json::from_str(input).unwrap();

          let call_input = serde_json::json!({
            "input": js_input,
            "caller": tx.owner.address
          });

          let valid = match rt.call(call_input).await {
            Ok(None) => true,
            Ok(Some(evolve)) => {
              let contract = shared_client
                .load_contract(contract_id.clone(), Some(evolve), None, true)
                .await
                .unwrap();

              let transaction = (&contract.contract_transaction).to_owned();
              let contract_info = ContractInfo {
                transaction,
                block: ContractBlock {
                  height: 0,
                  indep_hash: String::from(""),
                  timestamp: String::from(""),
                },
              };

              let state: Value = rt.get_contract_state().unwrap();
              rt = Runtime::new(
                &(String::from_utf8_lossy(&contract.contract_src)),
                state,
                contract_info,
              )
              .await
              .unwrap();

              true
            }
            Err(_) => false,
          };

          validity.insert(tx.id, valid);
        }

        let state_val: Value = rt.get_contract_state().unwrap();

        if cache {
          ARWEAVE_CACHE
            .cache_states(contract_id, &state_val, &validity)
            .await;
        }

        ExecuteResult::V8(state_val, validity)
      } else {
        on_cached(validity, cache_state)
      }
    }
    ContractType::WASM => {
      if needs_processing {
        let wasm = loaded_contract.contract_src.as_slice();

        let init_state_wasm = if cache_state.is_some() {
          let cache_state_unwrapped = cache_state.unwrap();
          let state_str = cache_state_unwrapped.to_string();
          state_str.as_bytes().to_vec()
        } else {
          loaded_contract.init_state.as_bytes().to_vec()
        };

        let mut state = init_state_wasm;
        let mut rt = WasmRuntime::new(wasm, contract_info).unwrap();

        for interaction in interactions {
          let tx = interaction.node;
          let input = get_input_from_interaction(&tx);
          let wasm_input: Value =
            deno_core::serde_json::from_str(input).unwrap();
          let call_input = serde_json::json!({
            "input": wasm_input,
            "caller": tx.owner.address,
          });

          let mut input = deno_core::serde_json::to_vec(&call_input).unwrap();
          let exec = rt.call(&mut state, &mut input);
          let valid = exec.is_ok();
          if valid {
            state = exec.unwrap();
          }
          validity.insert(tx.id, valid);
        }

        let state: Value = deno_core::serde_json::from_slice(&state).unwrap();

        if cache {
          ARWEAVE_CACHE
            .cache_states(contract_id, &state, &validity)
            .await;
        }

        ExecuteResult::V8(state, validity)
      } else {
        on_cached(validity, cache_state)
      }
    }
    ContractType::EVM => {
      // Contract source bytes.
      let bytecode = hex::decode(loaded_contract.contract_src.as_slice())
        .expect("Failed to decode contract bytecode");
      let store = hex::decode(loaded_contract.init_state.as_bytes())
        .expect("Failed to decode account state");

      let mut account_store = Storage::from_raw(&store);
      let mut result = vec![];
      for interaction in interactions {
        let tx = interaction.node;
        let input = get_input_from_interaction(&tx);
        let call_data = hex::decode(input).expect("Failed to decode input");

        let mut machine = Machine::new_with_data(nop_cost_fn, call_data);
        machine.set_storage(account_store.clone());

        match machine.execute(&bytecode) {
          ExecutionState::Abort(_) | ExecutionState::Revert => {
            validity.insert(tx.id, false);
          }
          ExecutionState::Ok => {
            account_store = machine.storage;
            result = machine.result;
            validity.insert(tx.id, true);
          }
        }
      }

      ExecuteResult::Evm(account_store, result, validity)
    }
  }
}

#[cfg(test)]
mod tests {
  use crate::executor::{raw_execute_contract, ExecuteResult};
  use deno_core::serde_json;
  use deno_core::serde_json::Value;
  use std::collections::HashMap;
  use three_em_arweave::arweave::Arweave;
  use three_em_arweave::arweave::{LoadedContract, TransactionData};
  use three_em_arweave::gql_result::{
    GQLBlockInterface, GQLEdgeInterface, GQLNodeInterface, GQLOwnerInterface,
    GQLTagInterface,
  };
  use three_em_arweave::miscellaneous::ContractType;

  #[tokio::test]
  pub async fn test_executor_js() {
    let init_state = serde_json::json!({
      "users": []
    });

    let fake_contract = generate_fake_loaded_contract_data(
      include_bytes!("../../testdata/contracts/users_contract.js"),
      ContractType::JAVASCRIPT,
      init_state.to_string(),
    );
    let fake_interactions = vec![
      generate_fake_interaction(
        serde_json::json!({
          "function": "add",
          "name": "Andres"
        }),
        "tx1",
      ),
      generate_fake_interaction(
        serde_json::json!({
          "function": "none",
          "name": "Tate"
        }),
        "tx2",
      ),
      generate_fake_interaction(
        serde_json::json!({
          "function": "add",
          "name": "Divy"
        }),
        "tx3",
      ),
    ];

    let result = raw_execute_contract(
      String::new(),
      fake_contract,
      fake_interactions,
      HashMap::new(),
      None,
      true,
      |_, _| {
        panic!("not implemented");
      },
      Arweave::new(443, "arweave.net".to_string()),
    )
    .await;

    if let ExecuteResult::V8(value, validity) = result {
      assert_eq!(validity.len(), 3);
      let (tx1, tx2, tx3) = (
        validity.get("tx1"),
        validity.get("tx2"),
        validity.get("tx3"),
      );
      assert_eq!(tx1.is_some(), true);
      assert_eq!(tx2.is_some(), true);
      assert_eq!(tx3.is_some(), true);
      assert_eq!((tx1.unwrap()).to_owned(), true);
      assert_eq!((tx2.unwrap()).to_owned(), false);
      assert_eq!((tx3.unwrap()).to_owned(), true);

      let value_state = value.get("users");
      assert_eq!(value_state.is_some(), true);
      let users = value_state
        .unwrap()
        .to_owned()
        .as_array()
        .unwrap()
        .to_owned();
      assert_eq!(
        users.get(0).unwrap().to_owned(),
        serde_json::json!("Andres")
      );
      assert_eq!(users.get(1).unwrap().to_owned(), serde_json::json!("Divy"));
    } else {
      panic!("Failed");
    }
  }

  #[tokio::test]
  async fn test_contract_evolve() {
    let init_state = serde_json::json!({
      "canEvolve": true,
      "v": 0,
      "i": 0,
      "pi": 0
    });

    let fake_contract = generate_fake_loaded_contract_data(
      include_bytes!("../../testdata/evolve/evolve1.js"),
      ContractType::JAVASCRIPT,
      init_state.to_string(),
    );

    let fake_interactions = vec![
      generate_fake_interaction(
        serde_json::json!({
          "function": "evolve",
          // Contract Source for "Zwp7r7Z10O0TuF6lmFApB7m5lJIrE5RbLAVWg_WKNcU"
          "value": "C0F9QvOOJNR2DDIicWeL9B-C5vFrtczmOjpW_3FCQBQ",
        }),
        "tx1",
      ),
      generate_fake_interaction(
        serde_json::json!({
          "function": "contribute",
        }),
        "tx2",
      ),
    ];

    let result = raw_execute_contract(
      String::from("Zwp7r7Z10O0TuF6lmFApB7m5lJIrE5RbLAVWg_WKNcU"),
      fake_contract,
      fake_interactions,
      HashMap::new(),
      None,
      true,
      |_, _| {
        panic!("not implemented");
      },
      Arweave::new(443, "arweave.net".to_string()),
    )
    .await;

    if let ExecuteResult::V8(value, validity) = result {
      // tx1 is the evolve action. This must not fail.
      // All network calls happen here and runtime is
      // re-initialized.
      assert_eq!(validity.get("tx1").unwrap(), &true);
      // tx2 is the interaction to the evolved source.
      // In this case, the pi contract.
      assert_eq!(validity.get("tx2").unwrap(), &true);

      let value_state = value.get("canEvolve");
      assert_eq!(value_state.is_some(), true);

      assert_eq!(value.get("v").unwrap(), 0.6666666666666667_f64);
    }
  }

  pub fn generate_fake_loaded_contract_data(
    contract_source: &[u8],
    contract_type: ContractType,
    init_state: String,
  ) -> LoadedContract {
    let contract_id = String::from("test");
    LoadedContract {
      id: contract_id,
      contract_src_tx_id: String::new(),
      contract_src: contract_source.to_vec(),
      contract_type,
      init_state,
      min_fee: None,
      contract_transaction: TransactionData {
        format: 0,
        id: String::new(),
        last_tx: String::new(),
        owner: String::new(),
        tags: Vec::new(),
        target: String::new(),
        quantity: String::new(),
        data: String::new(),
        reward: String::new(),
        signature: String::new(),
        data_size: String::new(),
        data_root: String::new(),
      },
    }
  }

  pub fn generate_fake_interaction(input: Value, id: &str) -> GQLEdgeInterface {
    GQLEdgeInterface {
      cursor: String::new(),
      node: GQLNodeInterface {
        id: String::from(id),
        anchor: None,
        signature: None,
        recipient: None,
        owner: GQLOwnerInterface {
          address: String::new(),
          key: None,
        },
        fee: None,
        quantity: None,
        data: None,
        tags: vec![GQLTagInterface {
          name: String::from("Input"),
          value: input.to_string(),
        }],
        block: GQLBlockInterface {
          id: String::new(),
          timestamp: 0,
          height: 0,
          previous: None,
        },
        parent: None,
      },
    }
  }
}
