# 3EM Dry Run Guide

## Introduction

3EM allows you to test your contracts without necessarily deploying them to Arweave. 

**Note**: Dry run is only available for Javascript & Web Assembly Contracts

## Configuration File

In order to use the dry run feature, you will need to create a configuration JSON file. This JSON file will contain enough information for 3EM to be able to interpret it as if it was a real contract.

The structure of the JSON dry run configuration file is as follows:

```typescript
export interface DryRunFile {
    contractType: "JAVASCRIPT" | "WASM",
    contractSource: string,
    initialState: any,
    interactions: Array<{
        id: string,
        caller: string,
        input: any,
        blockId?: string,
        blockHeight?: number,
        blockTimestamp?: number,
        quantity?: string,
        reward?: string,
        tags?: Array<{
            name: string,
            value: string
        }>,
        recipient?: string
    }>
}
```

- `contractType`
  - Indicates what runtime will be used to run the contract: Whether a JS or WASM runtime
- `contractSource`
  - File path of the contract source relative to where `three_em` is running.
- `initialState`
  - Initial state of the contract to be applied
- `interactions`
  - Array of interactions to be used during execution
    - `id`
      - Id of the interaction
    - `caller`
      - Interaction creator's address
    - `input`
      - Input of the interaction
    - `blockId`
      - Indep Hash of the block holding the interaction
    - `blockHeight`
      - Height of the current block holding the interaction
    - `blockTimestamp`
      - Timestamp of the current block holding the interaction
    - `quantity`
      - Quantity in Winston held by the interaction
    - `reward`
      - Reward in Winston given by the interaction
    - `tags`
      - List of tags available during the interaction
        - `name`
          - Tag key
        - `value`
          - Tag value
    - `recipient`
      - Recipient held in the interaction


## Test Example

Please refer to [our dry run example](https://github.com/3distributed/3em/tree/main/docs/dry_run) to see the usage of the configuration file and the source code.

## Creating a Dry-Run

**source.js**

```javascript
// Can be async or sync
export async function handle(state, action) {
 if(action.input.function === 'add') {
     state.users.push(action.input.name);
 } else {
     // Makes transaction invalid in the validity table
     throw new Error("Invalid operation");
 }

 return {
     state
 };
}
```

**configuration.json**
```json
{
"contractType": "JAVASCRIPT",
    "contractSource": "source.js",
    "initialState": {
        "users": []
    },
    "interactions": [
        {
            "id": "tx1",
            "caller": "ap-address",
            "input": {
                "function": "add",
                "name": "Andres Pirela"
            }
        },
        {
          "id": "tx2",
          "caller": "tate-address",
          "input": {
            "function": "none",
            "name": "Tate"
          }
        }
    ]
}
```

**Run**

```shell
$ path/to/three_em dry-run --file configuration.json --show-validity --pretty-print
```

**Output**
```json
{
  "state": {
    "users": [
      "Andres Pirela"
    ]
  },
  "validity": {
    "tx1": true,
    "tx2": false
  }
}
```


## Other Suggestions
- Do not use repeated transaction ids otherwise it will affect the validity table.
