use std::collections::BTreeMap;
pub use primitive_types::H128;
pub use primitive_types::U256;
pub use primitive_types::H160;
use std::str::FromStr;
use evm::{Config, ExitReason};
use evm::backend::MemoryBackend;
use evm::executor::stack::{MemoryStackState, PrecompileFn, PrecompileSet, StackExecutor, StackState};

pub struct EvmAccount {
    pub address: String,
    pub memory: evm::backend::MemoryAccount
}

pub struct EvmConfig {
    pub program: String,
    pub contract: EvmAccount,
    pub caller: EvmAccount,
    pub state: BTreeMap<H160, evm::backend::MemoryAccount>
}

pub struct CallData {
    input: String,
    amount: Option<U256>
}

pub struct Evm<'a> {
    pub memory: evm::backend::MemoryBackend<'a>,
    pub contract_address: H160,
    pub caller_address: H160,
    pub fork_config: evm::Config,
    pub precompiles: BTreeMap<H160, PrecompileFn>
}

impl<'memory> Evm<'memory> {
    
    pub fn new(config: EvmConfig, fork_config: evm::Config, vicinity: &'memory evm::backend::MemoryVicinity) -> Self {
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
        let backend = evm::backend::MemoryBackend::new(&vicinity, state);

        Self {
            memory: backend,
            contract_address,
            caller_address,
            fork_config,
            precompiles: BTreeMap::new()
        }
    }
    
    pub fn call_raw(&mut self, input: String, amount: Option<U256>) -> (ExitReason, Vec<u8>) {
        let metadata = evm::executor::stack::StackSubstateMetadata::new(u64::MAX, &self.fork_config);
        let mut state = evm::executor::stack::MemoryStackState::new(metadata, &mut self.memory);
        let mut executor = evm::executor::stack::StackExecutor::new_with_precompiles(state, &self.fork_config, &self.precompiles);

        let res = executor.transact_call(
            self.caller_address.clone(),
            self.contract_address.clone(),
            amount.unwrap_or(U256::zero()),
            hex::decode(input.as_str()).unwrap(),
            u64::MAX,
            Vec::new(),
        );

        res
    }

    pub fn call(&mut self, data: CallData) -> (ExitReason, Vec<u8>) {
        self.call_raw(data.input, data.amount)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use evm::backend::MemoryAccount;
    use evm::{Config, ExitReason};
    use crate::wevm;
    pub use wevm::EvmConfig;
    pub use wevm::EvmAccount;
    use crate::wevm::{CallData, Evm};
    pub use primitive_types::H128;
    pub use primitive_types::U256;
    pub use primitive_types::H160;
    use primitive_types::H256;
    use std::str::FromStr;

    #[tokio::test]
    pub async fn test_wevm() {
        // SPDX-License-Identifier: MIT
        //         pragma solidity ^0.8.0;
        // contract Calculator {
        //     string public storedString;
        //
        //     // Constructor to initialize the stored string
        //     constructor(string memory _initString) {
            //     storedString = _initString;
        //     }
        //
        //     // Function to add two numbers
        //     function add(uint256 a, uint256 b) public pure returns (uint256) {
            //     return a + b;
        //     }
        //
        //     // Recursive function to calculate Fibonacci number
        //     function fibonacci(uint256 n) public returns (uint256) {
            //     if (n <= 1) {
             //     return n;
            //     } else {
                //     return fibonacci(n - 1) + fibonacci(n - 2);
            //     }
        //     }
        // }

        let mut initial_memory: BTreeMap<H256, H256> = BTreeMap::new();
        initial_memory.insert(H256::from_str("0x0000000000000000000000000000000000000000000000000000000000000000").unwrap(), H256::from_str("0x48656c6c6f20576f726c64000000000000000000000000000000000000000016").unwrap());

        let config = EvmConfig {
            program: "608060405234801561001057600080fd5b50600436106100415760003560e01c806361047ff414610046578063771602f71461006c578063dcb21d121461007f575b600080fd5b610059610054366004610173565b610094565b6040519081526020015b60405180910390f35b61005961007a36600461018c565b6100ce565b6100876100e1565b60405161006391906101ae565b6000600182116100a2575090565b6100b0610054600284610213565b6100be610054600185610213565b6100c89190610226565b92915050565b60006100da8284610226565b9392505050565b6060600080546100f090610239565b80601f016020809104026020016040519081016040528092919081815260200182805461011c90610239565b80156101695780601f1061013e57610100808354040283529160200191610169565b820191906000526020600020905b81548152906001019060200180831161014c57829003601f168201915b5050505050905090565b60006020828403121561018557600080fd5b5035919050565b6000806040838503121561019f57600080fd5b50508035926020909101359150565b60006020808352835180602085015260005b818110156101dc578581018301518582016040015282016101c0565b506000604082860101526040601f19601f8301168501019250505092915050565b634e487b7160e01b600052601160045260246000fd5b818103818111156100c8576100c86101fd565b808201808211156100c8576100c86101fd565b600181811c9082168061024d57607f821691505b60208210810361026d57634e487b7160e01b600052602260045260246000fd5b5091905056fea26469706673582212201d681b10c3d1f5bfedecac6e7e165204af09ced692b3d5051fc09100107635dd64736f6c63430008170033".to_string(),
            contract: EvmAccount {
                address: "0x1000000000000000000000000000000000000000".to_string(),
                memory: MemoryAccount {
                    nonce: Default::default(),
                    balance: Default::default(),
                    storage: initial_memory,
                    code: vec![]
                }
            },
            caller: EvmAccount { address: "0xf000000000000000000000000000000000000000".to_string(), memory: Default::default() },
            state: Default::default()
        };

        let vicinity = evm::backend::MemoryVicinity {
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
        };
        
        let mut evm = Evm::new(config, Config::istanbul(), &vicinity);
        let res = evm.call(CallData {
            /// add(7,2)
            input: "771602f700000000000000000000000000000000000000000000000000000000000000070000000000000000000000000000000000000000000000000000000000000002".to_string(),
            amount: None
        });
        println!("Succeeded?: {}", res.0.is_succeed());

        /// 9
        let hex_result = hex::encode(res.1);
        println!("{}", hex_result.clone());
        assert_eq!(hex_result, "0000000000000000000000000000000000000000000000000000000000000009");
        
        /// Hello world
        let res = evm.call(CallData {
            input: "dcb21d12".to_string(),
            amount: None
        });

        println!("Hello world read succeded? {}", res.0.is_succeed());
        let hex_result = hex::encode(res.1.clone());
        assert_eq!(hex_result.clone(), "0000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000000b48656c6c6f20576f726c64000000000000000000000000000000000000000000");
        let relevant = String::from_utf8(hex::decode(&hex_result[128..150]).unwrap()).unwrap().trim_matches(char::from(0)).to_string();
        println!("{}", relevant.clone());
        assert_eq!("Hello World", relevant);
    }

}