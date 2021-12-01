build:
	cargo build --target wasm32-unknown-unknown --release
	cp target/wasm32-unknown-unknown/release/01_wasm.wasm .
	wasm-opt -O3 01_wasm.wasm -o 01_wasm.wasm

debug:
	cargo build --target wasm32-unknown-unknown
	cp target/wasm32-unknown-unknown/debug/01_wasm.wasm ./

