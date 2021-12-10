ECMAScript implementation of 3EM's WebAssembly execution engine.

## Usage

```javascript
// usage_01.js
import { WasmRuntime } from "./index.js";

const contractBytes = new Uint8Array([ /* ... */ ])
const rt = new WasmRuntime(contractBytes, {});

// `call` only accepts encoded bytes of
// the JSON state to provide better performance and reduce 
// conversion overhead on recursive runs.
const initialState = encode({ counter: 0 });
const input = encode({});

const result = rt.call(initialState, input);

const state = decode(result);
assertEqual(state.counter, 1);
```
