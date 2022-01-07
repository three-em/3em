pub mod runtime;

#[cfg(test)]
mod tests {
  use super::runtime::*;

  #[tokio::test]
  async fn test_runtime() {
    let mut rt = V8Runtime::new(V8RuntimeOptions {
      source: "async function handle() { return { state: -69 } }"
        .to_string(),
      state: (),
    })
    .await
    .unwrap();

    rt.call(()).await.unwrap();
  }
}
