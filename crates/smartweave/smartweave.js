(function (window) {
  const { crypto } = window.__bootstrap.crypto;
  const { subtle } = crypto;

  // Intentional copy.
  const BigNumber = window.BigNumber;

  // function getContract() {
  //   const data = Deno.core.opSync("op_smartweave_init");
  //   getContract = () => data;
  //   return data;
  // }

    function getInteraction() {
      const env = window.env.toObject();
      return {
        transaction: {
          id: env["TX_ID"],
          owner: env["TX_OWNER_ADDRESS"],
          target: env["TX_TARGET"],
          quantity: env["TX_QUANTITY"],
          reward: env["TX_REWARDS"],
          tags: JSON.parse(env["TX_TAGS"])
        },
        block: {
          height: parseInt(env["BLOCK_HEIGHT"]),
          indep_hash: env["BLOCK_ID"],
          timestamp: parseInt(env["BLOCK_TIMESTAMP"])
        }
      }
    }

  // Partially adapted from arweave-js
  // https://github.com/ArweaveTeam/arweave-js/blob/master/src/common/lib/crypto/webcrypto-driver.ts
  class CryptoInterface {
    async generateJwk() {
      const cryptoKey = await subtle.generateKey(
        {
          name: "RSA-PSS",
          modulusLength: 4096,
          publicExponent: new Uint8Array([0x01, 0x00, 0x01]),
          hash: {
            name: "SHA-256",
          },
        },
        true,
        ["sign"],
      );

      const jwk = await subtle.exportKey("jwk", cryptoKey.privateKey);

      return jwk;
    }

    async sign(jwk, data, options) {
      const signature = await subtle.sign(
        {
          name: "RSA-PSS",
          saltLength: 32,
        },
        await this.#jwkToCryptoKey(jwk),
        data,
      );

      return new Uint8Array(signature);
    }

    async verify(publicModulus, data, signature) {
      const publicKey = {
        kty: "RSA",
        e: "AQAB",
        n: publicModulus,
      };

      const key = await this.#jwkToCryptoKey(publicKey);

      const verifyWith32 = subtle.verify(
        {
          name: "RSA-PSS",
          saltLength: 32,
        },
        key,
        signature,
        data,
      );

      const verifyWith0 = subtle.verify(
        {
          name: "RSA-PSS",
          saltLength: 0,
        },
        key,
        signature,
        data,
      );

      return verifyWith32 || verifyWith0;
    }

    async hash(data, algorithm) {
      let digest = await subtle.digest(algorithm, data);
      return new Uint8Array(digest);
    }

    #jwkToCryptoKey(
      jwk,
    ) {
      return subtle.importKey(
        "jwk",
        jwk,
        {
          name: "RSA-PSS",
          hash: {
            name: "SHA-256",
          },
        },
        false,
        ["verify"],
      );
    }

    // TODO(@littledivy): Expose encrypt/decrypt ops from Rust.
    async encrypt(data, key, salt) {}
    async decrypt(encrypted, key, salt) {}
  }

  // Adapted from arweave-js
  // https://github.com/ArweaveTeam/arweave-js/blob/master/src/common/lib/utils.ts
  class ArweaveUtils {
    concatBuffers(
      buffers,
    ) {
      let total_length = 0;

      for (let i = 0; i < buffers.length; i++) {
        total_length += buffers[i].byteLength;
      }

      let temp = new Uint8Array(total_length);
      let offset = 0;

      temp.set(new Uint8Array(buffers[0]), offset);
      offset += buffers[0].byteLength;

      for (let i = 1; i < buffers.length; i++) {
        temp.set(new Uint8Array(buffers[i]), offset);
        offset += buffers[i].byteLength;
      }

      return temp;
    }

    b64UrlToString(b64UrlString) {
      let buffer = b64UrlToBuffer(b64UrlString);
      return new TextDecoder("utf-8", { fatal: true }).decode(buffer);
    }

    bufferToString(buffer) {
      return new TextDecoder("utf-8", { fatal: true }).decode(buffer);
    }

    stringToBuffer(string) {
      return new TextEncoder().encode(string);
    }

    stringToB64Url(string) {
      return this.bufferTob64Url(stringToBuffer(string));
    }

    b64UrlToBuffer(b64UrlString) {
      return Uint8Array.from(atob(b64UrlString), (c) => c.charCodeAt(0));
    }

    bufferTob64(buffer) {
      return btoa(String.fromCharCode.apply(null, new Uint8Array(buffer)));
    }

    bufferTob64Url(buffer) {
      return this.b64UrlEncode(this.bufferTob64(buffer));
    }

    b64UrlEncode(b64UrlString) {
      return b64UrlString
        .replace(/\+/g, "-")
        .replace(/\//g, "_")
        .replace(/\=/g, "");
    }

    b64UrlDecode(b64UrlString) {
      b64UrlString = b64UrlString.replace(/\-/g, "+").replace(/\_/g, "/");
      let padding = 0;
      if (b64UrlString.length % 4 !== 0) {
        padding = 4 - (b64UrlString.length % 4);
      }

      return b64UrlString.concat("=".repeat(padding));
    }
  }

  class Ar {
    constructor() {
      this.BigNum = (value, decimals) => {
        let instance = BigNumber.clone({ DECIMAL_PLACES: decimals });
        return new instance(value);
      };
    }

    winstonToAr(
      winstonString,
      { formatted = false, decimals = 12, trim = true } = {},
    ) {
      let number = this.#stringToBigNum(winstonString, decimals).shiftedBy(-12);

      return formatted ? number.toFormat(decimals) : number.toFixed(decimals);
    }

    arToWinston(arString, { formatted = false } = {}) {
      let number = this.#stringToBigNum(arString).shiftedBy(12);

      return formatted ? number.toFormat() : number.toFixed(0);
    }

    compare(winstonStringA, winstonStringB) {
      let a = this.#stringToBigNum(winstonStringA);
      let b = this.#stringToBigNum(winstonStringB);

      return a.comparedTo(b);
    }

    isEqual(winstonStringA, winstonStringB) {
      return this.compare(winstonStringA, winstonStringB) === 0;
    }

    isLessThan(winstonStringA, winstonStringB) {
      let a = this.#stringToBigNum(winstonStringA);
      let b = this.#stringToBigNum(winstonStringB);

      return a.isLessThan(b);
    }

    isGreaterThan(
      winstonStringA,
      winstonStringB,
    ) {
      let a = this.#stringToBigNum(winstonStringA);
      let b = this.#stringToBigNum(winstonStringB);

      return a.isGreaterThan(b);
    }

    add(winstonStringA, winstonStringB) {
      let a = this.#stringToBigNum(winstonStringA);
      let b = this.#stringToBigNum(winstonStringB);

      return a.plus(winstonStringB).toFixed(0);
    }

    sub(winstonStringA, winstonStringB) {
      let a = this.#stringToBigNum(winstonStringA);
      let b = this.#stringToBigNum(winstonStringB);
      return a.minus(winstonStringB).toFixed(0);
    }

    #stringToBigNum(
      stringValue,
      decimalPlaces = 12,
    ) {
      return this.BigNum(stringValue, decimalPlaces);
    }
  }

  class Wallets {
    #crypto = new CryptoInterface();
    #utils = new ArweaveUtils();

    generate() {
      return this.#crypto.generateJwk();
    }

    async ownerToAddress(owner) {
      return this.#utils.bufferTob64Url(
        await this.#crypto.hash(this.#utils.b64UrlToBuffer(owner)),
      );
    }

    getAddress(jwk) {
      return this.ownerToAddress(jwk.n);
    }

    jwkToAddress(jwk) {
      return this.getAddress(jwk);
    }

    getBalance(address) {
      return Deno.core.opAsync("op_smartweave_wallet_balance", address);
    }

    getLastTransactionID(address) {
      return Deno.core.opAsync("op_smartweave_wallet_last_tx", address);
    }
  }

  class Arweave {
    /** @deprecated */
    get crypto() {
      return new CryptoInterface();
    }

    /** @deprecated */
    get utils() {
      return new ArweaveUtils();
    }

    get ar() {
      return new Ar();
    }

    get wallets() {
      return new Wallets();
    }
  }

  class SmartWeave {
    get transaction() {
      return getInteraction().transaction;
    }

    get block() {
      return getInteraction().block;
    }

    get arweave() {
      return new Arweave();
    }

    // TODO
    get contracts() {}

    get unsafeClient() {
      throw new TypeError("Unsafe client not supported.");
    }
  }

  window.SmartWeave = new SmartWeave();
  window.crypto = crypto;

  // Remove non-deterministic GC dependent V8 globals.
  window.FinalizationRegistry = class FinalizationRegistry {
    #register;
    constructor(fn) {
      this.#register = fn;
    }

    register() {/* Nop */}
  };

  window.WeakRef = class WeakRef {
    #value;
    constructor(value) {
      this.#value = value;
    }

    deref() {
      return this.#value;
    }
  };

  // JSON.stringify is deterministic. Not action required there.
  // https://github.com/nodejs/node/issues/15628#issuecomment-332588533

  // xorshift128+ RNG adapted from https://github.com/AndreasMadsen/xorshift
  const s = 0.69 * Math.pow(2, 32);
  const seed = [
    s,
    s,
    s,
    s,
  ];
  // uint64_t s = [seed ...]
  let _state0U = seed[0] | 0;
  let _state0L = seed[1] | 0;
  let _state1U = seed[2] | 0;
  let _state1L = seed[3] | 0;

  Math.random = function () {
    // uint64_t s1 = s[0]
    var s1U = _state0U, s1L = _state0L;
    // uint64_t s0 = s[1]
    var s0U = _state1U, s0L = _state1L;

    // result = s0 + s1
    var sumL = (s0L >>> 0) + (s1L >>> 0);
    var resU = (s0U + s1U + (sumL / 2 >>> 31)) >>> 0;
    var resL = sumL >>> 0;

    // s[0] = s0
    _state0U = s0U;
    _state0L = s0L;

    // - t1 = [0, 0]
    var t1U = 0, t1L = 0;
    // - t2 = [0, 0]
    var t2U = 0, t2L = 0;

    // s1 ^= s1 << 23;
    // :: t1 = s1 << 23
    var a1 = 23;
    var m1 = 0xFFFFFFFF << (32 - a1);
    t1U = (s1U << a1) | ((s1L & m1) >>> (32 - a1));
    t1L = s1L << a1;
    // :: s1 = s1 ^ t1
    s1U = s1U ^ t1U;
    s1L = s1L ^ t1L;

    // t1 = ( s1 ^ s0 ^ ( s1 >> 17 ) ^ ( s0 >> 26 ) )
    // :: t1 = s1 ^ s0
    t1U = s1U ^ s0U;
    t1L = s1L ^ s0L;
    // :: t2 = s1 >> 18
    var a2 = 18;
    var m2 = 0xFFFFFFFF >>> (32 - a2);
    t2U = s1U >>> a2;
    t2L = (s1L >>> a2) | ((s1U & m2) << (32 - a2));
    // :: t1 = t1 ^ t2
    t1U = t1U ^ t2U;
    t1L = t1L ^ t2L;
    // :: t2 = s0 >> 5
    var a3 = 5;
    var m3 = 0xFFFFFFFF >>> (32 - a3);
    t2U = s0U >>> a3;
    t2L = (s0L >>> a3) | ((s0U & m3) << (32 - a3));
    // :: t1 = t1 ^ t2
    t1U = t1U ^ t2U;
    t1L = t1L ^ t2L;

    // s[1] = t1
    _state1U = t1U;
    _state1L = t1L;

    return resU * 2.3283064365386963e-10 +
      (resL >>> 12) * 2.220446049250313e-16;
  };

  const clonedDate = Date;
  function NewDate(...args) {
    const dateArgs = args.length === 0 ? [1479427200000] : args;
    const instance = new clonedDate(...dateArgs);
    Object.setPrototypeOf(instance, Object.getPrototypeOf(NewDate.prototype));
    return instance;
  }

  NewDate.prototype = Object.create(Date.prototype);
  Object.setPrototypeOf(NewDate, Date);

  NewDate.now = () => 1479427200000; // 2016-11-18 00:00:00.000

  Date = NewDate;

  let step = 0.0;
  window.performance = {};
  window.performance.now = () => {
    const now = step;
    step += 0.1;
    return now;
  };
})(this);
