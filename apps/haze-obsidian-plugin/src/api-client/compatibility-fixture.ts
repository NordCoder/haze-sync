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
  AdminFixtureDto,
  ConflictResolutionAction,
  ConflictsResponseDto,
  DeleteFileResponseDto,
  ErrorResponseDto,
  FileMetadataDto,
  PutFileResponseDto,
  ResolveConflictResponseDto,
  ServerInfoDto,
  ChangesResponseDto,
} from "./types";
import {
  isAdminFixtureDto,
  isChangesResponseDto,
  isConflictsResponseDto,
  isDeleteFileResponseDto,
  isErrorResponseDto,
  isFileMetadataDto,
  isPutFileResponseDto,
  isResolveConflictResponseDto,
  isServerInfoDto,
} from "./validators";

export const API_CONTRACT_FIXTURE_SCHEMA_VERSION = 1 as const;

export interface ConflictResolutionFixtureEntry {
  path_conflict_id: string;
  request: { resolution: ConflictResolutionAction };
  response: ResolveConflictResponseDto;
}

export interface ApiVocabularyFixture {
  server_capabilities: string[];
  put_statuses: string[];
  put_ignored_reasons: string[];
  put_rejected_reasons: string[];
  operation_kinds: string[];
  conflict_policies: string[];
  conflict_statuses: string[];
  conflict_resolution_actions: string[];
  conflict_resolve_statuses: string[];
  delete_statuses: string[];
  delete_rejected_reasons: string[];
  public_error_codes: string[];
  server_statuses: string[];
  dependency_readiness_states: string[];
  operational_check_statuses: string[];
  doctor_check_kinds: string[];
  cursor_presence_states: string[];
  adapter_runtime_states: string[];
}

export interface ApiContractFixtureV1 {
  schema_version: 1;
  server_info: ServerInfoDto;
  file_metadata: FileMetadataDto;
  put_file_outcomes: PutFileResponseDto[];
  changes_page: ChangesResponseDto;
  conflict_list_query: { status: "open" };
  conflict_list: ConflictsResponseDto;
  conflict_resolutions: ConflictResolutionFixtureEntry[];
  delete_file_outcomes: DeleteFileResponseDto[];
  public_errors: ErrorResponseDto[];
  admin: AdminFixtureDto;
  vocabulary: ApiVocabularyFixture;
}

export function parseApiContractFixture(value: unknown): ApiContractFixtureV1 {
  if (!isApiContractFixture(value)) {
    throw new Error("API compatibility fixture does not match schema version 1.");
  }
  return value;
}

export function isApiContractFixture(value: unknown): value is ApiContractFixtureV1 {
  if (
    !isExactRecord(value, [
      "schema_version",
      "server_info",
      "file_metadata",
      "put_file_outcomes",
      "changes_page",
      "conflict_list_query",
      "conflict_list",
      "conflict_resolutions",
      "delete_file_outcomes",
      "public_errors",
      "admin",
      "vocabulary",
    ]) ||
    value.schema_version !== API_CONTRACT_FIXTURE_SCHEMA_VERSION ||
    !isServerInfoDto(value.server_info) ||
    !isFileMetadataDto(value.file_metadata) ||
    !Array.isArray(value.put_file_outcomes) ||
    !value.put_file_outcomes.every(isPutFileResponseDto) ||
    !isChangesResponseDto(value.changes_page) ||
    !isExactRecord(value.conflict_list_query, ["status"]) ||
    value.conflict_list_query.status !== "open" ||
    !isConflictsResponseDto(value.conflict_list) ||
    !Array.isArray(value.conflict_resolutions) ||
    !value.conflict_resolutions.every(isConflictResolutionFixtureEntry) ||
    !Array.isArray(value.delete_file_outcomes) ||
    !value.delete_file_outcomes.every(isDeleteFileResponseDto) ||
    !Array.isArray(value.public_errors) ||
    !value.public_errors.every(isErrorResponseDto) ||
    !isAdminFixtureDto(value.admin) ||
    !isVocabularyFixture(value.vocabulary)
  ) {
    return false;
  }

  return (
    hasUniqueStatusCoverage(value.put_file_outcomes, ["accepted", "conflict_saved", "ignored", "rejected"]) &&
    hasUniqueStatusCoverage(value.delete_file_outcomes, ["tombstoned", "not_found", "rejected"]) &&
    hasUniqueResolutionCoverage(value.conflict_resolutions) &&
    value.admin.status_summary.adapter_count === value.admin.adapter_list.total_count &&
    value.admin.adapter_list.total_count === value.admin.adapter_list.adapters.length
  );
}

function isConflictResolutionFixtureEntry(value: unknown): value is ConflictResolutionFixtureEntry {
  return (
    isExactRecord(value, ["path_conflict_id", "request", "response"]) &&
    isNonEmptyString(value.path_conflict_id) &&
    isExactRecord(value.request, ["resolution"]) &&
    includes(CONFLICT_RESOLUTION_ACTIONS, value.request.resolution) &&
    isResolveConflictResponseDto(value.response) &&
    value.response.conflict_id === value.path_conflict_id &&
    value.response.resolution === value.request.resolution
  );
}

function isVocabularyFixture(value: unknown): value is ApiVocabularyFixture {
  if (
    !isExactRecord(value, [
      "server_capabilities",
      "put_statuses",
      "put_ignored_reasons",
      "put_rejected_reasons",
      "operation_kinds",
      "conflict_policies",
      "conflict_statuses",
      "conflict_resolution_actions",
      "conflict_resolve_statuses",
      "delete_statuses",
      "delete_rejected_reasons",
      "public_error_codes",
      "server_statuses",
      "dependency_readiness_states",
      "operational_check_statuses",
      "doctor_check_kinds",
      "cursor_presence_states",
      "adapter_runtime_states",
    ])
  ) {
    return false;
  }

  return (
    sameStringSet(value.server_capabilities, SERVER_CAPABILITIES) &&
    sameStringSet(value.put_statuses, ["accepted", "conflict_saved", "ignored", "rejected"]) &&
    sameStringSet(value.put_ignored_reasons, ["same_content"]) &&
    sameStringSet(value.put_rejected_reasons, PUT_REJECTED_REASONS) &&
    sameStringSet(value.operation_kinds, OPERATION_KINDS) &&
    sameStringSet(value.conflict_policies, CONFLICT_POLICIES) &&
    sameStringSet(value.conflict_statuses, CONFLICT_STATUSES) &&
    sameStringSet(value.conflict_resolution_actions, CONFLICT_RESOLUTION_ACTIONS) &&
    sameStringSet(value.conflict_resolve_statuses, ["resolved"]) &&
    sameStringSet(value.delete_statuses, ["tombstoned", "not_found", "rejected"]) &&
    sameStringSet(value.delete_rejected_reasons, DELETE_REJECTED_REASONS) &&
    sameStringSet(value.public_error_codes, PUBLIC_ERROR_CODES) &&
    sameStringSet(value.server_statuses, SERVER_STATUSES) &&
    sameStringSet(value.dependency_readiness_states, DEPENDENCY_READINESS_STATES) &&
    sameStringSet(value.operational_check_statuses, OPERATIONAL_CHECK_STATUSES) &&
    sameStringSet(value.doctor_check_kinds, DOCTOR_CHECK_KINDS) &&
    sameStringSet(value.cursor_presence_states, CURSOR_PRESENCE_STATES) &&
    sameStringSet(value.adapter_runtime_states, ADAPTER_RUNTIME_STATES)
  );
}

function hasUniqueStatusCoverage(
  values: readonly { status: string }[],
  expected: readonly string[],
): boolean {
  return values.length === expected.length && sameStringSet(values.map((value) => value.status), expected);
}

function hasUniqueResolutionCoverage(values: readonly ConflictResolutionFixtureEntry[]): boolean {
  return (
    values.length === CONFLICT_RESOLUTION_ACTIONS.length &&
    sameStringSet(values.map((value) => value.request.resolution), CONFLICT_RESOLUTION_ACTIONS)
  );
}

function sameStringSet(value: unknown, expected: readonly string[]): boolean {
  if (!Array.isArray(value) || value.some((item) => typeof item !== "string")) {
    return false;
  }
  const actualSet = new Set(value);
  const expectedSet = new Set(expected);
  return actualSet.size === value.length && actualSet.size === expectedSet.size &&
    Array.from(expectedSet).every((item) => actualSet.has(item));
}

function includes<const T extends readonly string[]>(values: T, value: unknown): value is T[number] {
  return typeof value === "string" && values.includes(value as T[number]);
}

function isNonEmptyString(value: unknown): value is string {
  return typeof value === "string" && value.length > 0;
}

function isExactRecord<K extends string>(value: unknown, keys: readonly K[]): value is Record<K, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    return false;
  }
  const actual = Object.keys(value);
  return actual.length === keys.length && actual.every((key) => keys.includes(key as K));
}
