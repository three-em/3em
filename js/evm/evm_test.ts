import { hex, Machine } from "./index.js";
import { assertEquals } from "https://deno.land/std@0.117.0/testing/asserts.ts";

const module = await Deno.readFile("./evm.wasm");

Deno.test("evm_test_solc2", () => {
  // label_0000:
  // 	// Inputs[1] { @0005  msg.value }
  // 	0000    60  PUSH1 0x80
  // 	0002    60  PUSH1 0x40
  // 	0004    52  MSTORE
  // 	0005    34  CALLVALUE
  // 	0006    80  DUP1
  // 	0007    15  ISZERO
  // 	0008    60  PUSH1 0x0f
  // 	000A    57  *JUMPI
  // 	// Stack delta = +1
  // 	// Outputs[2]
  // 	// {
  // 	//     @0004  memory[0x40:0x60] = 0x80
  // 	//     @0005  stack[0] = msg.value
  // 	// }
  // 	// Block ends with conditional jump to 0x000f, if !msg.value

  // label_000B:
  // 	// Incoming jump from 0x000A, if not !msg.value
  // 	// Inputs[1] { @000E  memory[0x00:0x00] }
  // 	000B    60  PUSH1 0x00
  // 	000D    80  DUP1
  // 	000E    FD  *REVERT
  // 	// Stack delta = +0
  // 	// Outputs[1] { @000E  revert(memory[0x00:0x00]); }
  // 	// Block terminates

  // label_000F:
  // 	// Incoming jump from 0x000A, if !msg.value
  // 	// Inputs[1] { @001B  memory[0x00:0x77] }
  // 	000F    5B  JUMPDEST
  // 	0010    50  POP
  // 	0011    60  PUSH1 0x77
  // 	0013    80  DUP1
  // 	0014    60  PUSH1 0x1d
  // 	0016    60  PUSH1 0x00
  // 	0018    39  CODECOPY
  // 	0019    60  PUSH1 0x00
  // 	001B    F3  *RETURN
  // 	// Stack delta = -1
  // 	// Outputs[2]
  // 	// {
  // 	//     @0018  memory[0x00:0x77] = code[0x1d:0x94]
  // 	//     @001B  return memory[0x00:0x77];
  // 	// }
  // 	// Block terminates

  // 	001C    FE    *ASSERT
  // 	001D    60    PUSH1 0x80
  // 	001F    60    PUSH1 0x40
  // 	0021    52    MSTORE
  // 	0022    34    CALLVALUE
  // 	0023    80    DUP1
  // 	0024    15    ISZERO
  // 	0025    60    PUSH1 0x0f
  // 	0027    57    *JUMPI
  // 	0028    60    PUSH1 0x00
  // 	002A    80    DUP1
  // 	002B    FD    *REVERT
  // 	002C    5B    JUMPDEST
  // 	002D    50    POP
  // 	002E    60    PUSH1 0x04
  // 	0030    36    CALLDATASIZE
  // 	0031    10    LT
  // 	0032    60    PUSH1 0x28
  // 	0034    57    *JUMPI
  // 	0035    60    PUSH1 0x00
  // 	0037    35    CALLDATALOAD
  // 	0038    60    PUSH1 0xe0
  // 	003A    1C    SHR
  // 	003B    80    DUP1
  // 	003C    63    PUSH4 0x4f2be91f
  // 	0041    14    EQ
  // 	0042    60    PUSH1 0x2d
  // 	0044    57    *JUMPI
  // 	0045    5B    JUMPDEST
  // 	0046    60    PUSH1 0x00
  // 	0048    80    DUP1
  // 	0049    FD    *REVERT
  // 	004A    5B    JUMPDEST
  // 	004B    60    PUSH1 0x03
  // 	004D    60    PUSH1 0x40
  // 	004F    51    MLOAD
  // 	0050    90    SWAP1
  // 	0051    81    DUP2
  // 	0052    52    MSTORE
  // 	0053    60    PUSH1 0x20
  // 	0055    01    ADD
  // 	0056    60    PUSH1 0x40
  // 	0058    51    MLOAD
  // 	0059    80    DUP1
  // 	005A    91    SWAP2
  // 	005B    03    SUB
  // 	005C    90    SWAP1
  // 	005D    F3    *RETURN
  // 	005E    FE    *ASSERT
  // 	005F    A2    LOG2
  // 	0060    64    PUSH5 0x6970667358
  // 	0066    22    22
  // 	0067    12    SLT
  // 	0068    20    SHA3
  // 	0069    FD    *REVERT
  // 	006A    A5    A5
  // 	006B    D9    D9
  // 	006C    D2    D2
  // 	006D    16    AND
  // 	006E    9A    SWAP11
  // 	006F    B4    B4
  // 	0070    47    SELFBALANCE
  // 	0071    B0    PUSH
  // 	0072    A2    LOG2
  // 	0073    35    CALLDATALOAD
  // 	0074    7F    PUSH32 0xe5d46648452a2ffa2d4046a757ef013fbfe7a7d764736f6c634300080a0033
  //
  // Basically, Solidity code for 1 + 2 = 3
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
});
