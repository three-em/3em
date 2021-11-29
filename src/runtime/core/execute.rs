use crate::runtime::core::arweave::Arweave;
use crate::runtime::Runtime;
use crate::runtime::core::gql_result::{GQLNodeInterface, GQLTagInterface, GQLEdgeInterface};

pub async fn execute_contract(arweave: &Arweave, contract_id: String, contract_src_tx: Option<String>, height: Option<usize>) {
    let loaded_contract= arweave.load_contract(contract_id.to_owned(), contract_src_tx);
    let interactions = arweave.get_interactions(contract_id.to_owned(), height);

    // TODO: Sort interactions

    // Todo: handle wasm, evm, etc.
    match &loaded_contract.contract_type[..] {
        "application/javascript" => {
            let source = &String::from_utf8(loaded_contract.contract_src).unwrap()[..];
            let rt = Runtime::new(source).await.unwrap();

            for interaction in interactions {
                let tx = interaction.node;
                let input = get_input_from_interaction(&tx);

                // TODO: has_multiple_interactions  https://github.com/ArweaveTeam/SmartWeave/blob/4d09c66d832091805f583ba73e8da96cde2c0190/src/contract-read.ts#L68
                
            }
        }
        _ => {}
    }
}

pub fn get_input_from_interaction(interaction_tx: &GQLNodeInterface) -> String {
    let tags = (&interaction_tx.tags).to_owned().into_iter().find(|data| data.name == String::from("input")).unwrap_or_else(|| {
        GQLTagInterface {
            name: String::from(""),
            value: String::from("")
        }
    }).value;

    String::from(tags)
}


pub fn has_multiple_interactions(interaction_tx: &GQLNodeInterface) -> bool {
    let tags = (&interaction_tx.tags).to_owned();
    let filtered_tags = tags.iter().filter(|data| data.name == String::from("Contract")).cloned().collect::<Vec<GQLTagInterface>>();
    filtered_tags.len() > 1
}
