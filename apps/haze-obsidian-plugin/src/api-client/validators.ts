import {
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
import type {
  AdapterOperationalSummaryDto,
  AdapterSummaryDto,
  AdminFixtureDto,
  ChangeDto,
  ChangesResponseDto,
  ConflictDto,
  ConflictsResponseDto,
  DeleteFileResponseDto,
  DoctorStatusDto,
  ErrorResponseDto,
  FileMetadataDto,
  PutFileResponseDto,
  ResolveConflictResponseDto,
  ServerInfoDto,
  StatusSummaryDto,
} from "./types";

export function isServerInfoDto(value: unknown): value is ServerInfoDto {
  return (
    isExactRecord(value, ["server_id", "protocol_version", "max_upload_bytes", "capabilities"]) &&
    isNonEmptyString(value.server_id) &&
    isSafeNonNegativeInteger(value.protocol_version) &&
    isSafeNonNegativeInteger(value.max_upload_bytes) &&
    isUniqueLiteralArray(value.capabilities, SERVER_CAPABILITIES)
  );
}

export function isFileMetadataDto(value: unknown): value is FileMetadataDto {
  return (
    isExactRecord(value, ["path", "revision_id", "content_sha256", "size_bytes", "updated_by", "updated_at"]) &&
    isNonEmptyString(value.path) &&
    isNonEmptyString(value.revision_id) &&
    isContentHash(value.content_sha256) &&
    isSafeNonNegativeInteger(value.size_bytes) &&
    isNonEmptyString(value.updated_by) &&
    isTimestamp(value.updated_at)
  );
}

export function isChangesResponseDto(value: unknown): value is ChangesResponseDto {
  return (
    isExactRecord(value, ["from_seq", "to_seq", "has_more", "changes"]) &&
    isSafeNonNegativeInteger(value.from_seq) &&
    isSafeNonNegativeInteger(value.to_seq) &&
    value.to_seq >= value.from_seq &&
    typeof value.has_more === "boolean" &&
    Array.isArray(value.changes) &&
    value.changes.every(isChangeDto)
  );
}

export function isChangeDto(value: unknown): value is ChangeDto {
  if (
    !isRecordWithAllowedKeys(value, [
      "seq",
      "kind",
      "path",
      "revision_id",
      "content_sha256",
      "size_bytes",
      "tombstone_id",
      "conflict_id",
      "updated_by",
      "updated_at",
    ]) ||
    !isSafeNonNegativeInteger(value.seq) ||
    !isLiteral(value.kind, OPERATION_KINDS) ||
    !isNonEmptyString(value.path) ||
    !isOptionalNonEmptyString(value.revision_id) ||
    !isOptionalContentHash(value.content_sha256) ||
    !isOptionalSafeNonNegativeInteger(value.size_bytes) ||
    !isOptionalNonEmptyString(value.tombstone_id) ||
    !isOptionalNonEmptyString(value.conflict_id) ||
    !isNonEmptyString(value.updated_by) ||
    !isTimestamp(value.updated_at)
  ) {
    return false;
  }

  switch (value.kind) {
    case "upsert_file":
    case "restore_file":
      return (
        isNonEmptyString(value.revision_id) &&
        isContentHash(value.content_sha256) &&
        isSafeNonNegativeInteger(value.size_bytes)
      );
    case "delete_file":
      return isNonEmptyString(value.tombstone_id);
    case "conflict_created":
    case "conflict_resolved":
      return isNonEmptyString(value.conflict_id);
    case "backup_created":
      return true;
  }
}

export function isPutFileResponseDto(value: unknown): value is PutFileResponseDto {
  if (!isRecord(value) || !isLiteral(value.status, ["accepted", "conflict_saved", "ignored", "rejected"] as const)) {
    return false;
  }

  switch (value.status) {
    case "accepted":
      return (
        isExactRecord(value, ["status", "path", "revision_id", "seq"]) &&
        isNonEmptyString(value.path) &&
        isNonEmptyString(value.revision_id) &&
        isSafeNonNegativeInteger(value.seq)
      );
    case "conflict_saved":
      return (
        isExactRecord(value, ["status", "path", "conflict_id", "materialized_path", "policy_applied", "seq"]) &&
        isNonEmptyString(value.path) &&
        isNonEmptyString(value.conflict_id) &&
        isNonEmptyString(value.materialized_path) &&
        isLiteral(value.policy_applied, CONFLICT_POLICIES) &&
        isSafeNonNegativeInteger(value.seq)
      );
    case "ignored":
      return (
        isExactRecord(value, ["status", "reason", "path"]) &&
        value.reason === "same_content" &&
        isNonEmptyString(value.path)
      );
    case "rejected":
      return (
        isExactRecord(value, ["status", "reason", "path"]) &&
        isLiteral(value.reason, PUT_REJECTED_REASONS) &&
        isNonEmptyString(value.path)
      );
  }
}

export function isDeleteFileResponseDto(value: unknown): value is DeleteFileResponseDto {
  if (!isRecord(value) || !isLiteral(value.status, ["tombstoned", "not_found", "rejected"] as const)) {
    return false;
  }

  switch (value.status) {
    case "tombstoned":
      return (
        isExactRecord(value, ["status", "path", "tombstone_id", "seq", "retention_until"]) &&
        isNonEmptyString(value.path) &&
        isNonEmptyString(value.tombstone_id) &&
        isSafeNonNegativeInteger(value.seq) &&
        isTimestamp(value.retention_until)
      );
    case "not_found":
      return isExactRecord(value, ["status", "path"]) && isNonEmptyString(value.path);
    case "rejected":
      return (
        isExactRecord(value, ["status", "reason", "path"]) &&
        isLiteral(value.reason, DELETE_REJECTED_REASONS) &&
        isNonEmptyString(value.path)
      );
  }
}

export function isConflictsResponseDto(value: unknown): value is ConflictsResponseDto {
  return (
    isExactRecord(value, ["conflicts"]) &&
    Array.isArray(value.conflicts) &&
    value.conflicts.every(isConflictDto)
  );
}

export function isConflictDto(value: unknown): value is ConflictDto {
  return (
    isRecordWithAllowedKeys(value, [
      "conflict_id",
      "original_path",
      "conflict_path",
      "current_revision_id",
      "conflict_revision_id",
      "incoming_revision_id",
      "source_adapter_id",
      "policy_applied",
      "status",
      "created_at",
      "updated_at",
    ]) &&
    isNonEmptyString(value.conflict_id) &&
    isNonEmptyString(value.original_path) &&
    isNonEmptyString(value.conflict_path) &&
    isNonEmptyString(value.current_revision_id) &&
    isOptionalNonEmptyString(value.conflict_revision_id) &&
    isOptionalNonEmptyString(value.incoming_revision_id) &&
    isNonEmptyString(value.source_adapter_id) &&
    isLiteral(value.policy_applied, CONFLICT_POLICIES) &&
    isLiteral(value.status, CONFLICT_STATUSES) &&
    isOptionalTimestamp(value.created_at) &&
    isOptionalTimestamp(value.updated_at)
  );
}

export function isResolveConflictResponseDto(value: unknown): value is ResolveConflictResponseDto {
  return (
    isExactRecord(value, ["status", "conflict_id", "resolution", "seq"]) &&
    value.status === "resolved" &&
    isNonEmptyString(value.conflict_id) &&
    isLiteral(value.resolution, CONFLICT_RESOLUTION_ACTIONS) &&
    isSafeNonNegativeInteger(value.seq)
  );
}

export function isErrorResponseDto(value: unknown): value is ErrorResponseDto {
  if (!isExactRecord(value, ["error"]) || !isRecordWithAllowedKeys(value.error, ["code", "message", "request_id", "details"])) {
    return false;
  }

  return (
    isLiteral(value.error.code, PUBLIC_ERROR_CODES) &&
    typeof value.error.message === "string" &&
    isOptionalNonEmptyString(value.error.request_id) &&
    isOptionalSafeDetails(value.error.details)
  );
}

export function isAdminFixtureDto(value: unknown): value is AdminFixtureDto {
  return (
    isExactRecord(value, ["status_summary", "adapter_list", "doctor_statuses", "adapter_operational_summaries"]) &&
    isStatusSummaryDto(value.status_summary) &&
    isExactRecord(value.adapter_list, ["total_count", "adapters"]) &&
    isSafeNonNegativeInteger(value.adapter_list.total_count) &&
    Array.isArray(value.adapter_list.adapters) &&
    value.adapter_list.adapters.every(isAdapterSummaryDto) &&
    Array.isArray(value.doctor_statuses) &&
    value.doctor_statuses.every(isDoctorStatusDto) &&
    Array.isArray(value.adapter_operational_summaries) &&
    value.adapter_operational_summaries.every(isAdapterOperationalSummaryDto)
  );
}

function isStatusSummaryDto(value: unknown): value is StatusSummaryDto {
  return (
    isExactRecord(value, [
      "server_status",
      "db_readiness_state",
      "object_store_readiness_state",
      "last_operation_sequence",
      "adapter_count",
      "pause",
    ]) &&
    isLiteral(value.server_status, SERVER_STATUSES) &&
    isLiteral(value.db_readiness_state, DEPENDENCY_READINESS_STATES) &&
    isLiteral(value.object_store_readiness_state, DEPENDENCY_READINESS_STATES) &&
    isSafeNonNegativeInteger(value.last_operation_sequence) &&
    isSafeNonNegativeInteger(value.adapter_count) &&
    isPauseSummary(value.pause)
  );
}

function isAdapterSummaryDto(value: unknown): value is AdapterSummaryDto {
  return (
    isExactRecord(value, ["adapter_id", "role", "mode", "enabled", "last_seen_at", "cursor"]) &&
    isNonEmptyString(value.adapter_id) &&
    isNonEmptyString(value.role) &&
    isNonEmptyString(value.mode) &&
    typeof value.enabled === "boolean" &&
    (value.last_seen_at === null || isTimestamp(value.last_seen_at)) &&
    (value.cursor === null || isAdapterCursor(value.cursor))
  );
}

function isDoctorStatusDto(value: unknown): value is DoctorStatusDto {
  return (
    isExactRecord(value, ["server_status", "checks"]) &&
    isLiteral(value.server_status, SERVER_STATUSES) &&
    Array.isArray(value.checks) &&
    value.checks.every(
      (check) =>
        isRecordWithAllowedKeys(check, ["check", "status", "readiness_state", "checked_at"]) &&
        isLiteral(check.check, DOCTOR_CHECK_KINDS) &&
        isLiteral(check.status, OPERATIONAL_CHECK_STATUSES) &&
        isLiteral(check.readiness_state, DEPENDENCY_READINESS_STATES) &&
        isOptionalTimestamp(check.checked_at),
    )
  );
}

function isAdapterOperationalSummaryDto(value: unknown): value is AdapterOperationalSummaryDto {
  return (
    isExactRecord(value, ["adapter", "runtime"]) &&
    isAdapterSummaryDto(value.adapter) &&
    isExactRecord(value.runtime, ["observation_status", "state", "pause"]) &&
    isLiteral(value.runtime.observation_status, OPERATIONAL_CHECK_STATUSES) &&
    (value.runtime.state === null || isLiteral(value.runtime.state, ADAPTER_RUNTIME_STATES)) &&
    isPauseSummary(value.runtime.pause)
  );
}

function isAdapterCursor(value: unknown): boolean {
  return (
    isExactRecord(value, ["last_core_seq", "last_success_at", "has_external_cursor"]) &&
    isSafeNonNegativeInteger(value.last_core_seq) &&
    isTimestamp(value.last_success_at) &&
    typeof value.has_external_cursor === "boolean"
  );
}

function isPauseSummary(value: unknown): boolean {
  return (
    isExactRecord(value, ["supported", "active"]) &&
    typeof value.supported === "boolean" &&
    (value.active === null || typeof value.active === "boolean")
  );
}

function isOptionalSafeDetails(value: unknown): boolean {
  if (value === undefined) {
    return true;
  }
  if (Array.isArray(value)) {
    return value.every((item) => typeof item === "string");
  }
  if (!isRecord(value)) {
    return false;
  }
  return Object.values(value).every(
    (messages) => Array.isArray(messages) && messages.every((message) => typeof message === "string"),
  );
}

function isContentHash(value: unknown): value is string {
  return typeof value === "string" && /^sha256:[0-9a-f]{64}$/u.test(value);
}

function isOptionalContentHash(value: unknown): boolean {
  return value === undefined || isContentHash(value);
}

function isTimestamp(value: unknown): value is string {
  return typeof value === "string" && value.length > 0 && Number.isFinite(Date.parse(value));
}

function isOptionalTimestamp(value: unknown): boolean {
  return value === undefined || isTimestamp(value);
}

function isNonEmptyString(value: unknown): value is string {
  return typeof value === "string" && value.length > 0;
}

function isOptionalNonEmptyString(value: unknown): boolean {
  return value === undefined || isNonEmptyString(value);
}

function isSafeNonNegativeInteger(value: unknown): value is number {
  return typeof value === "number" && Number.isSafeInteger(value) && value >= 0;
}

function isOptionalSafeNonNegativeInteger(value: unknown): boolean {
  return value === undefined || isSafeNonNegativeInteger(value);
}

function isLiteral<const T extends readonly string[]>(value: unknown, values: T): value is T[number] {
  return typeof value === "string" && values.includes(value as T[number]);
}

function isUniqueLiteralArray<const T extends readonly string[]>(value: unknown, values: T): value is T[number][] {
  return (
    Array.isArray(value) &&
    value.every((item) => isLiteral(item, values)) &&
    new Set(value).size === value.length
  );
}

function isExactRecord<K extends string>(value: unknown, keys: readonly K[]): value is Record<K, unknown> {
  return isRecord(value) && hasExactKeys(value, keys);
}

function isRecordWithAllowedKeys<K extends string>(value: unknown, keys: readonly K[]): value is Record<K, unknown> {
  return isRecord(value) && Object.keys(value).every((key) => keys.includes(key as K));
}

function hasExactKeys<K extends string>(value: Record<string, unknown>, keys: readonly K[]): boolean {
  const actual = Object.keys(value);
  return actual.length === keys.length && actual.every((key) => keys.includes(key as K));
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
