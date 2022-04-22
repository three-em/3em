## NodeJS

In order for use `3EM` for Node, please install the following package:

```shell
$ yarn add @three-em/node
# OR
# $ npm install --save @three-em/node
```

```javascript
import { executeContract } from "@three-em/node";
```

## Browser

```html
<script type="module">
    import { executeContract } from "https://unpkg.com/@three-em/js@0.2.8/index.js";
</script>
```

## Deno

```typescript
import { executeContract } from "https://deno.land/x/three_em@0.2.8/index.js";
```

## `executeContract`

Read the current state of all kinds of contracts.

```typescript
export function executeContract<T extends unknown>(
  contractTx: string,
  blockHeight?: number,
  gatewayConfig?: ExecuteConfig
): Promise<{
  state: T;
  validity: Record<string, bool>;
}>;
```

```javascript
import { executeContract } from "./index.js";

const { state, validity } = await executeContract(
  "t9T7DIOGxx4VWXoCEeYYarFYeERTpWIC1V3y",
);
```

```javascript
import { executeContract } from "./index.js";

const { state, validity } = await executeContract(
  "t9T7DIOGxx4VWXoCEeYYarFYeERTpWIC1V3y",
  undefined, 
  {
     host: "www.arweave.run",
     port: 443,
     protocol: "https" 
  }  
);
```

## Global Variables (Deno & Browser)
```javascript
globalThis.ARWEAVE_HOST = "www.arweave.run";
globalThis.ARWEAVE_PROTOCOL = "https";
globalThis.ARWEAVE_PORT = 443;
```

## Runtimes

Three seperate libraries are available for low level use **only**. The API is
subject to breaking changes.

### JavaScript

```javascript
import { Runtime } from "./sw.js";
const rt = new Runtime(source, state, {});

// Faster. At 100 interactions in about 3.68ms.
await rt.executeInteractions(interactions);

// OR

// Slower than `rt.executeInteractions` but more readable
// 100 interactions in ~30.06ms.
for (const interaction of interactions) {
  const input = interaction.node.tags.find((data) => data.name === "Input");
  await rt.execute({ input, caller: interaction.node.owner.address });
}

console.log(rt.state); // Read the state.

rt.destroy();
```

### WASM

```javascript
// usage_01.js
import { WasmRuntime } from "./wasm.js";

const contractBytes = new Uint8Array([/* ... */]);
const rt = new WasmRuntime();
await rt.compile(contractBytes, {});

// `call` only accepts encoded bytes of
// the JSON state to provide better performance and reduce
// conversion overhead on recursive runs.
const initialState = encode({ counter: 0 });
const input = encode({});

const result = rt.call(initialState, input);

const state = decode(result);
assertEqual(state.counter, 1);
```

### EVM

```javascript
// usage_01.js
import { hex, Machine } from "./evm/index.js";

// Solidity code for 1 + 2 = 3
// :-)
const machine = new Machine(module, hex("4f2be91f"));
machine.execute(
  hex(
    "6080604052348015600f57600080fd5b506004361060285760003560e01c80634f2be91f14602d575b600080fd5b60336047565b604051603e91906067565b60405180910390f35b60006003905090565b6000819050919050565b6061816050565b82525050565b6000602082019050607a6000830184605a565b9291505056fea26469706673582212200047574855cc88b41f29d7879f8126fe8da6f03c5f30c66c8e1290510af5253964736f6c634300080a0033",
  ),
);

assertEquals(
  machine.result,
  hex("0000000000000000000000000000000000000000000000000000000000000003"),
);
```
