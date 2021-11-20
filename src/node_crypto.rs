use openssl::rsa::{Rsa, Padding};
use openssl::sign::{Signer, Verifier};
use openssl::symm::Cipher;
use openssl::pkey::{PKey, Public, Private};
use openssl::error::ErrorStack;
use openssl::ec::EcKey;
use openssl::hash::MessageDigest;

pub struct GeneratedPair {
    private_key: Vec<u8>,
    public_key: Vec<u8>
}

pub fn generate_keypair() -> GeneratedPair {

    let rsa = Rsa::generate(2048).unwrap();

    let private_key: Vec<u8> = rsa.private_key_to_pem().unwrap();
    let public_key: Vec<u8> = rsa.public_key_to_pem_pkcs1().unwrap();

    GeneratedPair {
        private_key,
        public_key
    }
}

pub fn to_private_key(private_key: Vec<u8>) -> Result<Rsa<Private>, ErrorStack>
{
    let rsa_private_key = Rsa::private_key_from_pem(private_key.as_ref());
    rsa_private_key
}

fn to_public_key(public_key: Vec<u8>) -> Result<Rsa<Public>, ErrorStack>
{
    let pkey = Rsa::public_key_from_pem_pkcs1(&public_key.as_ref());
    return pkey;
}

pub fn encrypt(public_key: Vec<u8>, data: &str) -> (Vec<u8>, usize) {
    let rsa = to_public_key(public_key).unwrap();
    let mut buf: Vec<u8> = vec![0; rsa.size() as usize];
    let len = rsa.public_encrypt(data.as_bytes(), &mut buf, Padding::PKCS1).unwrap();

    (buf, len)
}

pub fn decrypt(private_key: Vec<u8>, data: Vec<u8>) -> (Vec<u8>, usize) {
    let rsa = to_private_key(private_key).unwrap();
    let mut buf: Vec<u8> = vec![0; rsa.size() as usize];
    let len = rsa.private_decrypt(&data, &mut buf, Padding::PKCS1).unwrap();

    (buf, len)
}

pub fn sign(private_key: Vec<u8>, data: &str) -> Vec<u8> {

    let private_key = match to_private_key(private_key) {
        Ok(key) => key,
        Err(_) => panic!("Key is invalid")
    };

    let pkey = PKey::from_rsa(private_key).unwrap();

    let mut signer = Signer::new(MessageDigest::sha256(), &pkey).unwrap();
    signer.update(data.as_bytes()).unwrap();
    let signature = signer.sign_to_vec().unwrap();

    signature
}

pub fn verify(public_key: Vec<u8>, signature: Vec<u8>, data: &str) -> bool {

    let public_key = match to_public_key(public_key) {
        Ok(key) => key,
        Err(_) => panic!("Key is invalid")
    };

    let pkey = PKey::from_rsa(public_key).unwrap();

    let mut verifier = Verifier::new(MessageDigest::sha256(), &pkey).unwrap();
    verifier.update(data.as_bytes()).unwrap();
    verifier.verify(&signature.as_ref()).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use crate::node_crypto::{generate_keypair, encrypt, decrypt, sign, verify};

    #[tokio::test]
    async fn test_encrypt() {
        let keypair = generate_keypair();
        let (encrypt, encrypt_len) = encrypt(keypair.public_key.to_owned(), "Hello Divy");
        let (decrypt, decrypt_len) = decrypt(keypair.private_key.to_owned(), encrypt);
        assert_eq!(String::from_utf8(decrypt[..decrypt_len].to_vec()).unwrap(), "Hello Divy");
    }

    #[tokio::test]
    async fn test_signing() {
        let keypair = generate_keypair();
        let keypair2 = generate_keypair();

        let signed = sign(keypair.private_key, "Hello World!");
        let is_valid = verify(keypair.public_key.to_owned(), signed.to_owned(), "Hello World!");
        assert!(is_valid);
        let is_valid = verify(keypair.public_key.to_owned(), signed.to_owned(), "Hello World");
        assert!(!is_valid);
        let is_valid = verify(keypair2.public_key.to_owned(), signed.to_owned(), "Hello World!");
        assert!(!is_valid);
    }

}
