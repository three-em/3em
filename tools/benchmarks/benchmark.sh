REPO=$(git rev-parse --show-toplevel)
BUILDS=${BUILDS:-target/release}

hyperfine \
  "$REPO/$BUILDS/bench_wasm" \
  "$REPO/$BUILDS/bench_evm" \
  "$REPO/$BUILDS/bench_fh" \
  "$REPO/$BUILDS/bench" \
  "node $REPO/tools/benchmarks/smartweave/index.js" \
  --runs 20 \
  --warmup 5 \
  --time-unit "millisecond" \
  --export-json "./results.json" \
  --export-markdown "./results.md"
