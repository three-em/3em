use sha2::Digest;

pub fn decode_base_64(data: String) -> String {
  String::from_utf8(base64::decode(data).unwrap_or_else(|_| vec![]))
    .unwrap_or(String::from(""))
}

pub fn hasher(data: &[u8]) -> Vec<u8> {
  let mut hasher = sha2::Sha256::new();
  hasher.update(data);
  hasher.finalize()[..].to_vec()
}

#[cfg(test)]
mod tests {
  use crate::utils::{u8_array_to_usize, usize_to_u8_array};

  #[tokio::test]
  async fn test_usize_to_u8_array() {
    let message = "Hello".repeat(100000);
    let bytes = message.as_bytes();
    let len = bytes.len();
    assert_eq!(len, 500000 as usize);
    let to_u8_array = usize_to_u8_array(len as u32);
    let expected: [u8; 4] = [32, 161, 7, 0];
    assert_eq!(to_u8_array, expected);
    assert_eq!(500000 as usize, u8_array_to_usize(to_u8_array));
    assert_eq!(usize_to_u8_array(500000), expected.to_owned());
  }
}
