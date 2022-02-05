import { executeContract, loadInteractions } from "./executor.js";
import { assertEquals } from "https://deno.land/std@0.125.0/testing/asserts.ts";

const interactions = await loadInteractions(
  "t9T7DIOGxx4VWXoCEeYYarFYeERTpWIC1V3y-BPZgKE",
  866956,
);
const { state, validity } = await executeContract(
  "t9T7DIOGxx4VWXoCEeYYarFYeERTpWIC1V3y-BPZgKE",
  866956,
  true,
);

console.log(interactions.length);
assertEquals(Object.keys(validity).length, interactions.length);
