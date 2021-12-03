(function (window) {

    class ContractError extends Error {
        constructor(message) {
            super(message);
            this.name = 'ContractError'
        }
    }

    function ContractAssert(cond, message) {
        if (!cond) throw new ContractError(message);
    }

    window.ContractError = ContractError;
    window.ContractAssert = ContractAssert;
})(this);
