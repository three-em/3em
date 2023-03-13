export async function handle(state, action) {
    const input = action.input;

    const signatures = state.signatures;
    let message_counter = state.message_counter;

    // if (input.function === "createContainer") {
    //   const { caller_address, sig, type, label } = input;
    //   ContractAssert(
    //     caller_address && sig && type,
    //     "ERROR_MISSING_REQUIRED_ARGUMENTS"
    //   );


    //   const caller = type === "ar" ? await _ownerToAddress(caller_address) : caller_address;

    //   ContractAssert(
    //     typeof label === "string" && label.trim().length,
    //     "ERROR_INVALID_CONTAINER_LABEL"
    //   );

    //   type === "ar"
    //     ? await _verifyArSignature(caller_address, sig, state.messages.ar)
    //     : await _moleculeSignatureVerification(
    //         caller_address,
    //         btoa(state.messages.evm + state.message_counter),
    //         sig,
    //         type
    //       );

    //   const timestamp = EXM.getDate().getTime();

    //   state.containers.push({
    //     id: SmartWeave.transaction.id,
    //     label: label.trim(),
    //     controller_address: caller,
    //     network: type,
    //     first_linkage: timestamp,
    //     last_modification: timestamp,
    //     addresses: [{ address: caller, network: type, proof: sig }],
    //     vouched_by: [],
    //   });

    //   return { state };
    // }

    if (input.function === "createContainer") {
        try {
            const { caller_address, sig, type, label } = input;
            const callerMessage = btoa(state.messages.evm + state.message_counter)
            ContractAssert(
                caller_address && sig && type,
                "ERROR_MISSING_REQUIRED_ARGUMENTS"
            );

            let caller;
            EXM.print("createContainer @ 1\n")
            if (type === "ar") {
                caller = await _ownerToAddress(caller_address)
            } else {
                caller = await _moleculeAddr(caller_address, callerMessage, sig, type);
            }
            EXM.print(`createContainer @ 2 -- caller resolved: ${caller}\n`)


            // const caller = type === "ar" ? await _ownerToAddress(caller_address) : await
            // ContractAssert(
            //   typeof label === "string" && label.trim().length,
            //   "ERROR_INVALID_CONTAINER_LABEL"
            // );

            type === "ar"
                ? await _verifyArSignature(caller_address, sig, state.messages.ar)
                : await _moleculeSignatureVerification(
                    caller_address,
                    btoa(state.messages.evm + state.message_counter),
                    sig,
                    type
                );

            EXM.print("createContainer @ 3\n")

            const timestamp = EXM.getDate().getTime();

            state.containers.push({
                id: SmartWeave.transaction.id,
                label: label.trim(),
                controller_address: caller,
                network: type,
                first_linkage: timestamp,
                last_modification: timestamp,
                addresses: [{ address: caller, network: type, proof: sig }],
                vouched_by: [],
            });

            EXM.print("createContainer @ 4\n")

            return { state };
        } catch(error) {
            EXM.print(error)
            throw new ContractError("error")
        }
    }


    // UTILS

    function _getContainerIndex(id) {
        const index = state.containers.findIndex(
            (container) => container.id === id
        );
        ContractAssert(index >= 0, "ERROR_INVALID_CONTAINER_ID");
        return index;
    }

    async function _ownerToAddress(pubkey) {
        try {
            const req = await EXM.deterministicFetch(
                `${state.molecule_endpoints.ar}/${pubkey}`
            );
            const address = req.asJSON()?.address;
            _validateArweaveAddress(address);
            return address;
        } catch (error) {
            throw new ContractError("ERROR_MOLECULE_SERVER_ERROR");
        }
    }

    async function _typeToMolecule(type) {
        switch (type) {
            case "evm":
                return `${state.molecule_endpoints.evm}`;
            case "sol":
                return `${state.molecule_endpoints.sol}`;
            case "tez":
                return `${state.molecule_endpoints.tez}`;
        }
    }

    async function _getStateAddresses() {
        return state.containers
            .map((container) => container.addresses)
            .flat()
            .map((obj) => obj.address);
    }

    async function _moleculeSignatureVerification(
        caller,
        message,
        signature,
        type
    ) {
        try {
            EXM.print(`_moleculeSignatureVerification @ 1`)
            ContractAssert(!signatures.includes(signature));
            const moleculeEndpoint = await _typeToMolecule(type);
            const endpoint = `${moleculeEndpoint}/${caller}/${message}/${signature}`;
            EXM.print(endpoint);
            const isValid = await EXM.deterministicFetch(
                endpoint
            );
            EXM.print(isValid);
            EXM.print(`_moleculeSignatureVerification @ 2`)
            // EXM.print("molecule here 1")
            // EXM.print(`here---> ${isValid.asJSON()}`)
            // EXM.print(`result: ${isValid.asJSON()?.result}`)
            EXM.print(`_moleculeSignatureVerification @ 3: ${isValid.asJSON()}`)
            ContractAssert(isValid.asJSON()?.result, "ERROR_INVALID_CALLER");
            EXM.print(`_moleculeSignatureVerification @ 4`)
            signatures.push(signature);
            state.message_counter += 1;
            // EXM.print("molecule here 1")
            if (isValid.asJSON()?.address) {
                return isValid.asJSON()?.address;
            }
            return caller;
        } catch (error) {
            EXM.print(error.stack)
            throw new ContractError("ERROR_MOLECULE_CONNECTION");
        }
    }


    async function _moleculeAddr(
        caller,
        message,
        signature,
        type
    ) {
        try {
            EXM.print("_moleculeAddr @ 1\n")
            const moleculeEndpoint = await _typeToMolecule(type);
            const isValid = await EXM.deterministicFetch( // mask the DF and the code will work
                `${moleculeEndpoint}/${caller}/${message}/${signature}`
            );
            EXM.print("_moleculeAddr @ 2\n")

            if (isValid.asJSON()?.address) {
                return isValid.asJSON()?.address;
            }
            // EXM.print(isValid.asJSON())
            // EXM.print(caller)
            EXM.print("_moleculeAddr @ 3\n")
            return caller;

        } catch (error) {
            EXM.print("error")
            throw new ContractError("ERROR_MOLECULE_CONNECTION");
        }
    }


    async function _verifyArSignature(owner, signature, message) {
        try {
            _validatePubKeySyntax(owner);

            const encodedMessage = new TextEncoder().encode(message);
            const typedArraySig = Uint8Array.from(atob(signature), (c) =>
                c.charCodeAt(0)
            );
            const isValid = await SmartWeave.arweave.crypto.verify(
                owner,
                encodedMessage,
                typedArraySig
            );

            ContractAssert(isValid, "ERROR_INVALID_CALLER_SIGNATURE");
            ContractAssert(
                !state.signatures.includes(signature),
                "ERROR_SIGNATURE_ALREADY_USED"
            );
            state.signatures.push(signature);
            // return await _ownerToAddress(owner)
        } catch (error) {
            throw new ContractError("ERROR_INVALID_CALLER_SIGNATURE");
        }
    }

    function _validateArweaveAddress(address) {
        ContractAssert(
            /[a-z0-9_-]{43}/i.test(address),
            "ERROR_INVALID_ARWEAVE_ADDRESS"
        );
    }

    function _validatePubKeySyntax(jwk_n) {
        ContractAssert(
            typeof jwk_n === "string" && jwk_n?.length === 683,
            "ERROR_INVALID_JWK_N_SYNTAX"
        );
    }
}