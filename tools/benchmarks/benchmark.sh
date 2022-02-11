hyperfine \
  '../../target/release/bench_wasm' \
  '../../target/release/bench_evm' \
  '../../target/release/bench_fh' \
  '../../target/release/bench' \
  'node ./smartweave/index.js' \
  --runs 20 \
  --warmup 5 \
  --time-unit "millisecond" \
  --export-json "./results.json" \
  --export-markdown "./results.md"
