build:
	cargo build --target wasm32-unknown-unknown --release
	cp target/wasm32-unknown-unknown/release/evm.wasm .
	wasm-opt -O3 evm.wasm -o evm.wasm

debug:
	cargo build --target wasm32-unknown-unknown
	cp target/wasm32-unknown-unknown/debug/evm.wasm ./
