use sha2::Digest;

pub fn decode_base_64(data: String) -> String {
  String::from_utf8(base64::decode(data).unwrap_or_else(|_| vec![]))
    .unwrap_or_else(|_| String::from(""))
}

pub fn hasher(data: &[u8]) -> Vec<u8> {
  let mut hasher = sha2::Sha256::new();
  hasher.update(data);
  hasher.finalize()[..].to_vec()
}
