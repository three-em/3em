#![allow(dead_code)]

use crate::utils::hasher;
use deno_crypto::rand::rngs::OsRng;
use rsa::pkcs1::{
  FromRsaPrivateKey, FromRsaPublicKey, ToRsaPrivateKey, ToRsaPublicKey,
};
use rsa::{PaddingScheme, PublicKey, RsaPrivateKey, RsaPublicKey};

pub struct GeneratedPair {
  private_key: Vec<u8>,
  public_key: Vec<u8>,
}

fn get_scheme() -> PaddingScheme {
  PaddingScheme::new_pkcs1v15_sign(Some(rsa::hash::Hash::SHA2_256))
}

pub async fn generate_keypair() -> GeneratedPair {
  let pair = tokio::task::spawn_blocking(move || {
    let mut rng = OsRng;
    let private_key = RsaPrivateKey::new(&mut rng, 2048_usize).unwrap();
    let public_key = RsaPublicKey::from(&private_key);

    let private_key_bytes = private_key.to_pkcs1_der().unwrap();
    let public_key_bytes = public_key.to_pkcs1_der().unwrap();

    GeneratedPair {
      private_key: private_key_bytes.as_ref().to_vec(),
      public_key: public_key_bytes.as_ref().to_vec(),
    }
  })
  .await
  .unwrap();
  pair
}

pub fn to_private_key(
  private_key: Vec<u8>,
) -> rsa::pkcs1::Result<RsaPrivateKey> {
  let bytes = &private_key[..];

  RsaPrivateKey::from_pkcs1_der(bytes)
}

fn to_public_key(public_key: Vec<u8>) -> rsa::pkcs1::Result<RsaPublicKey> {
  let bytes = &public_key[..];

  RsaPublicKey::from_pkcs1_der(bytes)
}

pub fn encrypt(public_key: Vec<u8>, data: &str) -> (Vec<u8>, usize) {
  let mut rng = OsRng;
  let rsa = to_public_key(public_key).unwrap();
  let padding = PaddingScheme::new_pkcs1v15_encrypt();
  let data_bytes = data.as_bytes();
  let enc_data = rsa.encrypt(&mut rng, padding, data_bytes).unwrap();
  let size = &enc_data.len();
  (enc_data, size.to_owned())
}

pub fn decrypt(private_key: Vec<u8>, data: Vec<u8>) -> (Vec<u8>, usize) {
  let padding = PaddingScheme::new_pkcs1v15_encrypt();
  let rsa = to_private_key(private_key).unwrap();
  let dec_data = rsa.decrypt(padding, &data).unwrap();
  let size = &dec_data.len();
  (dec_data, size.to_owned())
}

pub fn sign(private_key: Vec<u8>, data: &str) -> Vec<u8> {
  let private_key = match to_private_key(private_key) {
    Ok(key) => key,
    Err(_) => panic!("Key is invalid"),
  };

  let (scheme, hasher) = (get_scheme(), hasher(data.as_bytes()));

  private_key.sign(scheme, &hasher).unwrap()
}

pub fn verify(public_key: Vec<u8>, signature: Vec<u8>, data: &str) -> bool {
  let public_key = match to_public_key(public_key) {
    Ok(key) => key,
    Err(_) => panic!("Key is invalid"),
  };

  let (scheme, hasher) = (get_scheme(), hasher(data.as_bytes()));

  let verify = public_key.verify(scheme, &hasher, &signature[..]);

  verify.is_ok()
}

impl GeneratedPair {
  pub async fn new() -> GeneratedPair {
    generate_keypair().await
  }

  pub fn public_to_string(&self) -> String {
    String::from_utf8(self.public_key.to_owned()).unwrap()
  }

  pub fn private_to_string(&self) -> String {
    String::from_utf8(self.private_key.to_owned()).unwrap()
  }

  pub fn encrypt(&self, data: &str) -> (Vec<u8>, usize) {
    encrypt(self.public_key.to_owned(), data)
  }

  pub fn decrypt(&self, data: Vec<u8>) -> (Vec<u8>, usize) {
    decrypt(self.private_key.to_owned(), data)
  }

  pub fn sign(&self, data: &str) -> Vec<u8> {
    sign(self.private_key.to_owned(), data)
  }
}

#[cfg(not(debug_assertions))]
#[cfg(test)]
mod tests {
  use crate::node_crypto::{
    decrypt, encrypt, generate_keypair, sign, verify, GeneratedPair,
  };

  #[tokio::test]
  async fn test_encrypt() {
    let keypair = generate_keypair().await;
    let (encrypt, _) = encrypt(keypair.public_key.to_owned(), "Hello Divy");
    let (decrypt, decrypt_len) = decrypt(keypair.private_key, encrypt);
    assert_eq!(
      String::from_utf8(decrypt[..decrypt_len].to_vec()).unwrap(),
      "Hello Divy"
    );
  }

  #[tokio::test]
  async fn test_encrypt_from_internal() {
    let keypair = GeneratedPair::new().await;
    let (encrypt, _) = keypair.encrypt("Hello Divy");
    let (decrypt, decrypt_len) = keypair.decrypt(encrypt);
    assert_eq!(
      String::from_utf8(decrypt[..decrypt_len].to_vec()).unwrap(),
      "Hello Divy"
    );
  }

  #[tokio::test]
  async fn test_signing() {
    let keypair = generate_keypair().await;
    let keypair2 = generate_keypair().await;

    let signed = sign(keypair.private_key, "Hello World!");
    let is_valid = verify(
      keypair.public_key.to_owned(),
      signed.to_owned(),
      "Hello World!",
    );
    assert!(is_valid);
    let is_valid = verify(
      keypair.public_key.to_owned(),
      signed.to_owned(),
      "Hello World",
    );
    assert!(!is_valid);
    let is_valid = verify(keypair2.public_key, signed, "Hello World!");
    assert!(!is_valid);
  }

  #[tokio::test]
  async fn test_signing_internal() {
    let keypair = generate_keypair().await;
    let signed = keypair.sign("Hello World!");
    let is_valid = verify(
      keypair.public_key.to_owned(),
      signed.to_owned(),
      "Hello World!",
    );
    assert!(is_valid);
    let is_valid = verify(keypair.public_key, signed, "Hello World");
    assert!(!is_valid);
  }
}
