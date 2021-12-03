const SmartWeaveSdk = require("redstone-smartweave");
const Arweave = require("arweave");

const arweave = Arweave.init({
  host: "arweave.net",
  port: 443,
  protocol: "https",
});

const smartweave = SmartWeaveSdk.SmartWeaveNodeFactory.memCached(
  arweave,
  749180,
);
(async () => {
  await smartweave.contract("t9T7DIOGxx4VWXoCEeYYarFYeERTpWIC1V3y-BPZgKE")
    .setEvaluationOptions({
      fcpOptimization: true,
    }).readState(749180);
})();
