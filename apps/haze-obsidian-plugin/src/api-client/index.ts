export { HazeSyncApiClient } from "./client";
export type { HazeSyncApiClientOptions, HttpTransport } from "./client";
export {
  API_CONTRACT_FIXTURE_SCHEMA_VERSION,
  isApiContractFixture,
  parseApiContractFixture,
} from "./compatibility-fixture";
export type {
  ApiContractFixtureV1,
  ApiVocabularyFixture,
  ConflictResolutionFixtureEntry,
} from "./compatibility-fixture";
export {
  ApiClientError,
  createConfigurationError,
  createHttpError,
  createInvalidResponseError,
  createOfflineError,
  mapHttpStatusToCategory,
} from "./errors";
export type { ApiErrorCategory, SafeApiErrorSummary } from "./errors";
export {
  apiClientConfigFromSettings,
  assertConfiguredServer,
  assertSameOrigin,
  buildApiHeaders,
  buildApiUrl,
  buildFilePathUrl,
  encodeVaultPath,
} from "./request";
export type { ApiClientConfig, RequestHeadersInput } from "./request";
export {
  ADAPTER_RUNTIME_STATES,
  CONFLICT_POLICIES,
  CONFLICT_RESOLUTION_ACTIONS,
  CONFLICT_STATUSES,
  CURSOR_PRESENCE_STATES,
  DELETE_REJECTED_REASONS,
  DEPENDENCY_READINESS_STATES,
  DOCTOR_CHECK_KINDS,
  OPERATIONAL_CHECK_STATUSES,
  OPERATION_KINDS,
  PUBLIC_ERROR_CODES,
  PUT_REJECTED_REASONS,
  SERVER_CAPABILITIES,
  SERVER_STATUSES,
} from "./types";
export type {
  AdapterOperationalSummaryDto,
  AdapterRuntimeState,
  AdminFixtureDto,
  ChangeDto,
  ChangeKind,
  ChangesRequest,
  ChangesResponseDto,
  ConflictDto,
  ConflictId,
  ConflictListRequest,
  ConflictPolicy,
  ConflictResolutionAction,
  ConflictStatus,
  ConflictsResponseDto,
  ContentHash,
  DeleteFileRequest,
  DeleteFileResponseDto,
  DoctorStatusDto,
  ErrorResponseDto,
  FileDownload,
  FileMetadataDto,
  PublicErrorCode,
  PutFileRequest,
  PutFileResponseDto,
  ResolveConflictRequest,
  ResolveConflictResponseDto,
  RevisionId,
  Sequence,
  ServerInfoDto,
  StatusSummaryDto,
  VaultPath,
} from "./types";
export {
  isAdminFixtureDto,
  isChangeDto,
  isChangesResponseDto,
  isConflictDto,
  isConflictsResponseDto,
  isDeleteFileResponseDto,
  isErrorResponseDto,
  isFileMetadataDto,
  isPutFileResponseDto,
  isResolveConflictResponseDto,
  isServerInfoDto,
} from "./validators";
