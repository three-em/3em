# 3EM Technical Guide

## Introduction

This technical guides is intended to better understand the internal behaviors of 3EM and how it can be considered different from other SmartWeave clients.

In collaboration with:
- [Andres Pirela](https://twitter.com/andreestech)
- [Divy Srivastava](https://twitter.com/undefined_void)

------------------

3EM is capable of running JS, WASM, and EVM contracts. To make this possible, we use two technologies: 
- V8 Engine ([See more](https://github.com/denoland/rusty_v8))
- EVM byte code interpreter ([Included in our codebase](https://github.com/three-em/3em/blob/main/crates/evm/lib.rs))

## JS & WASM Context Isolation 

By using direct bindings with V8, we can accomplish a whole list of things:
- No need for runtime wrappers such as NodeJS, Deno, or even your browser.
- No unsafe function is exposed
  - For example, someone running a contract which uses `require("fs")` inside SmartWeave through NodeJS
- Fully deterministic by removing and seeding non-deterministic APIs such as `Math.Random`

By isolating the aforementioned behaviors, 3EM becomes an extremely secure sandbox for smart contracts in the Arweave Ecosystem, but it also reduces a lot of execution overhead at a lower level since only what is needed is provided and executed.

## JS & WASM Determinism

3EM is highly deterministic, this means, even if you try to write a malicious contract with non-deterministic states, chances are it will become deterministic inside 3EM's environment.  
In order to achieve this, we have mocked certain APIs such as [`WeakRef`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/WeakRef) & provided a seed value to other APIs such as `Math.Random` ([Read more about random seeding here](https://en.wikipedia.org/wiki/Random_seed)).

## EVM Interpreter
As mentioned before, 3EM is capable of interpreting EVM Byte Code used by the Ethereum Virtual Machine (EVM). This essentially means, you can write smart contracts using Solidity or other languages that compiled into EVM code inside the Arweave ecosystem. Though, running EVM contracts does not necessarily mean that they will be fully compatible with Arweave, more precisely, 3EM:
- `CALL` opcode is not fully implemented and might give unexpected results
- `CREATE` opcode is not implemented
  - A follow-up discussion for `CREATE` is available [here](https://github.com/three-em/3em/discussions/79). If this is vital for you, please expose your use case.

## Built-in Cache
3EM integrates a built-in cache system for JS and WASM contracts. Essentially, this cache system speeds up the execution of contracts in a reliable way.

1) Contract is ran for the first time
    1) Contract information is saved (including source code)
    2) Interactions from the first time are saved
    3) Latest evaluated state is saved
2) Contract is ran for the second time (**New interactions have taken place**)
   1) Contract information is re-used
   2) **Only** new interactions are fetched, while **old** interactions are used without re-fetching
   3) Latest evaluated state is saved
3) Contract is ran for the third time (**No new interactions are available**)
   1) Contract returns the latest evaluated state since there are no new interactions




