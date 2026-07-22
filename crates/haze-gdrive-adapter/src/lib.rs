//! Google Drive adapter runtime foundation.
//!
//! The crate owns configuration, redaction, provider-safe Drive metadata
//! normalization, fake-first and concrete bounded provider abstractions,
//! adapter-local mapping and cursor state, dependency-free content hashing,
//! full-scan import planning, change-feed reconciliation, Core-to-Drive export
//! planning/apply boundaries, conservative delete-candidate guardrails, OAuth
//! credential/token handling, and a bounded mode-aware HTTP client for
//! Server-owned durable state.
//! Long-running runtime composition remains deferred.

pub mod auth;
pub mod change_feed;
pub mod config;
pub mod delete_guard;
pub mod drive;
pub mod durable_state;
pub mod error;
pub mod export;
pub mod google_http;
pub mod hash;
pub mod identity;
pub mod runtime;
pub mod scan;
pub mod state;

pub use auth::{
    preflight, AccessToken, AuthError, AuthErrorCategory, FakeGoogleProviderClient,
    FakeTokenEndpoint, GoogleAuthClient, GoogleCredentials, GoogleProviderClient, PreflightReport,
    TokenEndpoint, TokenRefreshRequest,
};
pub use change_feed::{
    classify_drive_changes, run_change_feed_cycle, ChangeFeedCycleInput, ChangeFeedCycleOutcome,
    ChangeFeedError, ChangeFeedModelError, ChangePollDebouncer, ChangePollTrigger,
    ChangeProcessingError, ChangeWorkBatch, ChangeWorkItem, ChangeWorkProcessor, CursorStoreError,
    DebouncedPoll, DriveChangeEntry, DriveChangeFeedProvider, DriveChangePage, DriveChangePoll,
    DriveCursorStore, FakeDriveChangeFeedProvider, FullScanFallbackReason,
    InMemoryDriveCursorStore, ProviderBackoffPolicy, RetryDisposition,
};
pub use config::{
    AdapterConfig, AdapterMode, DeleteSafetyConfig, RuntimeIntervals, SecretPath, SecretString,
};
pub use delete_guard::{
    run_delete_reconciliation, CompleteDeleteScan, ConfirmedDeleteCandidate, CoreDeleteError,
    CoreDeleteErrorCategory, CoreDeleteGateway, CoreDeleteGuardBlockReason,
    CoreDeleteGuardDecision, CoreDeleteGuardRequest, CoreDeleteRejectedReason, CoreDeleteRequest,
    CoreDeleteResponse, DeleteBlockReason, DeleteCandidateStateStore, DeleteExecution,
    DeleteMappingState, DeleteModelError, DeleteReconciliationError, DeleteReconciliationInput,
    DeleteReconciliationOutcome, DeleteRejection, DeleteSafetyNotice, DeleteScanIssue,
    DeleteScanObservation, DeleteStateError, FakeCoreDeleteGateway,
    InMemoryDeleteCandidateStateStore, ManualDeleteUnlockAvailability, MovedProviderIdentity,
    RecoveredDeleteCandidate, GDRIVE_ADAPTER_ID,
};
pub use drive::{
    classify_drive_metadata, normalize_drive_metadata, DriveEntryClassification, DriveEntryKind,
    DriveMetadata, DriveMutationOutcome, DriveProvider, DriveUpdateRequest, DriveUploadRequest,
    FakeDriveProvider, NormalizedDriveEntry, ProviderError, ProviderErrorCategory,
    SupportedFileType, UnsupportedEntryReason,
};
pub use durable_state::{
    CollectedGDriveState, DurableStateClient, DurableStateClientError, DurableStateErrorCategory,
    HttpClientPolicy, HttpDurableStateClient, HttpMethod, HttpRequest, HttpResponse, HttpTransport,
    HttpTransportError, ModeAwareDurableStateClient, UreqHttpTransport,
};
pub use error::{ConfigError, ConfigErrorCategory, RuntimeError, RuntimeErrorCategory};
pub use export::{
    cursor_after_page, echo_observation_for_mapping, plan_core_export, run_export_cycle,
    CoreClientError, CoreClientErrorCategory, CoreExportChange, CoreExportClient, CoreExportPage,
    CoreFileContent, DriveCreateExportRequest, DriveCreateTarget, DriveExportError,
    DriveExportErrorCategory, DriveExportProvider, DriveExportReceipt, DriveTrashExportRequest,
    DriveUpdateExportRequest, ExportCycleInput, ExportCycleOutcome, ExportError, ExportExecution,
    ExportModelError, ExportPlanItem, ExportRetryDisposition, ExportRetryPolicy, ExportSkipReason,
    ExportStateError, ExportStateStore, FakeCoreExportClient, FakeDriveExportProvider,
    InMemoryExportStateStore, VerifiedExportSource, DEFAULT_CORE_CHANGE_PAGE_LIMIT,
};
pub use google_http::{
    AuthorizationCodeRequest, GoogleAccessTokenProvider, GoogleClock, GoogleDriveHttpClient,
    GoogleHttpMethod, GoogleHttpPolicy, GoogleHttpRequest, GoogleHttpResponse, GoogleHttpTransport,
    GoogleHttpTransportError, GoogleOAuthConfig, GoogleOAuthHttpEndpoint, OfflineTokenGrant,
    SystemGoogleClock, UreqGoogleHttpTransport,
};
pub use hash::ContentSha256;
pub use identity::{AdapterIdentity, AdapterIdentitySource, ENV_ADAPTER_ID};
pub use runtime::{AdapterRuntime, RuntimeState, StartupStatus};
pub use scan::{
    plan_full_scan, CoreUploadRequest, DeleteCandidatePlan, FullScanError, FullScanInput,
    FullScanPlan, ImportChangeKind, ImportExecution, PlannedImport, ScanSkipReason, SkippedImport,
    SkippedScanEntry, UnchangedDriveEntry,
};
pub use state::{
    CoreChangeCursor, CoreStateObservation, DriveChangeCursor, DriveEchoObservation,
    DriveStateObservation, EchoDecision, EchoGuard, EchoGuardEntry, GDriveMapping,
    MappingPersistenceBoundary, SafeTimestamp, StateError, StatePersistencePolicy, VaultPath,
};
