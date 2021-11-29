build:
	clang --target=wasm32-unknown-wasi -O3 -flto -Wl,--lto-O3 --sysroot ./libc -nostartfiles -Wl,--no-entry -v -o contract.wasm contract.c cJSON/cJSON.c