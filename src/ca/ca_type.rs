use digest::Digest;
use dsa::{Signature, SigningKey, VerifyingKey};
use pkcs8::{
    der::{Decode, Encode},
    DecodePrivateKey,
};
use sha2::Sha256;
use signature::{DigestVerifier, RandomizedDigestSigner, SignatureEncoding};

use crate::scp::scp::SCP;

// pub type PublicKey = [u8; 64];
// pub type PublicKey = String;
#[derive(Clone, PartialEq)]
pub struct PublicKey {
    // TODO: remove option
    key: VerifyingKey,
}

impl PublicKey {}

pub type Timestamp = u64;

#[derive(Clone)]
pub struct SCPSignature {
    pk: PublicKey,
    // TODO: remove the option
    sig: Signature,
}

pub fn mock_public_key() -> PublicKey {
    let (verifying_key, _) = mock_public_key_sig();

    PublicKey {
        key: verifying_key.clone(),
    }
}

fn mock_public_key_sig() -> (VerifyingKey, Signature) {
    const OPENSSL_PEM_PRIVATE_KEY: &str = include_str!("../../test_private.pem");
    let signing_key: SigningKey = SigningKey::from_pkcs8_pem(OPENSSL_PEM_PRIVATE_KEY)
        .expect("Failed to decode PEM encoded OpenSSL signing key");
    let verifying_key: &VerifyingKey = signing_key.verifying_key();
    let sig = signing_key
        .sign_digest_with_rng(&mut rand::thread_rng(), Sha256::new().chain_update(b"Ok"));

    (verifying_key.clone(), sig)
}

impl SCPSignature {
    pub fn verify(&self) -> bool {
        self.pk
            .key
            .verify_digest(Sha256::new().chain_update(b"Ok"), &self.sig)
            .is_ok()
    }

    pub fn from_signing_key(signing_key: &SigningKey) -> Self {
        let sig = signing_key
            .sign_digest_with_rng(&mut rand::thread_rng(), Sha256::new().chain_update(b"Ok"));
        SCPSignature {
            pk: PublicKey {
                key: signing_key.verifying_key().clone(),
            },
            sig: sig,
        }
    }

    pub fn test_gen_fake_signature() -> SCPSignature {
        const OPENSSL_PEM_PRIVATE_KEY: &str = include_str!("../../test_private.pem");
        let signing_key: SigningKey = SigningKey::from_pkcs8_pem(OPENSSL_PEM_PRIVATE_KEY)
            .expect("Failed to decode PEM encoded OpenSSL signing key");

        let mut scp_sig = SCPSignature::default();

        let sig = signing_key.sign_digest_with_rng(
            &mut rand::thread_rng(),
            Sha256::new().chain_update(b"Not ok"),
        );
        // bytes[0] = if bytes[0] == 0 {1} else {0};

        scp_sig.sig = sig;
        scp_sig
    }
}

impl Default for SCPSignature {
    // TODO: this is only for dev and mock testing.
    fn default() -> Self {
        let (_, sig) = mock_public_key_sig();

        Self {
            pk: mock_public_key(),
            sig: sig,
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

    #[test]
    fn verify_signature() {
        let intact = SCPSignature::default();
        assert!(intact.verify());
    }

    #[test]
    fn corrupted_signature() {
        let corrupted = SCPSignature::test_gen_fake_signature();
        assert!(!corrupted.verify());
    }
}
