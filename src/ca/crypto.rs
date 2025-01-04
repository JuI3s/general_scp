use std::fmt::{self, Display};

use digest::Digest;
use dsa::{Signature, SigningKey, VerifyingKey};
use pkcs8::{DecodePrivateKey, DecodePublicKey, EncodePublicKey};
use serde::{de, ser, Deserialize, Deserializer, Serialize};
use sha2::Sha256;
use signature::{DigestVerifier, RandomizedDigestSigner};
use syn::token::Pub;

pub const TEST_OPENSSL_PRIVATE_KEY: &str = include_str!("../../test_private.pem");

#[derive(Clone, PartialEq, Debug)]
pub struct PublicKey(pub VerifyingKey);

impl Serialize for PublicKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        match self.0.to_public_key_der() {
            Ok(doc) => {
                let bytes = doc.as_bytes();
                serializer.serialize_bytes(bytes)
            }
            Err(err) => {
                let err_msg = format!("Failed to serialize public key, err: {:?}", err);
                Err(ser::Error::custom(err_msg))
            }
        }
    }
}

impl<'de> Deserialize<'de> for PublicKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let bytes: &[u8] = serde_bytes::deserialize(deserializer)?;
        match VerifyingKey::from_public_key_der(bytes) {
            Ok(key) => Ok(PublicKey(key)),
            Err(err) => {
                let err_msg = format!("Failed to serialize public key, err: {:?}", err);
                Err(de::Error::custom(err_msg))
            }
        }
    }
}

#[derive(Clone, PartialEq)]
pub struct PrivateKey(pub SigningKey);

impl PrivateKey {
    pub fn from_pkcs8_pem(pem: &str) -> Self {
        PrivateKey(
            SigningKey::from_pkcs8_pem(pem)
                .expect("Failed to decode PEM encoded OpenSSL signing key"),
        )
    }

    pub fn public_key(&self) -> PublicKey {
        PublicKey(self.0.verifying_key().clone())
    }
}

// Custom wrapper around verifying key for serialization and deserializatioon.
// Uses DER format to encode the veryifying key to bytes.
#[derive(Debug)]
pub struct SCPVerifyingKeySerdeError {}

impl Display for SCPVerifyingKeySerdeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SCPVerifyingKeySerdeError")
    }
}

impl ser::Error for SCPVerifyingKeySerdeError {
    fn custom<T: Display>(msg: T) -> Self {
        SCPVerifyingKeySerdeError {}
    }
}

impl de::Error for SCPVerifyingKeySerdeError {
    fn custom<T: Display>(msg: T) -> Self {
        SCPVerifyingKeySerdeError {}
    }
}

impl std::error::Error for SCPVerifyingKeySerdeError {}

impl PublicKey {}

#[derive(Clone)]
pub struct SCPSignature {
    pk: PublicKey,
    // TODO: remove the option
    sig: Signature,
}

pub fn mock_public_key() -> PublicKey {
    let (verifying_key, _) = mock_public_key_sig();

    PublicKey(verifying_key.clone())
}

fn mock_public_key_sig() -> (VerifyingKey, Signature) {
    let signing_key: SigningKey = SigningKey::from_pkcs8_pem(TEST_OPENSSL_PRIVATE_KEY)
        .expect("Failed to decode PEM encoded OpenSSL signing key");
    let verifying_key: &VerifyingKey = signing_key.verifying_key();
    let sig = signing_key
        .sign_digest_with_rng(&mut rand::thread_rng(), Sha256::new().chain_update(b"Ok"));

    (verifying_key.clone(), sig)
}

pub fn mock_sig() -> SCPSignature {
    let (_, sig) = mock_public_key_sig();

    SCPSignature {
        pk: mock_public_key(),
        sig,
    }
}

pub fn mock_fake_signature() -> SCPSignature {
    const OPENSSL_PEM_PRIVATE_KEY: &str = include_str!("../../test_private.pem");
    let signing_key = PrivateKey(
        SigningKey::from_pkcs8_pem(OPENSSL_PEM_PRIVATE_KEY)
            .expect("Failed to decode PEM encoded OpenSSL signing key"),
    );

    let mut scp_sig = mock_sig();

    let sig = SCPSignature::sign(&signing_key, b"Not ok").sig;

    scp_sig.sig = sig;
    scp_sig
}

impl SCPSignature {
    pub fn sign(private_key: &PrivateKey, msg: &[u8]) -> Self {
        let sig = private_key
            .0
            .sign_digest_with_rng(&mut rand::thread_rng(), Sha256::new().chain_update(msg));

        SCPSignature {
            pk: PublicKey(private_key.0.verifying_key().clone()),
            sig,
        }
    }

    pub fn verify(&self, msg: &[u8]) -> bool {
        self.pk
            .0
            .verify_digest(Sha256::new().chain_update(msg), &self.sig)
            .is_ok()
    }
}

#[cfg(test)]
mod tests {

    use digest::Digest;
    use dsa::{SigningKey, VerifyingKey};
    use pkcs8::{DecodePrivateKey, DecodePublicKey, EncodePrivateKey, EncodePublicKey, LineEnding};

    use super::*;
    use sha2::Sha256;
    use signature::{DigestVerifier, RandomizedDigestSigner};

    const OPENSSL_PEM_PRIVATE_KEY: &str = include_str!("../../test_private.pem");
    const OPENSSL_PEM_PUBLIC_KEY: &str = include_str!("../../test_public.pem");

    #[test]
    fn serialize_and_deserialize_public_key() {
        let private_key = PrivateKey::from_pkcs8_pem(OPENSSL_PEM_PRIVATE_KEY);
        let public_key = private_key.public_key();

        let serialized = bincode::serialize(&public_key).expect("Failed to serialize public key");
        let deserialized: PublicKey =
            bincode::deserialize(&serialized).expect("Failed to deserialize public key");

        assert_eq!(public_key, deserialized);

        let msg_bytes = "Ok".as_bytes();
        let mut sig = SCPSignature::sign(&private_key, msg_bytes);

        sig.pk = deserialized.clone();
        assert!(sig.verify(msg_bytes));

        sig.pk = public_key;
        assert!(sig.verify(msg_bytes));
    }

    #[test]
    fn decode_encode_openssl_signing_key() {
        // https://github.com/RustCrypto/signatures/blob/e3a163c76b699492541d9b8e78223f60ad22493f/dsa/tests/signing_key.rs#L13

        let signing_key = SigningKey::from_pkcs8_pem(OPENSSL_PEM_PRIVATE_KEY)
            .expect("Failed to decode PEM encoded OpenSSL key");

        let reencoded_signing_key = signing_key
            .to_pkcs8_pem(LineEnding::LF)
            .expect("Failed to encode private key into PEM representation");

        assert_eq!(*reencoded_signing_key, OPENSSL_PEM_PRIVATE_KEY);
        let new_signing_key = SigningKey::from_pkcs8_pem(&reencoded_signing_key)
            .expect("Failed to decode PEM encoded OpenSSL key");

        assert_eq!(signing_key, new_signing_key);
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
    fn sign_and_verify_scp_signature() {
        let private_key = PrivateKey(
            SigningKey::from_pkcs8_pem(OPENSSL_PEM_PRIVATE_KEY)
                .expect("Failed to decode PEM encoded OpenSSL signing key"),
        );

        let signature = SCPSignature::sign(&private_key, b"Ok");
        assert!(signature.verify(b"Ok"));
    }

    #[test]
    fn corrupted_signature() {
        let corrupted = mock_fake_signature();
        assert!(!corrupted.verify(b"Ok"));
    }
}
