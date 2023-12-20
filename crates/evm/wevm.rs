use std::collections::BTreeMap;
pub use primitive_types::H128;
pub use primitive_types::U256;
pub use primitive_types::H160;
use std::str::FromStr;
use evm::{Config, ExitReason};
use evm::executor::stack::{PrecompileSet, StackExecutor, StackState};

pub struct EvmAccount {
    pub address: String,
    pub memory: evm::backend::MemoryAccount
}

pub struct EvmConfig {
    pub program: String,
    pub contract: EvmAccount,
    pub caller: EvmAccount,
    pub vicinity: Option<evm::backend::MemoryVicinity>,
    pub state: BTreeMap<H160, evm::backend::MemoryAccount>
}

pub struct CallData {
    config: EvmConfig,
    input: String
}

pub struct Evm;

impl Evm {
    
    pub fn new() -> Self {
        Self
    }
    
    pub fn call_raw(&self, config: EvmConfig, fork_config: evm::Config, input: String) -> (ExitReason, Vec<u8>) {
        let vicinity = config.vicinity.unwrap_or_else(|| evm::backend::MemoryVicinity {
            gas_price: U256::zero(),
            origin: H160::default(),
            block_hashes: Vec::new(),
            block_number: Default::default(),
            block_coinbase: Default::default(),
            block_timestamp: Default::default(),
            block_difficulty: Default::default(),
            block_gas_limit: Default::default(),
            chain_id: U256::one(),
            block_base_fee_per_gas: U256::zero(),
            block_randomness: None
        });

        let mut state: BTreeMap<H160, evm::backend::MemoryAccount> = BTreeMap::new();

        let contract_address = H160::from_str(config.contract.address.as_str()).unwrap();
        let caller_address = H160::from_str(config.caller.address.as_str())
            .unwrap();

        let mut contract_memory = config.contract.memory.clone();
        contract_memory.code = hex::decode(config.program.clone().as_str()).unwrap();

        // Add Contract
        state.insert(
            contract_address.clone(),
            contract_memory
        );

        // Add Caller
        state.insert(
            caller_address.clone(),
            config.caller.memory.clone()
        );

        // Prepare the executor.
        let mut backend = evm::backend::MemoryBackend::new(&vicinity, state);
        let metadata = evm::executor::stack::StackSubstateMetadata::new(u64::MAX, &fork_config);
        let mut state = evm::executor::stack::MemoryStackState::new(metadata, &mut backend);
        let precompiles = BTreeMap::new();
        let mut executor = evm::executor::stack::StackExecutor::new_with_precompiles(state, &fork_config, &precompiles);

        let res = executor.transact_call(
            caller_address,
            contract_address,
            U256::zero(),
            hex::decode(input.as_str()).unwrap(),
            u64::MAX,
            Vec::new(),
        );

        res
    }

    pub fn call(&self, data: CallData, fork_config: evm::Config) -> (ExitReason, Vec<u8>) {
        self.call_raw(data.config, fork_config, data.input)
    }

    pub fn call_istanbul(&self, data: CallData) -> (ExitReason, Vec<u8>) {
        self.call(data, Config::istanbul())
    }
}

#[cfg(test)]
mod tests {

    use crate::wevm;
    pub use wevm::EvmConfig;
    pub use wevm::EvmAccount;
    use crate::wevm::{CallData, Evm};

    #[tokio::test]
    pub async fn test_wevm() {

        /// contract Calculator {
        //     function add(uint256 a, uint256 b) public pure returns (uint256) {
        //         return a + b;
        //     }
        //
        //     function fibonacci(uint256 n) public returns (uint256) {
        //         if (n <= 1) {
        //             return n;
        //         } else {
        //             return fibonacci(n - 1) + fibonacci(n - 2);
        //         }
        //     }
        // }
        let config = EvmConfig {
            program: "608060405234801561001057600080fd5b50600436106100365760003560e01c806361047ff41461003b578063771602f714610060575b600080fd5b61004e6100493660046100c0565b610073565b60405190815260200160405180910390f35b61004e61006e3660046100d9565b6100ad565b600060018211610081575090565b61008f610049600284610111565b61009d610049600185610111565b6100a79190610124565b92915050565b60006100b98284610124565b9392505050565b6000602082840312156100d257600080fd5b5035919050565b600080604083850312156100ec57600080fd5b50508035926020909101359150565b634e487b7160e01b600052601160045260246000fd5b818103818111156100a7576100a76100fb565b808201808211156100a7576100a76100fb56fea264697066735822122080971c9f1b1121b059f767d9e92368761438251c03f418c4b88a39898d6f94fd64736f6c63430008170033".to_string(),
            contract: EvmAccount {
                address: "0x1000000000000000000000000000000000000000".to_string(),
                memory: Default::default()
            },
            caller: EvmAccount { address: "0xf000000000000000000000000000000000000000".to_string(), memory: Default::default() },
            vicinity: None,
            state: Default::default()
        };
        
        let evm = Evm::new();
        let res = evm.call_istanbul(CallData {
            config,
            /// add(7,2)
            input: "771602f700000000000000000000000000000000000000000000000000000000000000070000000000000000000000000000000000000000000000000000000000000002".to_string()
        });
        println!("Succeeded?: {}", res.0.is_succeed());

        /// 9
        let hex_result = hex::encode(res.1);
        println!("{}", hex_result.clone());
        assert_eq!(hex_result, "0000000000000000000000000000000000000000000000000000000000000009")
    }

}