REPO=$(git rev-parse --show-toplevel)
BUILDS=${BUILDS:-$REPO/target/release}

hyperfine \
  "$BUILDS/bench_wasm" \
  "$BUILDS/bench_evm" \
  "$BUILDS/bench_fh" \
  "$BUILDS/bench" \
  "node ./smartweave/index.js" \
  --runs 20 \
  --warmup 5 \
  --time-unit "millisecond" \
  --export-json "./results.json" \
  --export-markdown "./results.md"
