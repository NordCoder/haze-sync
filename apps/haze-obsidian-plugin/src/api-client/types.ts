export type RevisionId = string;
export type OperationId = string;
export type ConflictId = string;
export type ContentHash = string;
export type VaultPath = string;

export interface ServerInfoDto {
  server_version?: string;
  api_version?: string;
  instance_id?: string;
  features?: string[];
}

export interface ChangesRequest {
  since?: number;
  limit?: number;
}

export interface ChangesResponseDto {
  changes: ChangeDto[];
  next_since?: number;
  has_more?: boolean;
}

export type ChangeKind = "upsert" | "delete" | "conflict";

export interface ChangeDto {
  sequence: number;
  operation_id?: OperationId;
  kind: ChangeKind | string;
  path: VaultPath;
  revision_id?: RevisionId | null;
  content_hash?: ContentHash | null;
  occurred_at?: string;
}

export interface FileMetadataDto {
  path: VaultPath;
  revision_id: RevisionId;
  content_hash: ContentHash;
  size_bytes?: number;
  updated_at?: string;
}

export interface FileDownload {
  metadata: Partial<FileMetadataDto>;
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

export interface PutFileResponseDto {
  status: "accepted" | "same_content" | "conflict_saved" | "rejected" | string;
  path: VaultPath;
  revision_id?: RevisionId | null;
  conflict_id?: ConflictId | null;
  content_hash?: ContentHash | null;
}

export interface DeleteFileRequest {
  path: VaultPath;
  baseRevisionId: RevisionId | null;
  idempotencyKey: string;
}

export interface DeleteFileResponseDto {
  status: "accepted" | "tombstoned" | "same_content" | "conflict_saved" | "rejected" | string;
  path: VaultPath;
  revision_id?: RevisionId | null;
  tombstone_id?: string | null;
  conflict_id?: ConflictId | null;
}

export interface ConflictListRequest {
  status?: "open" | "resolved";
}

export interface ConflictsResponseDto {
  conflicts: ConflictDto[];
}

export interface ConflictDto {
  id: ConflictId;
  original_path: VaultPath;
  conflict_path?: VaultPath | null;
  status: "open" | "resolved" | string;
  current_revision_id?: RevisionId | null;
  conflict_revision_id?: RevisionId | null;
  source_adapter_id?: string | null;
  created_at?: string;
  resolved_at?: string | null;
}

export type ConflictResolutionAction =
  | "accept_current"
  | "accept_conflict"
  | "keep_both"
  | "mark_resolved";

export interface ResolveConflictRequest {
  conflictId: ConflictId;
  action: ConflictResolutionAction;
  idempotencyKey: string;
}

export interface ResolveConflictResponseDto {
  status: "resolved" | "kept_both" | "rejected" | string;
  conflict_id: ConflictId;
  revision_id?: RevisionId | null;
}

export interface ErrorResponseDto {
  error: {
    code: string;
    message: string;
  };
}
