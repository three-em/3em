### Prerequisites

- `wasi-libc`

```bash
$ cd wasi-libc
$ make install INSTALL_DIR=../libc
```

- `wasi-sdk`

Install and extract static library from here:
https://github.com/WebAssembly/wasi-sdk/releases/tag/wasi-sdk-14

```bash
$ mv path/to/libcland_rt.builtins.wasm32.a /path/to/llvm/lib/clang/VERSION/lib/wasi/
```

- `cJSON`

### Compiling

```bash
$ sudo apt install llvm clang
$ make
```
