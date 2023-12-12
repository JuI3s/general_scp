use digest::Digest;
use dsa::{Signature, SigningKey, VerifyingKey};
use sha2::Sha256;
use signature::{DigestVerifier, RandomizedDigestSigner};

// pub type PublicKey = [u8; 64];
// pub type PublicKey = String;
#[derive(Clone, PartialEq)]
pub struct PublicKey {
    // TODO: remove option
    key: Option<VerifyingKey>,
}

impl PublicKey {}

pub type Timestamp = u64;

#[derive(Clone)]
pub struct SCPSignature {
    pk: PublicKey,
    // TODO: remove the option
    sig: Option<Signature>,
}

pub fn mock_public_key() -> PublicKey {
    PublicKey { key: None }
}

impl SCPSignature {
    pub fn verify(&self) -> bool {
        if let Some(key) = &self.pk.key {
            if let Some(sig) = &self.sig {
                return key
                    .verify_digest(Sha256::new().chain_update(b"Ok"), sig)
                    .is_ok();
            }
        }

        false
    }

    pub fn from_signing_key(signing_key: &SigningKey) -> Self {
        let sig = signing_key
            .sign_digest_with_rng(&mut rand::thread_rng(), Sha256::new().chain_update(b"Ok"));
        SCPSignature {
            pk: PublicKey {
                key: Some(signing_key.verifying_key().clone()),
            },
            sig: Some(sig),
        }
    }
}

impl Default for SCPSignature {
    // TODO: this is only for dev and mock testing.
    fn default() -> Self {
        Self {
            pk: mock_public_key(),
            sig: None,
            // sig: Signature::from_der(include_str!("../../test_signature.der").as_bytes()).expect(""),
        }
    }
}

#[cfg(test)]
mod tests {

    use digest::Digest;
    use dsa::{Components, KeySize, SigningKey, VerifyingKey};
    use pkcs8::{DecodePrivateKey, DecodePublicKey, EncodePrivateKey, EncodePublicKey, LineEnding};
    use sha1::Sha1;
    use sha2::Sha256;
    use signature::{DigestVerifier, RandomizedDigestSigner, SignatureEncoding};
    use std::{fs::File, io::Write};

    use super::*;

    const OPENSSL_PEM_PRIVATE_KEY: &str = include_str!("../../test_private.pem");
    const OPENSSL_PEM_PUBLIC_KEY: &str = include_str!("../../test_public.pem");

    #[test]
    fn decode_encode_openssl_signing_key() {
        // https://github.com/RustCrypto/signatures/blob/e3a163c76b699492541d9b8e78223f60ad22493f/dsa/tests/signing_key.rs#L13

        let signing_key = SigningKey::from_pkcs8_pem(OPENSSL_PEM_PRIVATE_KEY)
            .expect("Failed to decode PEM encoded OpenSSL key");

        let reencoded_signing_key = signing_key
            .to_pkcs8_pem(LineEnding::LF)
            .expect("Failed to encode private key into PEM representation");

        assert_eq!(*reencoded_signing_key, OPENSSL_PEM_PRIVATE_KEY);
    }

    #[test]
    fn sign_digital_signature_from_test_pem_files() {
        // Basic example showing how to use the library.
        // https://github.com/RustCrypto/signatures/blob/master/dsa/tests/signature.rs

        let msg = b"hello world";
        let signing_key = SigningKey::from_pkcs8_pem(OPENSSL_PEM_PRIVATE_KEY)
            .expect("Failed to decode PEM encoded OpenSSL signing key");

        let signature: Signature = signing_key
            .sign_digest_with_rng(&mut rand::thread_rng(), Sha256::new().chain_update(msg));

        let verifying_key = signing_key.verifying_key();

        assert!(verifying_key
            .verify_digest(Sha256::new().chain_update(msg), &signature)
            .is_ok());

        let veryifing_key_pem = verifying_key
            .to_public_key_pem(LineEnding::LF)
            .expect("Error converting public key to pem");

        assert_eq!(veryifing_key_pem, OPENSSL_PEM_PUBLIC_KEY);

        let verifying_key_der = verifying_key.to_public_key_der().expect("");
        let verifying_key_bytes = verifying_key_der.as_bytes();
        let verifying_key_from_bytes = VerifyingKey::from_public_key_der(verifying_key_bytes)
            .expect("Error converting to public key from bytes");
        assert!(verifying_key_from_bytes
            .verify_digest(Sha256::new().chain_update(msg), &signature)
            .is_ok());
    }

    #[test]
    fn sign_scp_signature() {
        let signing_key = SigningKey::from_pkcs8_pem(OPENSSL_PEM_PRIVATE_KEY)
            .expect("Failed to decode PEM encoded OpenSSL signing key");

        let signature = SCPSignature::from_signing_key(&signing_key);
        assert!(signature.verify());
    }
}
