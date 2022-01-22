use ring::rand;
use ring::signature::Ed25519KeyPair;

/// Generate a random key pair.
pub fn random() -> Ed25519KeyPair {
    let rng = rand::SystemRandom::new();        // generate random number
    let pkcs8_bytes = Ed25519KeyPair::generate_pkcs8(&rng).unwrap();    // generate new key pair and return the pair
    Ed25519KeyPair::from_pkcs8(pkcs8_bytes.as_ref().into()).unwrap()    // 验证公钥私钥是否一致
}
