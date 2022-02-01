const Arweave = require("arweave");
const { readContract } = require("smartweave");
const { writeFileSync, existsSync } = require("fs");
require('isomorphic-fetch');

(async () => {

    const arweave = Arweave.init({
        host: "arweave.net",
        port: 443,
        protocol: "https",
    });

    const communityContract = await ((await fetch("https://storage.googleapis.com/verto-exchange-contracts/t9T7DIOGxx4VWXoCEeYYarFYeERTpWIC1V3y-BPZgKE/t9T7DIOGxx4VWXoCEeYYarFYeERTpWIC1V3y-BPZgKE_state.json")).json());

    for (const item of communityContract["tokens"]) {
        const { id } = item;
        const height = 864568;
        const path = `../../testdata/sw/${id}$$${height}.json`;

        if(!existsSync(path)) {
            try {
                const readContractInfo = await readContract(
                    arweave,
                    id,
                    height,
                    true);

                writeFileSync(path, JSON.stringify(readContractInfo, null, 2));
            } catch(e) {}
        }
    }

})();
