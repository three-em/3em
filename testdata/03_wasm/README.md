### Compiling

```
rustup add target wasm32-unknown-unknown
```

```
cargo build --target wasm32-unknown-unknown --release
cp target/wasm32-unknown-unknown/release/01_wasm.wasm .
```
