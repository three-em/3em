use three_em_arweave::arweave::Arweave;
use three_em_executor::execute_contract;

#[tokio::main]
async fn main() {
  let arweave =
    Arweave::new(443, "arweave.net".to_string(), String::from("https"));

  execute_contract(
    arweave,
    "t9T7DIOGxx4VWXoCEeYYarFYeERTpWIC1V3y-BPZgKE".to_string(),
    None,
    None,
    Some(749180),
    true,
    false,
  )
  .await
  .unwrap();
}
