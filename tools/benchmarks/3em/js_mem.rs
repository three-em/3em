use std::time::Instant;
use three_em_arweave::arweave::Arweave;
use three_em_arweave::cache::CacheExt;
use three_em_arweave::lru_cache::ArweaveLruCache;
use three_em_executor::execute_contract;

#[tokio::main]
async fn main() {
  let arweave = Arweave::new(
    443,
    "arweave.net".to_string(),
    String::from("https"),
    ArweaveLruCache::new(),
  );

  let mut sum: u128 = 0;
  const NUM_ITERATIONS: isize = 1_000_000;

  for i in 0..NUM_ITERATIONS {
    let now = Instant::now();
    execute_contract(
      &arweave,
      "t9T7DIOGxx4VWXoCEeYYarFYeERTpWIC1V3y-BPZgKE".to_string(),
      None,
      None,
      Some(749180),
      true,
      false,
    )
    .await
    .unwrap();
    let elapsed = now.elapsed();
    sum += elapsed.as_nanos();
  }

  let mean = sum / NUM_ITERATIONS as u128;

  println!("Mean: {:.2} nanoseconds", mean);
}
