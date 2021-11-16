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

  class Arweave {
    get crypto() {
      return new CryptoInterface();
    }

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
