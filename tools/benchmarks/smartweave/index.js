const Arweave = require("arweave");
const { readContract } = require("smartweave");

const arweave = Arweave.init({
  host: "www.arweave.run",
  port: 443,
  protocol: "https",
});

(async () => {
  console.log(
    await readContract(
      arweave,
      "tC4k2NpJoXNDbnBMhQw02o7lmKLqHfsOQcQ9u8wF3a4",
    ),
  );
})();
