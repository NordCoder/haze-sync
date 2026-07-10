export type RevisionId = string;
export type ConflictId = string;
export type ContentHash = string;
export type VaultPath = string;
export type AdapterId = string;
export type Timestamp = string;
export type Sequence = number;

export const SERVER_CAPABILITIES = [
  "sha256",
  "operation_log",
  "tombstones",
  "conflicts",
  "conflict_center",
  "batch_changes",
] as const;
export type ServerCapability = (typeof SERVER_CAPABILITIES)[number];

export const OPERATION_KINDS = [
  "upsert_file",
  "delete_file",
  "restore_file",
  "conflict_created",
  "conflict_resolved",
  "backup_created",
] as const;
export type ChangeKind = (typeof OPERATION_KINDS)[number];

export const CONFLICT_POLICIES = [
  "preserve_both",
  "current_wins_with_incoming_backup",
] as const;
export type ConflictPolicy = (typeof CONFLICT_POLICIES)[number];

export const CONFLICT_STATUSES = ["open", "resolved", "ignored"] as const;
export type ConflictStatus = (typeof CONFLICT_STATUSES)[number];

export const CONFLICT_RESOLUTION_ACTIONS = [
  "accept_current",
  "accept_conflict",
  "keep_both",
  "mark_resolved",
] as const;
export type ConflictResolutionAction = (typeof CONFLICT_RESOLUTION_ACTIONS)[number];

export const PUT_REJECTED_REASONS = [
  "hash_mismatch",
  "stale_base_revision",
  "ignored_path",
  "validation_error",
  "idempotency_conflict",
] as const;
export type PutRejectedReason = (typeof PUT_REJECTED_REASONS)[number];

export const DELETE_REJECTED_REASONS = [
  "stale_base_revision",
  "unsafe_delete",
  "idempotency_conflict",
  "validation_error",
] as const;
export type DeleteRejectedReason = (typeof DELETE_REJECTED_REASONS)[number];

export const PUBLIC_ERROR_CODES = [
  "invalid_request",
  "invalid_path",
  "validation_error",
  "unauthorized",
  "missing_token",
  "invalid_token",
  "forbidden_role",
  "not_found",
  "conflict",
  "idempotency_conflict",
  "payload_too_large",
  "rate_limited",
  "unsafe_delete",
  "ignored_path",
  "internal_error",
] as const;
export type PublicErrorCode = (typeof PUBLIC_ERROR_CODES)[number];

export const SERVER_STATUSES = ["not_ready", "ready", "degraded", "maintenance"] as const;
export type ServerStatus = (typeof SERVER_STATUSES)[number];

export const DEPENDENCY_READINESS_STATES = ["unknown", "ready", "not_ready"] as const;
export type DependencyReadinessState = (typeof DEPENDENCY_READINESS_STATES)[number];

export const OPERATIONAL_CHECK_STATUSES = [
  "passed",
  "failed",
  "skipped",
  "not_run",
  "placeholder",
] as const;
export type OperationalCheckStatus = (typeof OPERATIONAL_CHECK_STATUSES)[number];

export const DOCTOR_CHECK_KINDS = [
  "database",
  "object_store",
  "operation_log",
  "adapter_registry",
] as const;
export type DoctorCheckKind = (typeof DOCTOR_CHECK_KINDS)[number];

export const CURSOR_PRESENCE_STATES = ["unknown", "absent", "present"] as const;
export type CursorPresenceState = (typeof CURSOR_PRESENCE_STATES)[number];

export const ADAPTER_RUNTIME_STATES = [
  "unknown",
  "disabled",
  "idle",
  "running",
  "degraded",
] as const;
export type AdapterRuntimeState = (typeof ADAPTER_RUNTIME_STATES)[number];

export interface ServerInfoDto {
  server_id: string;
  protocol_version: number;
  max_upload_bytes: number;
  capabilities: ServerCapability[];
}

export interface ChangesRequest {
  since: Sequence;
  limit: number;
}

export interface ChangesResponseDto {
  from_seq: Sequence;
  to_seq: Sequence;
  has_more: boolean;
  changes: ChangeDto[];
}

export interface ChangeDto {
  seq: Sequence;
  kind: ChangeKind;
  path: VaultPath;
  revision_id?: RevisionId;
  content_sha256?: ContentHash;
  size_bytes?: number;
  tombstone_id?: string;
  conflict_id?: ConflictId;
  updated_by: AdapterId;
  updated_at: Timestamp;
}

export interface FileMetadataDto {
  path: VaultPath;
  revision_id: RevisionId;
  content_sha256: ContentHash;
  size_bytes: number;
  updated_by: AdapterId;
  updated_at: Timestamp;
}

export interface FileDownload {
  metadata: Partial<Pick<FileMetadataDto, "path" | "revision_id" | "content_sha256" | "size_bytes">>;
  body: ArrayBuffer;
  contentType: string | null;
}

export interface PutFileRequest {
  path: VaultPath;
  body: BodyInit;
  contentHash: ContentHash;
  baseRevisionId: RevisionId | null;
  idempotencyKey: string;
  contentType?: string;
}

export interface PutFileAcceptedDto {
  status: "accepted";
  path: VaultPath;
  revision_id: RevisionId;
  seq: Sequence;
}

export interface PutFileConflictSavedDto {
  status: "conflict_saved";
  path: VaultPath;
  conflict_id: ConflictId;
  materialized_path: VaultPath;
  policy_applied: ConflictPolicy;
  seq: Sequence;
}

export interface PutFileIgnoredDto {
  status: "ignored";
  reason: "same_content";
  path: VaultPath;
}

export interface PutFileRejectedDto {
  status: "rejected";
  reason: PutRejectedReason;
  path: VaultPath;
}

export type PutFileResponseDto =
  | PutFileAcceptedDto
  | PutFileConflictSavedDto
  | PutFileIgnoredDto
  | PutFileRejectedDto;

export interface DeleteFileRequest {
  path: VaultPath;
  baseRevisionId: RevisionId | null;
  idempotencyKey: string;
}

export interface DeleteFileTombstonedDto {
  status: "tombstoned";
  path: VaultPath;
  tombstone_id: string;
  seq: Sequence;
  retention_until: Timestamp;
}

export interface DeleteFileNotFoundDto {
  status: "not_found";
  path: VaultPath;
}

export interface DeleteFileRejectedDto {
  status: "rejected";
  reason: DeleteRejectedReason;
  path: VaultPath;
}

export type DeleteFileResponseDto =
  | DeleteFileTombstonedDto
  | DeleteFileNotFoundDto
  | DeleteFileRejectedDto;

export interface ConflictListRequest {
  status?: "open";
}

export interface ConflictsResponseDto {
  conflicts: ConflictDto[];
}

export interface ConflictDto {
  conflict_id: ConflictId;
  original_path: VaultPath;
  conflict_path: VaultPath;
  current_revision_id: RevisionId;
  conflict_revision_id?: RevisionId;
  incoming_revision_id?: RevisionId;
  source_adapter_id: AdapterId;
  policy_applied: ConflictPolicy;
  status: ConflictStatus;
  created_at?: Timestamp;
  updated_at?: Timestamp;
}

export interface ResolveConflictRequest {
  conflictId: ConflictId;
  resolution: ConflictResolutionAction;
  idempotencyKey: string;
}

export interface ResolveConflictResponseDto {
  status: "resolved";
  conflict_id: ConflictId;
  resolution: ConflictResolutionAction;
  seq: Sequence;
}

export type SafeErrorDetails = Record<string, string[]> | string[];

export interface ErrorResponseDto {
  error: {
    code: PublicErrorCode;
    message: string;
    request_id?: string;
    details?: SafeErrorDetails;
  };
}

export interface PauseSummaryDto {
  supported: boolean;
  active: boolean | null;
}

export interface StatusSummaryDto {
  server_status: ServerStatus;
  db_readiness_state: DependencyReadinessState;
  object_store_readiness_state: DependencyReadinessState;
  last_operation_sequence: Sequence;
  adapter_count: number;
  pause: PauseSummaryDto;
}

export interface AdapterCursorSummaryDto {
  last_core_seq: Sequence;
  last_success_at: Timestamp;
  has_external_cursor: boolean;
}

export interface AdapterSummaryDto {
  adapter_id: AdapterId;
  role: string;
  mode: string;
  enabled: boolean;
  last_seen_at: Timestamp | null;
  cursor: AdapterCursorSummaryDto | null;
}

export interface AdapterListDto {
  total_count: number;
  adapters: AdapterSummaryDto[];
}

export interface DoctorCheckDto {
  check: DoctorCheckKind;
  status: OperationalCheckStatus;
  readiness_state: DependencyReadinessState;
  checked_at?: Timestamp;
}

export interface DoctorStatusDto {
  server_status: ServerStatus;
  checks: DoctorCheckDto[];
}

export interface AdapterRuntimeDto {
  observation_status: OperationalCheckStatus;
  state: AdapterRuntimeState | null;
  pause: PauseSummaryDto;
}

export interface AdapterOperationalSummaryDto {
  adapter: AdapterSummaryDto;
  runtime: AdapterRuntimeDto;
}

export interface AdminFixtureDto {
  status_summary: StatusSummaryDto;
  adapter_list: AdapterListDto;
  doctor_statuses: DoctorStatusDto[];
  adapter_operational_summaries: AdapterOperationalSummaryDto[];
}
