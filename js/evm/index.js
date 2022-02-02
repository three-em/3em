export class Machine {
  #instance;
  #ptr;

  constructor(
    moduleBytes,
    // Optional
    inputData,
  ) {
    const module = new WebAssembly.Module(moduleBytes);
    this.#instance = new WebAssembly.Instance(module);

    if (inputData) {
      const inputPtr = this.#instance.exports.alloc(inputData.byteLength);
      const inputRegion = new Uint8Array(
        this.#instance.exports.memory.buffer,
        inputPtr,
        inputData.byteLength,
      );
      inputRegion.set(new Uint8Array(inputData));

      this.#ptr = this.#instance.exports.machine_new_with_data(
        inputPtr,
        inputData.byteLength,
      );
    } else {
      this.#ptr = this.#instance.exports.machine_new();
    }
  }

  execute(bytecode) {
    const bytecodePtr = this.#instance.exports.alloc(bytecode.byteLength);

    const byteCodeRegion = new Uint8Array(
      this.#instance.exports.memory.buffer,
      bytecodePtr,
      bytecode.byteLength,
    );
    byteCodeRegion.set(new Uint8Array(bytecode));

    // Update pointer to machine.
    this.#ptr = this.#instance.exports.machine_execute(
      this.#ptr,
      bytecodePtr,
      bytecode.byteLength,
    );
  }

  get result() {
    const resultLen = this.#instance.exports.machine_result_len(this.#ptr);
    const resultPtr = this.#instance.exports.machine_result(this.#ptr);
    const result = new Uint8Array(
      this.#instance.exports.memory.buffer,
      resultPtr,
      resultLen,
    );
    return result;
  }
}

export function hex(str) {
  const bytes = [];
  for (let c = 0; c < str.length; c += 2) {
    bytes.push(parseInt(str.substr(c, 2), 16));
  }

  return new Uint8Array(bytes);
}
