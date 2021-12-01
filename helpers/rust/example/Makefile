build:
	cargo build --target wasm32-unknown-unknown --release
	cp target/wasm32-unknown-unknown/release/contract.wasm .
	wasm-opt -O3 contract.wasm -o contract.wasm