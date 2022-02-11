import { Runtime } from "../sw.js";
import { generateFakeInteraction, getDefaultFee } from "./utils.ts";
import { assertEquals } from "https://deno.land/std@0.123.0/testing/asserts.ts";

Deno.test("SmartWeave globals", async function () {
  const runtime = new Runtime(
    `export const handle = async () => {
            return { 
              state: {
                  txId: SmartWeave.transaction.id,
                  txOwner: SmartWeave.transaction.owner,
                  txTarget: SmartWeave.transaction.target,
                  txQuantity: SmartWeave.transaction.quantity,
                  txReward: SmartWeave.transaction.reward,
                  txTags: SmartWeave.transaction.tags,
                  txHeight: SmartWeave.block.height,
                  txIndepHash: SmartWeave.block.indep_hash,
                  txTimestamp: SmartWeave.block.timestamp,
              }
            }
       }`,
    {},
  );

  const interaction = generateFakeInteraction(
    {},
    "1234-TXID",
    "1239210-BLKID",
    870000,
    "MYADDRESS",
    "RECIPIENT",
    undefined,
    getDefaultFee(),
    getDefaultFee(),
    100,
  );
  await runtime.executeInteractions([interaction]);

  assertEquals(runtime.state, {
    state: {
      txId: "1234-TXID",
      txOwner: "MYADDRESS",
      txTarget: "RECIPIENT",
      txQuantity: { winston: "", ar: "" },
      txReward: { winston: "", ar: "" },
      txTags: [{ name: "Input", value: "{}" }],
      txHeight: 870000,
      txIndepHash: "1239210-BLKID",
      txTimestamp: 100,
    },
    validity: { "1234-TXID": true },
  });
});
