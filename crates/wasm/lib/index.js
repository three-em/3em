const encoder = new TextEncoder();

export class WasmRuntime {
  #cost = 0;
  #exports;
  #contractInfo;
  #contractInfoPtr;

  constructor(
    moduleBytes,
    contract,
  ) {
    const module = new WebAssembly.Module(moduleBytes);
    const imports = {
      "env": {
        abort: () => {},
      },
      "3em": {
        consumeGas: (cost) => {
          this.#cost += cost;
        },
        smartweave_read_state: () => {
          throw new Error("not implemented");
        },
      },
      "wasi_snapshot_preview1": {
        "fd_close": () => {},
        "fd_seek": () => {},
        "fd_write": () => {},
      },
    };

    const instance = new WebAssembly.Instance(module, imports);

    const contractInfo = encoder.encode(JSON.stringify(contract));
    const contractPtr = instance.exports._alloc(contractInfo.byteLength);

    this.#contractInfo = contractInfo;
    this.#contractInfoPtr = contractPtr;
    this.#exports = instance.exports;
  }

  get cost() {
    return this.#cost;
  }

  call(state, action) {
    const statePtr = this.#exports._alloc(state.byteLength);
    const actionPtr = this.#exports._alloc(action.byteLength);

    const stateMemRegion = new Uint8Array(
      this.#exports.memory.buffer,
      statePtr,
      state.byteLength,
    );
    stateMemRegion.set(state);

    const actionMemRegion = new Uint8Array(
      this.#exports.memory.buffer,
      actionPtr,
      action.byteLength,
    );
    actionMemRegion.set(action);

    const contractMemRegion = new Uint8Array(
      this.#exports.memory.buffer,
      this.#contractInfoPtr,
      this.#contractInfo.byteLength,
    );
    contractMemRegion.set(this.#contractInfo);

    const resultPtr = this.#exports.handle(
      statePtr,
      state.byteLength,
      actionPtr,
      action.byteLength,
      this.#contractInfoPtr,
      this.#contractInfo.byteLength,
    );

    const resultLen = this.#exports.get_len(resultPtr);

    return new Uint8Array(this.#exports.memory.buffer, resultPtr, resultLen);
  }
}
