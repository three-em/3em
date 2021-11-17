const WASM_BINARY = await Deno.readFile("01_wasm.wasm");

let memory;

const module = new WebAssembly.Module(WASM_BINARY);
const instance = new WebAssembly.Instance(module, {
  env: { abort: function () {} },
});

const alloc = instance.exports.alloc;
memory = instance.exports.memory;

function copyMemory(data) {
  const d = new Uint8Array(data);
  const ptr = instance.exports._alloc(d.byteLength);
  const mem = new Uint8Array(memory.buffer, ptr, d.byteLength);
  mem.set(d);

  return ptr;
}

const state = Deno.core.encode(JSON.stringify({ counter: 0 }));
const action = Deno.core.encode(JSON.stringify({}));

const ptr = copyMemory(state);
const actionPtr = copyMemory(action);

const resPtr = instance.exports.handle(
  ptr,
  state.byteLength,
  actionPtr,
  action.byteLength,
);

const resLen = instance.exports.get_len();

const resultBuf = new Uint8Array(memory.buffer, resPtr, resLen);

console.log(
  JSON.parse(Deno.core.decode(resultBuf)),
);
