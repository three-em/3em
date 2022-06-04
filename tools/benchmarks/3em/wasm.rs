use three_em_arweave::arweave::Arweave;
use three_em_arweave::cache::ArweaveCache;
use three_em_arweave::cache::CacheExt;
use three_em_executor::execute_contract;

#[tokio::main]
async fn main() {
  let arweave = Arweave::new(
    443,
    "arweave.net".to_string(),
    String::from("https"),
    ArweaveCache::new(),
  );

  execute_contract(
    "KfU_1Uxe3-h2r3tP6ZMfMT-HBFlM887tTFtS-p4edYQ".to_string(),
    None,
    true,
    false,
    None,
    None,
    &arweave,
  )
  .await
  .unwrap();
}
