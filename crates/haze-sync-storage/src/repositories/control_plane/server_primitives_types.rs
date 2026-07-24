/// One exact credential lifecycle mutation performed as part of an atomic rotation.
#[derive(Clone, Debug, PartialEq)]
pub struct CredentialRotationMutation {
    pub credential_id: String,
    pub expected_credential_version: i64,
    pub expected_status: String,
    pub update: CredentialLifecycleUpdate,
}

/// Caller-selected inputs for one complete credential-set rotation transaction.
///
/// Storage validates exact CAS identities and durable set invariants, but does not
/// choose grace duration, revocation policy, authorization, or public response data.
#[derive(Clone, PartialEq)]
pub struct CredentialSetRotationInput {
    pub expected_principal_version: i64,
    pub expected_credential_set_generation: i64,
    pub new_credential: NewCredential,
    pub affected_credentials: Vec<CredentialRotationMutation>,
}

impl fmt::Debug for CredentialSetRotationInput {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CredentialSetRotationInput")
            .field("expected_principal_version", &self.expected_principal_version)
            .field(
                "expected_credential_set_generation",
                &self.expected_credential_set_generation,
            )
            .field("new_credential", &self.new_credential)
            .field("affected_credentials", &self.affected_credentials)
            .finish()
    }
}

/// Result of an atomic rotation. Verifier material remains redacted by row formatting.
#[derive(Clone, PartialEq)]
pub struct CredentialSetRotationResult {
    pub principal: PrincipalRow,
    pub new_credential: CredentialRow,
    pub affected_credentials: Vec<CredentialRow>,
}

impl fmt::Debug for CredentialSetRotationResult {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CredentialSetRotationResult")
            .field("principal", &self.principal)
            .field("new_credential", &self.new_credential)
            .field("affected_credentials", &self.affected_credentials)
            .finish()
    }
}
