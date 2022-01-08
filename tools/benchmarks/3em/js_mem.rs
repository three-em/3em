use three_em_arweave::arweave::Arweave;
use three_em_executor::execute_contract;
use three_em_arweave::lru_cache::ArweaveLruCache;
use three_em_arweave::cache::CacheExt;

#[tokio::main]
async fn main() {
  let arweave =
    Arweave::new(443, "arweave.net".to_string(), String::from("https"), ArweaveLruCache::new());

  for _ in 0..1_000_000 {
    execute_contract(
      arweave.clone(),
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
}
