use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Algorithm, Argon2, Params, Version,
};
use haze_sync_common::Sha256 as CommonSha256;
use haze_sync_storage::control_plane::SecretDigest;
use sha2::{Digest, Sha256};

const ARGON2_MEMORY_KIB: u32 = 65_536;
const ARGON2_ITERATIONS: u32 = 3;
const ARGON2_PARALLELISM: u32 = 1;
const CREDENTIAL_SECRET_BYTES: usize = 32;
const CREDENTIAL_SALT_BYTES: usize = 16;
const IDENTIFIER_RANDOM_BYTES: usize = 16;

#[derive(Clone)]
pub(crate) struct ControlPlaneSecrets {
    credential_issuance_pepper: [u8; 32],
    operational_idempotency_pepper: [u8; 32],
    control_idempotency_pepper: [u8; 32],
    confirmation_pepper: [u8; 32],
    lease_pepper: [u8; 32],
}

impl ControlPlaneSecrets {
    pub(crate) fn derive_from_server_secret(server_secret: &[u8]) -> Self {
        Self {
            credential_issuance_pepper: derive_key(server_secret, b"credential-issuance-v1"),
            operational_idempotency_pepper: derive_key(
                server_secret,
                b"operational-idempotency-v1",
            ),
            control_idempotency_pepper: derive_key(server_secret, b"control-idempotency-v1"),
            confirmation_pepper: derive_key(server_secret, b"job-confirmation-v1"),
            lease_pepper: derive_key(server_secret, b"executor-lease-v1"),
        }
    }

    #[cfg(test)]
    pub(crate) fn deterministic_test() -> Self {
        Self::derive_from_server_secret(b"haze-sync-control-plane-test-secret")
    }

    pub(crate) fn credential_idempotency_digest(
        &self,
        raw_key: &str,
    ) -> Result<SecretDigest, CryptoError> {
        secret_digest(hmac_sha256(
            &self.credential_issuance_pepper,
            raw_key.as_bytes(),
        ))
    }

    pub(crate) fn operational_idempotency_digest(
        &self,
        raw_key: &str,
    ) -> Result<SecretDigest, CryptoError> {
        secret_digest(hmac_sha256(
            &self.operational_idempotency_pepper,
            raw_key.as_bytes(),
        ))
    }

    pub(crate) fn control_idempotency_key(&self, namespace: &str, raw_key: &str) -> String {
        let digest = hmac_sha256(&self.control_idempotency_pepper, raw_key.as_bytes());
        format!("control:v1:{namespace}:{}", lower_hex(&digest))
    }

    pub(crate) fn confirmation_digest(
        &self,
        operation_id: &str,
        confirmation: &str,
    ) -> Result<SecretDigest, CryptoError> {
        let mut input = Vec::with_capacity(operation_id.len() + confirmation.len() + 1);
        input.extend_from_slice(operation_id.as_bytes());
        input.push(0);
        input.extend_from_slice(confirmation.as_bytes());
        secret_digest(hmac_sha256(&self.confirmation_pepper, &input))
    }

    pub(crate) fn lease_digest(
        &self,
        operation_id: &str,
        lease_token: &str,
    ) -> Result<SecretDigest, CryptoError> {
        let mut input = Vec::with_capacity(operation_id.len() + lease_token.len() + 1);
        input.extend_from_slice(operation_id.as_bytes());
        input.push(0);
        input.extend_from_slice(lease_token.as_bytes());
        secret_digest(hmac_sha256(&self.lease_pepper, &input))
    }
}

impl std::fmt::Debug for ControlPlaneSecrets {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("ControlPlaneSecrets([REDACTED])")
    }
}

#[derive(Clone, Eq, PartialEq)]
pub(crate) struct IssuedCredential {
    pub(crate) credential_id: String,
    pub(crate) plaintext: String,
    pub(crate) encoded_verifier: String,
}

impl std::fmt::Debug for IssuedCredential {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("IssuedCredential")
            .field("credential_id", &self.credential_id)
            .field("plaintext", &"[REDACTED]")
            .field("encoded_verifier", &"[REDACTED]")
            .finish()
    }
}

pub(crate) fn issue_credential() -> Result<IssuedCredential, CryptoError> {
    let credential_id = random_identifier("cred_")?;
    let mut secret_bytes = [0_u8; CREDENTIAL_SECRET_BYTES];
    getrandom::fill(&mut secret_bytes).map_err(|_| CryptoError::RandomUnavailable)?;
    let secret = lower_hex(&secret_bytes);
    let plaintext = format!("hs1.{credential_id}.{secret}");

    let mut salt_bytes = [0_u8; CREDENTIAL_SALT_BYTES];
    getrandom::fill(&mut salt_bytes).map_err(|_| CryptoError::RandomUnavailable)?;
    let salt = SaltString::encode_b64(&salt_bytes).map_err(|_| CryptoError::HashingFailed)?;
    let params = Params::new(
        ARGON2_MEMORY_KIB,
        ARGON2_ITERATIONS,
        ARGON2_PARALLELISM,
        None,
    )
    .map_err(|_| CryptoError::HashingFailed)?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let encoded_verifier = argon2
        .hash_password(secret.as_bytes(), &salt)
        .map_err(|_| CryptoError::HashingFailed)?
        .to_string();

    Ok(IssuedCredential {
        credential_id,
        plaintext,
        encoded_verifier,
    })
}

pub(crate) fn verify_argon2id_secret(secret: &str, encoded_verifier: &str) -> bool {
    let Ok(parsed) = PasswordHash::new(encoded_verifier) else {
        return false;
    };
    if parsed.algorithm.as_str() != "argon2id" {
        return false;
    }
    let Ok(params) = Params::try_from(&parsed) else {
        return false;
    };
    if params.m_cost() < ARGON2_MEMORY_KIB
        || params.t_cost() < ARGON2_ITERATIONS
        || params.p_cost() < ARGON2_PARALLELISM
    {
        return false;
    }
    Argon2::default()
        .verify_password(secret.as_bytes(), &parsed)
        .is_ok()
}

pub(crate) fn random_identifier(prefix: &str) -> Result<String, CryptoError> {
    let mut bytes = [0_u8; IDENTIFIER_RANDOM_BYTES];
    getrandom::fill(&mut bytes).map_err(|_| CryptoError::RandomUnavailable)?;
    Ok(format!("{prefix}{}", lower_hex(&bytes)))
}

pub(crate) fn random_secret(prefix: &str) -> Result<String, CryptoError> {
    let mut bytes = [0_u8; CREDENTIAL_SECRET_BYTES];
    getrandom::fill(&mut bytes).map_err(|_| CryptoError::RandomUnavailable)?;
    Ok(format!("{prefix}{}", lower_hex(&bytes)))
}

pub(crate) fn request_sha256(parts: &[&str]) -> Result<CommonSha256, CryptoError> {
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update(part.as_bytes());
        hasher.update([0]);
    }
    CommonSha256::parse(&lower_hex(&hasher.finalize())).map_err(|_| CryptoError::HashingFailed)
}

pub(crate) fn request_secret_digest(parts: &[&str]) -> Result<SecretDigest, CryptoError> {
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update(part.as_bytes());
        hasher.update([0]);
    }
    secret_digest(hasher.finalize())
}


pub(crate) fn constant_time_digest_eq(left: &SecretDigest, right: &SecretDigest) -> bool {
    let left = left.as_str().as_bytes();
    let right = right.as_str().as_bytes();
    if left.len() != right.len() {
        return false;
    }
    left.iter()
        .zip(right.iter())
        .fold(0_u8, |difference, (left, right)| difference | (left ^ right))
        == 0
}

pub(crate) fn legacy_sha256_digest(token: &str) -> Result<SecretDigest, CryptoError> {
    secret_digest(Sha256::digest(token.as_bytes()))
}

fn derive_key(secret: &[u8], domain: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"haze-sync.control-plane.key-derivation.v1");
    hasher.update([0]);
    hasher.update(domain);
    hasher.update([0]);
    hasher.update(secret);
    hasher.finalize().into()
}

fn hmac_sha256(key: &[u8], data: &[u8]) -> [u8; 32] {
    const BLOCK_SIZE: usize = 64;
    let mut normalized = [0_u8; BLOCK_SIZE];
    if key.len() > BLOCK_SIZE {
        let digest = Sha256::digest(key);
        normalized[..32].copy_from_slice(&digest);
    } else {
        normalized[..key.len()].copy_from_slice(key);
    }
    let mut inner_pad = [0x36_u8; BLOCK_SIZE];
    let mut outer_pad = [0x5c_u8; BLOCK_SIZE];
    for index in 0..BLOCK_SIZE {
        inner_pad[index] ^= normalized[index];
        outer_pad[index] ^= normalized[index];
    }
    let mut inner = Sha256::new();
    inner.update(inner_pad);
    inner.update(data);
    let inner_digest = inner.finalize();
    let mut outer = Sha256::new();
    outer.update(outer_pad);
    outer.update(inner_digest);
    outer.finalize().into()
}

fn secret_digest(bytes: impl AsRef<[u8]>) -> Result<SecretDigest, CryptoError> {
    SecretDigest::parse(lower_hex(bytes.as_ref())).map_err(|_| CryptoError::HashingFailed)
}

pub(crate) fn lower_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CryptoError {
    RandomUnavailable,
    HashingFailed,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn issued_credentials_use_hs1_and_required_argon2_parameters() {
        let issued = issue_credential().unwrap();
        let mut parts = issued.plaintext.split('.');
        assert_eq!(parts.next(), Some("hs1"));
        assert_eq!(parts.next(), Some(issued.credential_id.as_str()));
        let secret = parts.next().unwrap();
        assert_eq!(secret.len(), 64);
        assert!(parts.next().is_none());
        assert!(verify_argon2id_secret(secret, &issued.encoded_verifier));
        assert!(!verify_argon2id_secret("wrong-secret", &issued.encoded_verifier));
        assert!(!format!("{issued:?}").contains(secret));
    }

    #[test]
    fn peppers_and_generated_secrets_are_redacted() {
        let secrets = ControlPlaneSecrets::deterministic_test();
        assert_eq!(format!("{secrets:?}"), "ControlPlaneSecrets([REDACTED])");
        let first = secrets.control_idempotency_key("maintenance", "raw-key");
        let second = secrets.control_idempotency_key("maintenance", "raw-key");
        assert_eq!(first, second);
        assert!(!first.contains("raw-key"));
        assert_ne!(random_secret("lease_").unwrap(), random_secret("lease_").unwrap());
    }
}
