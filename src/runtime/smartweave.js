(function (window) {
  function getContract() {
    const data = Deno.core.opSync("op_smartweave_init");
    getContract = () => data;
    return data;
  }

  class Arweave {
    get crypto() {}
    get utils() {}
    get ar() {}
    get wallets() {}
  }

  class SmartWeave {
    get transaction() {
      return getContract().transaction;
    }

    get block() {
      return getContract().block;
    }

    get arweave() {
      return new Arweave();
    }

    get contracts() {}

    get unsafeClient() {
      throw new TypeError("Unsafe client not supported.");
    }
  }

  window.SmartWeave = new SmartWeave();
})(this);
