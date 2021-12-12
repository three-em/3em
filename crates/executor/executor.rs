use crate::{get_input_from_interaction, nop_cost_fn};
use deno_core::serde_json;
use deno_core::serde_json::Value;
use std::collections::HashMap;
use three_em_arweave::arweave::LoadedContract;
use three_em_arweave::gql_result::GQLEdgeInterface;
use three_em_arweave::miscellaneous::ContractType;
use three_em_evm::{ExecutionState, Machine, Storage};
use three_em_js::Runtime;
use three_em_smartweave::{ContractBlock, ContractInfo};
use three_em_wasm::WasmRuntime;
use three_em_arweave::arweave::ARWEAVE_CACHE;

pub type ValidityTable = HashMap<String, bool>;
pub type CachedState = Option<Value>;

pub enum ExecuteResult {
  V8(Value, ValidityTable),
  Evm(Storage, Vec<u8>, ValidityTable)
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
        let mut state: Value = cache_state.unwrap_or_else(|| {
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

          let valid = rt.call(call_input).await.is_ok();
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
