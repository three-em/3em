import {getTagSource} from "./getTagSource.js";
import {loadContractSource} from "./loadContractSource.js";
import {loadInteractionsSource} from "./loadInteractionsSource.js";
import {Runtime} from "./sw.js";
import {WasmRuntime} from "./wasm.js";
import {hex, Machine} from "./evm/index.js";

const encode = (s) => new TextEncoder().encode(JSON.stringify(s));
function str2u8(str) {
    const bufView = new Uint8Array(str.length);
    for (let i = 0; i < str.length; i++) {
        bufView[i] = str.charCodeAt(i);
    }
    return bufView;
}

export class ExecutorV2 {

    constructor() {
        this.loadContractWorker = this.#createWorker('loadContract');
        this.interactionsWorker = this.#createWorker('loadInteractions');
        this.contractProcessingQueue = {};
        this.interactionProcessingQueue = {};
        this.k = 0;

        const workerHandler = (type) => (event) => {
            const p = this[type === 'loadContract' ? "contractProcessingQueue" : "interactionProcessingQueue"][event.data.key];
            p(event.data.result);
        }

        this.loadContractWorker.onmessage = workerHandler('loadContract');
        this.interactionsWorker.onmessage = workerHandler('loadInteractions');
        this.loadContractWorker.onerror = (error) => { throw error };
        this.interactionsWorker.onerror = (error) =>  { throw error };
    }

    getArweaveGlobalUrl(gateway) {
        return `${((gateway || globalThis || window).ARWEAVE_PROTOCOL) || "https"}://${((gateway || globalThis || window).ARWEAVE_HOST) || "arweave.net"}:${((gateway || globalThis || window).ARWEAVE_PORT) || 443}`;
    }

    #getLoadContractBlob() {
        const loadContractSources = [getTagSource, loadContractSource(this.getArweaveGlobalUrl())];
        return new Blob(loadContractSources, {
            type: "application/javascript",
        });
    }

    #getLoadInteractionsBlob() {
        const sources = [getTagSource, loadInteractionsSource(this.getArweaveGlobalUrl())];
        return new Blob(sources, {
            type: "application/javascript",
        });
    }

    #createWorker(type) {
        return new Worker(
            URL.createObjectURL(type === 'loadContract' ? this.#getLoadContractBlob() : this.#getLoadInteractionsBlob()),
            { eval: true, type: "module" },
        );
    }

    async loadContract(tx, gateway) {
        const key = this.k++;
        const args = { tx, key };
        args.baseUrlCustom = this.getArweaveGlobalUrl(gateway);
        return new Promise((r) => {
            this.loadContractWorker.postMessage(args);
            this.contractProcessingQueue[key] = r;
        });
    }

    async updateInteractions(tx, height, last, gateway) {
        const key = this.k++;
        const args = { tx, height, last, key };
        args.baseUrlCustom = this.getArweaveGlobalUrl(gateway);

        return new Promise((r) => {
            this.interactionsWorker.postMessage(args);
            this.interactionProcessingQueue[key] = r;
        });
    }

    async loadInteractions(tx, height, gateway) {
        return this.updateInteractions(tx, height, false, gateway);
    }

    async executeContract(
        contractId,
        height,
        clearCache,
        gateway,
    ) {
        if (clearCache) {
            localStorage.clear();
        }

        const cachedContract = localStorage.getItem(contractId);
        const cachedInteractions = localStorage.getItem(`${contractId}-interactions`);

        let [contract, interactions] = await Promise.all([
            cachedContract ? JSON.parse(cachedContract) : this.loadContract(contractId, gateway),
            cachedInteractions
                ? JSON.parse(cachedInteractions)
                : this.loadInteractions(contractId, height, gateway),
        ]);

        let updatePromise = [];
        if (cachedInteractions) {
            // So now we have the cached interactions
            // but we still need to ensure that the cached interactions are up to date.
            const lastEdge = interactions[interactions.length - 1];
            if (lastEdge) {
                updatePromise = this.updateInteractions(contractId, height, lastEdge.cursor, gateway);
            }
        }

        if (!cachedContract) {
            localStorage.setItem(contractId, JSON.stringify(contract));
        }
        if (!cachedInteractions) {
            localStorage.setItem(
                `${contractId}-interactions`,
                JSON.stringify(interactions),
            );
        }

        const { source, state, type, tx } = contract;
        let result = undefined;
        switch (type) {
            case "application/javascript":
                const rt = new Runtime(source, state, {}, this, gateway);

                // Slower than `rt.executeInteractions` but more readable
                // 100 interactions in ~30.06ms.
                //
                // for (const interaction of interactions) {
                //    const input = interaction.node.tags.find(data => data.name === "Input");
                //    await rt.execute({ input, caller: interaction.node.owner.address });
                // }

                // Faster. At 100 interactions in about 3.68ms.
                await rt.executeInteractions(interactions, tx);

                const updatedInteractions = await updatePromise;
                if (updatedInteractions?.length > 0) {
                    await rt.executeInteractions(updatedInteractions, tx);
                }

                rt.destroy();

                result = rt.state;
                break;
            case "application/wasm":
                const module = str2u8(source);
                const wasm = new WasmRuntime();
                await wasm.compile(
                    module,
                    {},
                );

                let currState = encode(state);
                for (const interaction of interactions) {
                    const input = interaction.node.tags.find((data) =>
                        data.name === "Input"
                    );
                    currState = wasm.call(
                        currState,
                        encode({
                            input,
                            caller: interaction.node.owner.address,
                        }),
                    );
                }

                result = currState;
                break;
            case "application/octet-stream":
                // TODO(perf): Streaming initalization
                const res = await fetch(
                    "https://github.com/three-em/3em/raw/js_library/js/evm/evm.wasm",
                );
                const evmModule = new Uint8Array(await res.arrayBuffer());
                const bytecode = hex(source);
                const _storage = hex(state);
                for (const interaction of interactions) {
                    const input = interaction.node.tags.find((data) =>
                        data.name === "Input"
                    );

                    const machine = new Machine(evmModule, hex(input));
                    machine.execute(bytecode);
                    result = machine.result;
                }
                break;
            default:
                throw new Error(`Unsupported contract type: ${type}`);
        }

        return result;
    }

}

export const executor = new ExecutorV2();
export const executeContract = (...params) => executor.executeContract(...params);
export const loadContract = (...params) => executor.loadContract(...params);
