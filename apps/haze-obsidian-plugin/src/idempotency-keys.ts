import type { ConflictResolutionAction } from "./api-client";

export type LocalMutationKind = "upload" | "delete";

const RANDOM_BYTES_LENGTH = 16;

export function generateLocalIdempotencyKey(kind: LocalMutationKind, createdAt = new Date()): string {
  return `obsidian-plugin:${kind}:${createdAt.toISOString()}:${randomHex(RANDOM_BYTES_LENGTH)}`;
}

export function generateConflictResolutionIdempotencyKey(
  action: ConflictResolutionAction,
  createdAt = new Date(),
): string {
  return `obsidian-plugin:resolve-conflict:${action}:${createdAt.toISOString()}:${randomHex(RANDOM_BYTES_LENGTH)}`;
}

function randomHex(length: number): string {
  const bytes = new Uint8Array(length);
  crypto.getRandomValues(bytes);

  return Array.from(bytes, (byte) => byte.toString(16).padStart(2, "0")).join("");
}
