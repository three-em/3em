build:
	cargo build --target wasm32-unknown-unknown --release
	cp target/wasm32-unknown-unknown/release/03_wasm.wasm .
	wasm-opt -O3 03_wasm.wasm -o 03_wasm.wasm

debug:
	cargo build --target wasm32-unknown-unknown
	cp target/wasm32-unknown-unknown/debug/03_wasm.wasm ./

