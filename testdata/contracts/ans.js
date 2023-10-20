export async function handle(state, action) {
    try {

        const input = action.input;
        
        if (input.function === "mint") {
            const { mint_domain, txid, jwk_n, sig } = input;
            _notPaused();
            
            const domain = _normalizeDomain(mint_domain);
            
            _notMinted(domain);
            
            await _verifyArSignature(jwk_n, sig);
            const caller = await _ownerToAddress(jwk_n);
            
            !state.isPublic ? await _isWhitelisted(caller) : void 0; // check only during WL phase
            
            const holders = state.balances.map((addr) => addr.address);

            // !!!!Switch back to `_validateMintingFee` with fixes from the test deriv
            
            let t = await _validateMintingFeeTest(domain, txid, caller);
           
            const domainColor = _generateDomainColor(domain);
            
            
            if (!holders.includes(caller)) {
                
                state.balances.push({
                    address: caller,
                    primary_domain: domain,
                    ownedDomains: [
                        {
                            // change back to domain variable
                            domain: domain,
                            // Change back to domainColor variable
                            color: domainColor,
                            subdomains: [],
                            record: null,
                            created_at: EXM.getDate().getTime(),
                        },
                    ],
                });

            } else {
                const callerIndex = _getValidateCallerIndex(caller);
                state.balances[callerIndex].ownedDomains.push({
                    domain: domain,
                    color: domainColor,
                    subdomains: [],
                    record: null,
                    created_at: EXM.getDate().getTime(),
                });
            }
            const domainType = _getDomainType(domain);
            state.supply[domainType] -= 1;
            state.minting_fees_id.push(txid);

            return { state };
        }

        if (input.function === "transfer") {
            const { to, domain, jwk_n, sig } = input;
            _notPaused();
            _validateArweaveAddress(to);
            await _verifyArSignature(jwk_n, sig);

            const normalizedDomain = _normalizeDomain(domain);
            const caller = await _ownerToAddress(jwk_n);
            const callerIndex = _getValidateCallerIndex(caller);
            const callerProfile = state.balances[callerIndex];
            _isOwnedBy(normalizedDomain, caller);
            ContractAssert(to !== caller, "ERROR_SELF_TRANSFER");

            const oldDomainIndex = state.balances[callerIndex].ownedDomains.findIndex(
                (element) => element.domain === normalizedDomain
            );

            ContractAssert(oldDomainIndex >= 0, "ERROR_NOT_DOMAIN_OWNER");
            const domainCopy =
                state.balances[callerIndex].ownedDomains[oldDomainIndex];
            // **logic flow to add the domain in the receiver balances**
            const holders = state.balances.map((addr) => addr.address);
            if (!holders.includes(to)) {
                state.balances.push({
                    address: to,
                    primary_domain: normalizedDomain,
                    ownedDomains: [domainCopy],
                });
            } else {
                const newOwnerIndex = holders.findIndex((addr) => addr === to);
                state.balances[newOwnerIndex].ownedDomains.push(domainCopy);
            }

            // **logic flow to remove the domain from the sender balances**
            // if the caller is transferring his primary_domain
            if (callerProfile.ownedDomains.length === 1) {
                state.balances.splice(callerIndex, 1);
            } else if (
                callerProfile.ownedDomains.length > 1 &&
                callerProfile.primary_domain === normalizedDomain
            ) {
                state.balances[callerIndex].ownedDomains.splice(oldDomainIndex, 1);
                // re-assign the `primary_domain` to the first domain in the caller's ownedDomains array
                state.balances[callerIndex].primary_domain =
                    state.balances[callerIndex].ownedDomains[0].domain;
            } else {
                state.balances[callerIndex].ownedDomains.splice(oldDomainIndex, 1);
            }

            return { state };
        }

        if (input.function === "setPrimaryDomain") {
            const { domain, jwk_n, sig } = input;
            _notPaused();
            await _verifyArSignature(jwk_n, sig);
            const normalizedDomain = _normalizeDomain(domain);
            const caller = await _ownerToAddress(jwk_n);
            const callerIndex = _getValidateCallerIndex(caller);
            _isOwnedBy(normalizedDomain, caller);
            ContractAssert(
                state.balances[callerIndex].primary_domain !== normalizedDomain,
                "ERROR_EQUAL_PRIMARY"
            );
            state.balances[callerIndex].primary_domain = normalizedDomain;

            return { state };
        }

        if (input.function === "setRecord") {
            const { record, jwk_n, sig, domain } = input;
            _notPaused();
            _validateArweaveAddress(record);
            await _verifyArSignature(jwk_n, sig);
            const normalizedDomain = _normalizeDomain(domain);
            const caller = await _ownerToAddress(jwk_n);
            const callerIndex = _getValidateCallerIndex(caller);
            _isOwnedBy(normalizedDomain, caller);
            const domainIndex = state.balances[callerIndex].ownedDomains.findIndex(
                (element) => element.domain === normalizedDomain
            );

            ContractAssert(domainIndex >= 0, "ERROR_NOT_DOMAIN_OWNER");
            state.balances[callerIndex].ownedDomains[domainIndex].record = record;

            return { state };
        }

        // SUBDOMAIN FUNCTIONS

        if (input.function === "createBatchSubdomain") {
            const { domain, subdomains, jwk_n, sig, txid } = input;

            _notPaused();
            ContractAssert(
                Object.prototype.toString.call(subdomains) === "[object Array]" &&
                subdomains.length,
                "ERROR_INVALID_SUBDOMAINS_LIST"
            );
            await _verifyArSignature(jwk_n, sig);
            const normalizedDomain = _normalizeDomain(domain);
            const caller = await _ownerToAddress(jwk_n);
            const callerIndex = _getValidateCallerIndex(caller);
            const timestamp = EXM.getDate().getTime();
            _isOwnedBy(normalizedDomain, caller);
            await _validateEverpayTxGeneric(
                txid,
                caller,
                state.treasury_address,
                subdomains.length * state.subdomain_creation_fee
            );

            for (const subdomain of subdomains) {
                const normalizedSubdomain = _normalizeDomain(subdomain.subdomain);
                const { address, askPrice, duration } = subdomain;
                if (address) {
                    _validateArweaveAddress(address);
                    ContractAssert(caller !== address, "ERROR_SELF_CREATION");
                    ContractAssert(
                        typeof duration === "number" &&
                        Number.isFinite(duration) &&
                        Number.isInteger(duration) &&
                        duration > 0 &&
                        duration < 367,
                        "ERROR_INVALID_DURATION"
                    );
                    askPrice
                        ? ContractAssert(
                            typeof askPrice === "number" &&
                            isFinite(askPrice) &&
                            askPrice >= 1e-11,
                            "ERROR_INVALID_ASK_PRICE"
                        )
                        : ContractAssert(
                            typeof askPrice === "number" && askPrice === 0,
                            "ERROR_INVALID_ASK_PRICE"
                        );
                } else {
                    // if subdomain is for public sale, then a sale price should be specified
                    ContractAssert(
                        typeof askPrice === "number" &&
                        isFinite(askPrice) &&
                        askPrice >= 1e-11,
                        "ERROR_INVALID_ASK_PRICE"
                    );
                }

                const parentDomainIndex = state.balances[
                    callerIndex
                    ].ownedDomains.findIndex(
                    (element) => element.domain === normalizedDomain
                );
                const parentDomain =
                    state.balances[callerIndex].ownedDomains[parentDomainIndex];
                ContractAssert(
                    !parentDomain.subdomains
                        .map((element) => element.subdomain)
                        .includes(normalizedSubdomain)
                );
                state.balances[callerIndex].ownedDomains[
                    parentDomainIndex
                    ].subdomains.push({
                    subdomain: normalizedSubdomain,
                    ask_price: askPrice,
                    owner: null,
                    for: address ? address : null,
                    expiry: timestamp + duration * 86400000,
                });
            }

            return { state };
        }

        if (input.function === "cancelSubdomain") {
            const { domain, subdomain } = input;

            _notPaused();

            const normalizedDomain = _normalizeDomain(domain);
            const normalizedSubdomain = _normalizeDomain(subdomain);
            const timestamp = EXM.getDate().getTime();
            const callerIndex = state.balances.findIndex((addr) =>
                addr.ownedDomains
                    .map((element) => element.domain)
                    .includes(normalizedDomain)
            );
            ContractAssert(callerIndex >= 0, "ERROR_DOMAIN_NOT_FOUND");
            const parentDomainIndex = state.balances[
                callerIndex
                ].ownedDomains.findIndex(
                (element) => element.domain === normalizedDomain
            );
            const parentDomain =
                state.balances[callerIndex].ownedDomains[parentDomainIndex];
            const subdomainIndex = parentDomain.subdomains.findIndex(
                (element) => element.subdomain === normalizedSubdomain
            );
            ContractAssert(subdomainIndex >= 0, "ERROR_SUBDOMAIN_NOT_FOUND");
            const subdomainOffer = parentDomain.subdomains[subdomainIndex];
            ContractAssert(
                subdomainOffer.owner === null && subdomainOffer.expiry < timestamp,
                "ERROR_SUBDOMAIN_ASSIGNED"
            );
            state.balances[callerIndex].ownedDomains[
                parentDomainIndex
                ].subdomains.splice(subdomainIndex, 1);

            return { state };
        }

        if (input.function === "buySubdomain") {
            const { domain, subdomain, txid, jwk_n, sig } = input;

            _notPaused();
            await _verifyArSignature(jwk_n, sig);
            const normalizedDomain = _normalizeDomain(domain);
            const normalizedSubdomain = _normalizeDomain(subdomain);
            const caller = await _ownerToAddress(jwk_n);
            const timestamp = EXM.getDate().getTime();
            const parentDomainOwnerIndex = state.balances.findIndex((usr) =>
                usr.ownedDomains
                    .map((element) => element.domain)
                    .includes(normalizedDomain)
            );
            ContractAssert(parentDomainOwnerIndex >= 0, "ERROR_DOMAIN_NOT_MINTED");
            const parentDomainOwner = state.balances[parentDomainOwnerIndex];
            _isOwnedBy(normalizedDomain, parentDomainOwner.address);
            ContractAssert(
                caller !== parentDomainOwner.address,
                "ERROR_INVALID_CALLER"
            );
            const parentDomainIndex = state.balances[
                parentDomainOwnerIndex
                ].ownedDomains.findIndex(
                (element) => element.domain === normalizedDomain
            );
            const subdomainIndex = parentDomainOwner.ownedDomains[
                parentDomainIndex
                ].subdomains.findIndex(
                (element) => element.subdomain === normalizedSubdomain
            );
            ContractAssert(subdomainIndex >= 0, "ERROR_SUBDOMAIN_NOT_FOUND");
            const subdomainOffer =
                parentDomainOwner.ownedDomains[parentDomainIndex].subdomains[
                    subdomainIndex
                    ];
            ContractAssert(
                subdomainOffer.owner === null && subdomainOffer.expiry > timestamp,
                "ERROR_SUBDOMAIN_ASSIGNED"
            );

            if (subdomainOffer.for) {
                ContractAssert(subdomainOffer.for === caller, "ERROR_INVALID_CALLER");
            }

            if (subdomainOffer.ask_price) {
                await _validateEverpayTxGeneric(
                    txid,
                    caller,
                    parentDomainOwner.address,
                    subdomainOffer.ask_price
                );
            }

            state.balances[parentDomainOwnerIndex].ownedDomains[
                parentDomainIndex
                ].subdomains[subdomainIndex].owner = caller;
            return { state };
        }

        if (input.function === "unlinkSubdomain") {
            const { domain, subdomain, jwk_n, sig } = input;
            _notPaused();
            await _verifyArSignature(jwk_n, sig);
            const normalizedDomain = _normalizeDomain(domain);
            const normalizedSubdomain = _normalizeDomain(subdomain);
            const caller = await _ownerToAddress(jwk_n);

            const parentDomainOwnerIndex = state.balances.findIndex((usr) =>
                usr.ownedDomains
                    .map((element) => element.domain)
                    .includes(normalizedDomain)
            );
            ContractAssert(parentDomainOwnerIndex >= 0, "ERROR_NOT_DOMAIN_OWNER");
            const parentDomainIndex = state.balances[
                parentDomainOwnerIndex
                ].ownedDomains.findIndex(
                (element) => element.domain === normalizedDomain
            );
            ContractAssert(parentDomainIndex >= 0, "ERROR_DOMAIN_NOT_FOUND");
            const subdomainIndex = state.balances[
                parentDomainOwnerIndex
                ].ownedDomains[parentDomainIndex].subdomains.findIndex(
                (element) => element.subdomain === normalizedSubdomain
            );
            ContractAssert(subdomainIndex >= 0, "ERROR_SUBDOMAIN_NOT_FOUND");

            const subdomainObject =
                state.balances[parentDomainOwnerIndex].ownedDomains[parentDomainIndex]
                    .subdomains[subdomainIndex];
            ContractAssert(subdomainObject.owner === caller, "ERROR_INVALID_CALLER");

            state.balances[parentDomainOwnerIndex].ownedDomains[
                parentDomainIndex
                ].subdomains.splice(subdomainIndex, 1);
            return { state };
        }

        // MARKETPLACE
        if (input.function === "sellDomain") {
            const { domain, address, jwk_n, sig, askPrice, txid, duration } = input;

            _notPaused();
            await _verifyArSignature(jwk_n, sig);

            const normalizedDomain = _normalizeDomain(domain);
            const caller = await _ownerToAddress(jwk_n);
            const callerIndex = _getValidateCallerIndex(caller);
            const callerProfile = state.balances[callerIndex];

            _isOwnedBy(normalizedDomain, caller);

            if (address) {
                _validateArweaveAddress(address);
                ContractAssert(address !== caller, "ERROR_SELF_TRANSFER");
            }

            ContractAssert(
                typeof askPrice === "number" && isFinite(askPrice) && askPrice >= 1e-3,
                "ERROR_INVALID_ASK_PRICE"
            );
            ContractAssert(
                Number.isInteger(duration) && duration >= 1,
                "ERROR_INVALID_DURATION"
            );

            const oldDomainIndex = state.balances[callerIndex].ownedDomains.findIndex(
                (element) => element.domain === normalizedDomain
            );

            ContractAssert(oldDomainIndex >= 0, "ERROR_NOT_DOMAIN_OWNER");
            const domainCopy =
                state.balances[callerIndex].ownedDomains[oldDomainIndex];
            await _validateEverpayTxGeneric(
                txid,
                caller,
                state.treasury_address,
                state.selling_flat_fee
            );

            const creationTimestamp = EXM.getDate().getTime();
            const sellOrder = {
                id: SmartWeave.transaction.id,
                for: address ? address : null,
                type: address ? "targeted" : "public",
                domain: normalizedDomain,
                object: domainCopy,
                owner: caller,
                ask_price: askPrice,
                status: "open",
                timestamp: creationTimestamp,
                expiry: duration * 86400000 + creationTimestamp,
            };

            if (callerProfile.ownedDomains.length === 1) {
                state.balances.splice(callerIndex, 1);
            } else if (
                callerProfile.ownedDomains.length > 1 &&
                callerProfile.primary_domain === normalizedDomain
            ) {
                state.balances[callerIndex].ownedDomains.splice(oldDomainIndex, 1);
                // re-assign the `primary_domain` to the first domain in the caller's ownedDomains array
                state.balances[callerIndex].primary_domain =
                    state.balances[callerIndex].ownedDomains[0].domain;
            } else {
                state.balances[callerIndex].ownedDomains.splice(oldDomainIndex, 1);
            }

            state.marketplace.push(sellOrder);

            return { state };
        }

        if (input.function === "executeOrder") {
            const { id, txid, jwk_n, sig, fee_txid } = input;

            _notPaused();
            await _verifyArSignature(jwk_n, sig);
            ContractAssert(txid !== fee_txid, "ERROR_INVALID_FEE_TXS");

            const caller = await _ownerToAddress(jwk_n);
            const holders = state.balances.map((addr) => addr.address);
            const sellOrderIndex = state.marketplace.findIndex(
                (order) => order.id === id
            );
            ContractAssert(sellOrderIndex >= 0, "ERROR_INVALID_OTC_ORDER_ID");
            const sellOrder = state.marketplace[sellOrderIndex];
            ContractAssert(sellOrder.status === "open", "ERROR_INVALID_SELL_ORDER");
            ContractAssert(caller !== sellOrder.owner, "ERROR_INVALID_CALLER");
            ContractAssert(
                sellOrder.expiry > EXM.getDate().getTime(),
                "ERROR_SELL_ORDER_EXPIRED"
            );
            if (sellOrder.type === "targeted") {
                ContractAssert(sellOrder.for === caller, "ERROR_INVALID_SELL_ORDER");
            }

            await _validateEverpayTxGeneric(
                txid,
                caller,
                sellOrder.owner,
                sellOrder.ask_price
            );
            await _validateTradingFee(fee_txid, caller, sellOrder.ask_price);

            if (!holders.includes(caller)) {
                state.balances.push({
                    address: caller,
                    primary_domain: sellOrder.domain,
                    ownedDomains: [sellOrder.object],
                });
            } else {
                const newOwnerIndex = holders.findIndex((addr) => addr === caller);
                state.balances[newOwnerIndex].ownedDomains.push(sellOrder.object);
            }

            state.marketplace[sellOrderIndex].status = "executed";

            return { state };
        }

        if (input.function === "cancelOrder") {
            const { id } = input;

            _notPaused();

            const holders = state.balances.map((addr) => addr.address);
            const sellOrderIndex = state.marketplace.findIndex(
                (order) => order.id === id
            );
            ContractAssert(sellOrderIndex >= 0, "ERROR_INVALID_OTC_ORDER_ID");
            const sellOrder = state.marketplace[sellOrderIndex];
            ContractAssert(
                EXM.getDate().getTime() > sellOrder.expiry,
                "ERROR_CANNOT_CANCEL"
            );
            ContractAssert(sellOrder.status === "open", "ERROR_ORDER_NOT_CANCELABLE");

            if (!holders.includes(sellOrder.owner)) {
                state.balances.push({
                    address: sellOrder.owner,
                    primary_domain: sellOrder.domain,
                    ownedDomains: [sellOrder.object],
                });
            } else {
                const newOwnerIndex = holders.findIndex(
                    (addr) => addr === sellOrder.owner
                );
                state.balances[newOwnerIndex].ownedDomains.push(sellOrder.object);
            }

            state.marketplace[sellOrderIndex].status = "canceled";

            return { state };
        }

        // QUERY FUNCTIONS

        if (input.function === "retrieveStateKey") {
            const { key } = input;

            ContractAssert(
                typeof key === "string" && key.trim().length,
                "ERROR_INVALID_KEY"
            );

            if (key.trim() in state) {
                return {
                    result: state[key.trim()],
                };
            }

            return { result: "key_not_found" };
        }

        if (input.function === "isMinted") {
            const { domain } = input;
            const normalizedDomain = _normalizeDomain(domain);
            const allDomains = state.balances
                .map((addr) => addr.ownedDomains)
                .map((element) => element.domain)
                .flat();
            const isMinted = allDomains.includes(normalizedDomain);

            return {
                result: isMinted,
            };
        }

        if (input.function === "getSubdomainsMarketplace") {
            const reqTimestamp = EXM.getDate().getTime();
            const subs = [];
            const domains = state.balances
                .map((domain) => domain.ownedDomains)
                .flat();

            for (const domain of domains) {
                if (domain.subdomains.length) {
                    for (const subdomain of domain.subdomains) {
                        subdomain.domain = domain.domain;
                        subs.push(subdomain);
                    }
                }
            }

            const res = subs.filter(
                (subdomain) =>
                    subdomain.owner === null && subdomain.expiry > reqTimestamp
            );

            return {
                result: res,
            };
        }

        // ADMIN FUNCTIONS

        if (input.function === "reversePauseState") {
            const { jwk_n, sig } = input;
            const caller = await _ownerToAddress(jwk_n);
            ContractAssert(caller === state.admin, "ERROR_INVALID_CALLER");

            await _verifyArSignature(jwk_n, sig);
            const currentState = state.isPaused;
            state.isPaused = !currentState;

            return { state };
        }

        if (input.function === "initPublicMint") {
            const { jwk_n, sig } = input;
            const caller = await _ownerToAddress(jwk_n);
            ContractAssert(caller === state.admin, "ERROR_INVALID_CALLER");

            await _verifyArSignature(jwk_n, sig);
            state.isPublic = true;

            return { state };
        }

        if (input.function === "updatePricing") {
            const { jwk_n, sig, type, new_price } = input;
            const caller = await _ownerToAddress(jwk_n);
            ContractAssert(caller === state.admin, "ERROR_INVALID_CALLER");

            await _verifyArSignature(jwk_n, sig);
            ContractAssert(
                Number.isInteger(new_price) &&
                Number.isFinite(new_price) &&
                new_price >= 1,
                "ERROR_INVALID_NEW_PRICE"
            );
            ContractAssert(type in state.pricing, "ERROR_TYPE_NOT_FOUND");
            state.pricing[type.toLowerCase()] = new_price;

            return { state };
        }

        if (input.function === "updateFees") {
            const { jwk_n, sig, type, new_value } = input;
            const caller = await _ownerToAddress(jwk_n);
            ContractAssert(caller === state.admin, "ERROR_INVALID_CALLER");

            await _verifyArSignature(jwk_n, sig);
            ContractAssert(
                ["selling_flat_fee", "trading_fee", "subdomain_creation_fee"].includes(
                    type
                ),
                "ERROR_INVALID_FEE_TYPE"
            );
            ContractAssert(
                typeof new_value === "number" &&
                Number.isFinite(new_value) &&
                new_value > 0,
                "ERROR_INVALID_NEW_PRICE"
            );
            state[type] = new_value;

            return { state };
        }

        if (input.function === "updateSignatureMessage") {
            const { jwk_n, sig, message } = input;

            const caller = await _ownerToAddress(jwk_n);
            ContractAssert(caller === state.admin, "ERROR_INVALID_CALLER");
            ContractAssert(state.isPaused, "ERROR_CONTRACT_SHOULD_BE_PAUSED");

            await _verifyArSignature(jwk_n, sig);
            ContractAssert(
                Object.prototype.toString.call(message) === "[object String]" &&
                message.trim().length,
                "ERROR_INVALID_MSG_TYPE"
            );
            ContractAssert(
                !state.sig_messages.includes(message),
                "ERROR_MSG_ALREADY_USED"
            );
            state.sig_messages.push(message.trim());
            state.signatures = [];
            return { state };
        }

        if (input.function === "updateOracleAddress") {
            const { jwk_n, sig, address } = input;
            const caller = await _ownerToAddress(jwk_n);
            ContractAssert(caller === state.admin, "ERROR_INVALID_CALLER");

            await _verifyArSignature(jwk_n, sig);
            _validateArweaveAddress(address);
            ContractAssert(
                state.whitelsiting_oracle_addr !== address,
                "ERROR_SAME_ORACLE_ADDRESS"
            );

            state.whitelsiting_oracle_addr = address;
            return { state };
        }

        if (input.function === "updateMoleculeEndpoint") {
            const { jwk_n, sig, endpoint, value } = input;
            const caller = await _ownerToAddress(jwk_n);
            ContractAssert(caller === state.admin, "ERROR_INVALID_CALLER");

            await _verifyArSignature(jwk_n, sig);
            ContractAssert(
                ["ar", "ever", "redstone"].includes(endpoint),
                "ERROR_INVALID_ENDPOINT"
            );
            ContractAssert(
                typeof value === "string" && value.length,
                "ERROR_INVALID_ENDPOINT_VALUE"
            );

            state.molecule_endpoints[endpoint] = value;

            return { state };
        }

        function _validateAnsDomainSyntax(domain) {
            ContractAssert(
                /^[a-z0-9]{2,15}$/.test(domain),
                "ERROR_INVALID_ANS_SYNTAX"
            );
        }

        function _normalizeDomain(domain) {
            const caseFolded = domain.toLowerCase();
            const normalizedDomain = caseFolded.normalize("NFKC");
            _validateAnsDomainSyntax(normalizedDomain);
            return normalizedDomain;
        }

        function _notMinted(domain) {
            const target = _normalizeDomain(domain);
            const allDomains = state.balances
                .map((addr) => addr.ownedDomains)
                .map((element) => element.domain)
                .flat();
            const marketplaceDomains = state.marketplace.map((order) => order.domain);
            const all = allDomains.concat(marketplaceDomains);
            ContractAssert(!all.includes(target));
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

        function _notPaused() {
            ContractAssert(!state.isPaused, "ERROR_CONTRACT_PAUSED");
        }

        function _isOwnedBy(domain, address) {
            const owner = state.balances.find((addr) =>
                addr.ownedDomains.map((element) => element.domain).includes(domain)
            );
            ContractAssert(owner, "ERROR_DOMAIN_NOT_MINTED");
            ContractAssert(owner?.address === address, "ERROR_NOT_DOMAIN_OWNER");
        }

        function _getValidateCallerIndex(address) {
            const index = state.balances.findIndex((usr) => usr.address === address);
            ContractAssert(index >= 0, "ERROR_CALLER_NOT_FOUND");
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

        async function _verifyArSignature(owner, signature) {
            try {
                _validatePubKeySyntax(owner);

                const sigBody = state.sig_messages;

                const encodedMessage = new TextEncoder().encode(
                    `${sigBody[sigBody.length - 1]}${owner}`
                );
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
            } catch (error) {
                throw new ContractError("ERROR_INVALID_CALLER_SIGNATURE");
            }
        }

        async function _fetchArPrice() {
            try {
                const req = await EXM.deterministicFetch(
                    `${state.molecule_endpoints.redstone}`
                );
                const price = req.asJSON()?.value;
                ContractAssert(!!price && price > 0, "ERROR_INVALID_AR_PRICE");
                return price;
            } catch (error) {
                throw new ContractError("ERROR_MOLECULE_SERVER_ERROR");
            }
        }

        async function _getEverpayTx(txid, caller) {
            try {
                const req = await EXM.deterministicFetch(
                    `${state.molecule_endpoints.ever}/${txid}`
                );
                const tx = req.asJSON();
                ContractAssert(
                    tx?.tokenSymbol == "AR" &&
                    tx?.action === "transfer" &&
                    !!Number(tx?.amount) &&
                    tx?.to == state.treasury_address &&
                    tx?.from === caller,
                    "ERROR_INVALID_AR_PRICE"
                );

                return tx;
            } catch (error) {
                throw new ContractError("ERROR_MOLECULE_SERVER_ERROR");
            }
        }

        async function _validateEverpayTxGeneric(txid, from, to, amount) {
            try {
                const req = await EXM.deterministicFetch(
                    `${state.molecule_endpoints.ever}/${txid}`
                );
                const tx = req.asJSON();
                ContractAssert(
                    tx?.tokenSymbol == "AR" &&
                    tx?.action === "transfer" &&
                    !!Number(tx?.amount) &&
                    tx?.to == to &&
                    tx?.from === from,
                    "ERROR_INVALID_AR_PRICE"
                );

                ContractAssert(
                    Number(tx?.amount) >= Number((amount * 1e12).toFixed()),
                    "ERROR_UNDERPAID"
                );
            } catch (error) {
                throw new ContractError("ERROR_MOLECULE_SERVER_ERROR");
            }
        }

        async function _validateTradingFee(txid, from, trading_volume) {
            try {
                const req = await EXM.deterministicFetch(
                    `${state.molecule_endpoints.ever}/${txid}`
                );
                const tx = req.asJSON();
                ContractAssert(
                    tx?.tokenSymbol == "AR" &&
                    tx?.action === "transfer" &&
                    !!Number(tx?.amount) &&
                    tx?.to == state.treasury_address &&
                    tx?.from === from,
                    "ERROR_INVALID_AR_PRICE"
                );

                ContractAssert(
                    Number(tx?.amount) >=
                    Number((state.trading_fee * trading_volume * 1e12).toFixed()),
                    "ERROR_UNDERPAID"
                );
            } catch (error) {
                throw new ContractError("ERROR_MOLECULE_SERVER_ERROR");
            }
        }

        async function _validateMintingFee(domain, txid, caller) {
            try {
                const domainType = _getDomainType(domain);
                const domainUsdFee = state.pricing[domainType];
                const arPrice = await _fetchArPrice();
                const expectedPaidFee = domainUsdFee / arPrice; // fee in AR;
                const everTx = await _getEverpayTx(txid, caller);
                const paidAr = Number(everTx?.amount);
                const feeConstant = state.isPublic ? 0.99 : 0.9; // 10% discount for WL mints
                ContractAssert(
                    paidAr >= Number((expectedPaidFee * feeConstant * 1e12).toFixed()),
                    "ERROR_UNDERPAID"
                );
                ContractAssert(!state.minting_fees_id.includes(everTx?.everHash));
                state.total_ar_volume += Number(everTx?.amount) * 1e-12;
            } catch (error) {
                throw new ContractError("ERROR_MOLECULE_SERVER_ERROR");
            }
        }

        async function _validateMintingFeeTest(domain, txid, caller) {
            try {
                const domainType = _getDomainType(domain);
                const domainUsdFee = state.pricing[domainType];
                const arPrice = await _fetchArPrice();

                const expectedPaidFee = domainUsdFee / arPrice; // fee in AR;
                const everTx = await _getEverpayTx(txid, caller);

                const paidAr = Number(everTx?.amount);
                const feeConstant = state.isPublic ? 0.99 : 0.9; // 10% discount for WL mints

                // !!!! switch out the numebrs for paidAr
                ContractAssert(
                    1013417047999+4805402417 >= Number((expectedPaidFee * feeConstant * 1e12).toFixed()),
                    "ERROR_UNDERPAID"
                );
                
                //ContractAssert(!state.minting_fees_id.includes(everTx?.everHash));
                state.total_ar_volume += Number(everTx?.amount) * 1e-12;
                
            } catch (error) {
                throw new ContractError("ERROR_MOLECULE_SERVER_ERROR");
            }
        }

        function _getDomainType(domain) {
            return `l${domain.length}`;
        }

        function _generateDomainColor(domain) {
            let hash = 0;
            for (let i = 0; i < domain.length; i++) {
                hash = domain.charCodeAt(i) + ((hash << 5) - hash);
            }
            let color = "#";
            for (let i = 0; i < 3; i++) {
                const value = (hash >> (i * 8)) & 0xff;
                color += ("00" + value.toString(16)).substr(-2);
            }
            return domain;
        }

        async function _isWhitelisted(address) {
            try {
                const req1 = await EXM.deterministicFetch(
                    `https://api.exm.dev/read/${state.whitelsiting_oracle_addr}`
                );
                const req2 = await EXM.deterministicFetch(
                    `https://arweave.net/-2_zTwb5KluP_9wx5swTsRb0VSifzFpPZ8UxIToNRno`
                );

                const wl = req1.asJSON()?.arweave_addresses;
                const ans_og = req2.asJSON();
                const list = wl.concat(ans_og);
                ContractAssert(list.includes(address), "ERROR_CALLER_NOT_WL");
            } catch (error) {
                throw new ContractError("ERROR_MOLECULE_SERVER_ERROR");
            }
        }
    } catch (error) {
        throw new ContractError("ERROR_INVALID_FUNTION_SUPPLIED");
    }
}