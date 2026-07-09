export { HazeSyncApiClient } from "./client";
export type { HazeSyncApiClientOptions, HttpTransport } from "./client";
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
export type {
  ChangeDto,
  ChangeKind,
  ChangesRequest,
  ChangesResponseDto,
  ConflictDto,
  ConflictId,
  ConflictListRequest,
  ConflictResolutionAction,
  ConflictsResponseDto,
  ContentHash,
  DeleteFileRequest,
  DeleteFileResponseDto,
  ErrorResponseDto,
  FileDownload,
  FileMetadataDto,
  OperationId,
  PutFileRequest,
  PutFileResponseDto,
  ResolveConflictRequest,
  ResolveConflictResponseDto,
  RevisionId,
  ServerInfoDto,
  VaultPath,
} from "./types";
