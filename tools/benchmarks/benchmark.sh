REPO=$(git rev-parse --show-toplevel)
BUILDS=${BUILDS:-target/release}

hyperfine \
  --command-name "3em_js_fh" "$REPO/$BUILDS/bench_fh" \
  --command-name "3em_js" "$REPO/$BUILDS/bench" \
  --command-name "3em_evm" "$REPO/$BUILDS/bench_evm" \
  --command-name "3em_wasm" "$REPO/$BUILDS/bench_wasm" \
  --command-name "smartweave.js" "node $REPO/tools/benchmarks/smartweave/index.js" \
  --runs 20 \
  --warmup 5 \
  --time-unit "millisecond" \
  --export-json "results.json" \
  --export-markdown "results.md"
