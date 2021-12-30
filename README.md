<p align="center">
<h3 align="center">3EM</h3>

  <p align="center">
    A blazingly fast, secure, reliable, multi-language execution environment for smart contracts inside the <a href="https://arweave.org">Arweave</a> ecosystem.
  </p>
</p>

<p align="center">
Special thanks to:
<ul>
    <li>
        <a href="https://verto.exchange">Verto Exchange</a> for incubating this project as well as all their members which have been key part of conceiving 3EM.
    </li>
</ul>
</p>

## Purpose

3EM was conceived as an alternate solution to [SmartWeave](https://github.com/ArweaveTeam/SmartWeave) while still accomplishing the same goal: Simple & Scalable smart contracts on the Arweave protocol.

After receiving multiple feedback from different individuals and teams, we realize there were 3 main issues that needed to be solved:
- Speed
- Security
- Multi-language support

For more information on how we solved the issues mentioned above, please refer to our [technical guide](https://github.com/3distributed/3em/tree/main/docs/technical_guide.md).

## Benchmarks

- [Bar chart benchmark](https://github.com/3distributed/3em/blob/main/data/benchmark_bar.png)
- [Line chart benchmark](https://github.com/3distributed/3em/blob/main/data/benchmark_line.png)

**Note**: Benchmarking is done using `hyperfine` with a max execution of 10 attempts. It measures contracts up to 105 interactions.

## Multi-Language Support

3EM supports contracts written in:
- JS
- Web Assembly (Such as a Rust contracts compiled into WASM)
- EVM Contracts (Any language that compiles to Ethereum Byte Code such as Solidity)

Please refer to our [test data](https://github.com/3distributed/3em/tree/main/testdata) for guidance.

## Smartweave Compatability

3EM aims to follow the SmartWeave standard ([See more](https://github.com/ArweaveTeam/SmartWeave/blob/master/CONTRACT-GUIDE.md)). This essentially means two things:
- Contracts need to be deployed to Arweave in order for 3EM to run them
- All contracts follow the same logic SmartWeave uses & 3EM also exposes the SmartWeave APIs that are available during execution.

## Determinism

3EM isolates every non-deterministic behavior and makes it deterministic. This is done by a technique called "Seeding". For example, you are still allowed to use APIs like `Math.Random` and while it will give you a different value every time you call it, it will have a deterministic seed, you can read more about it in our technical guide.

By making non-deterministic behaviors deterministic, 3EM ensures the same output across contracts, even if they are considered faulty.

## CLI

### Available Commands

- `three_em run`
  - Runs a contract deployed to Arweave given certain options.

### run
The following flags are available for `three_em run`:
- `--arweave-host` | `string`
  - URL of gateway to be used during execution
  - Default: arweave.net
- `--arweave-port` | `number`
  - Port of gateway to be used during execution
  - Default: 443
- `--arweave-protocol` | `string`
  - Network protocol to be used during execution
  - Default: HTTPS
- `--contract-id` | `string`
  - ID of contract to be evaluated
- `--pretty-print` | `boolean`
  - Whether output should be printed in a prettified JSON form
  - Default: false
- `--no-print` | `boolean`
  - Whether output should be printed in the console. True will not print any output
  - Default: false
- `--show-validity` | `boolean`
  - Whether output should contain the validity table of evaluated interactions
- `--save` | `string`
  - If provided, it contains a file path where output will be saved in JSON form
- `--benchmark` | `boolean`
  - Whether to benchmark the execution time of the contract
  - Default: false
- `--height` | `number`
  - Maximum height to be used during evaluation
- `--no-cache`
  - Whether it should use 3EM's built-in cache system
- `--show-errors`
  - Whether errors from failed interactions should be printed

**Example**

```shell
three_em run --contract-id t9T7DIOGxx4VWXoCEeYYarFYeERTpWIC1V3y-BPZgKE
```



