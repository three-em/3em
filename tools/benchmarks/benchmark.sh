hyperfine \
  '../../target/release/bench' \
  '../../target/release/bench_evm' \
  '../../target/release/bench_wasm' \
  '../../target/release/bench_fh' \
  'node ./smartweave/index.js' \
  --runs 20 \
  --warmup 5 \
  --ignore-failure \
  --time-unit "millisecond" \
  --export-json "./results.json" \
  --export-markdown "./results.md"
