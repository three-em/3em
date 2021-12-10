## WASM version of the 3em EVM machine

```javascript
// usage_01.js
import { Machine, hex } from "./index.js";

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