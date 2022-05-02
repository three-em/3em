const Arweave = require("arweave");
const { readContract } = require("smartweave");

const arweave = Arweave.init({
  host: "www.arweave.run",
  port: 443,
  protocol: "https",
  logging: true
});

(async () => {
  console.log(
    await readContract(
      arweave,
      "Vjt13JlvOzaOs4St_Iy2jmanxa7dc-Z3pDk3ktwEQNA",
    ),
  );
})();
