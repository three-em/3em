(function (window) {
  const { crypto } = window.__bootstrap.crypto;
  const { subtle } = crypto;
  function getContract() {
    const data = Deno.core.opSync("op_smartweave_init");
    getContract = () => data;
    return data;
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
      return bufferTob64Url(stringToBuffer(string));
    }

    b64UrlToBuffer(b64UrlString) {
      return Uint8Array.from(atob(b64UrlString), c => c.charCodeAt(0));
    }

    bufferTob64(buffer) {
      return B64js.fromByteArray(new Uint8Array(buffer));
    }

    bufferTob64Url(buffer) {
      return b64UrlEncode(bufferTob64(buffer));
    }

    b64UrlEncode(b64UrlString) {
      return b64UrlString
        .replace(/\+/g, "-")
        .replace(/\//g, "_")
        .replace(/\=/g, "");
    }

    b64UrlDecode(b64UrlString) {
      b64UrlString = b64UrlString.replace(/\-/g, "+").replace(/\_/g, "/");
      let padding;
      b64UrlString.length % 4 == 0
        ? (padding = 0)
        : (padding = 4 - (b64UrlString.length % 4));
      return b64UrlString.concat("=".repeat(padding));
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
