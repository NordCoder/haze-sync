const CANONICAL_CONTENT_HASH_PATTERN = /^sha256:[0-9a-f]{64}$/u;
const LEGACY_CONTENT_HASH_PATTERN = /^[0-9a-f]{64}$/u;

export function isCanonicalContentHash(value: unknown): value is string {
  return typeof value === "string" && CANONICAL_CONTENT_HASH_PATTERN.test(value);
}

export function normalizeStoredContentHash(value: unknown): string | undefined {
  if (isCanonicalContentHash(value)) {
    return value;
  }
  if (typeof value === "string" && LEGACY_CONTENT_HASH_PATTERN.test(value)) {
    return `sha256:${value}`;
  }
  return undefined;
}

export async function sha256ContentHash(body: ArrayBuffer): Promise<string> {
  const digest = await crypto.subtle.digest("SHA-256", body);
  const bytes = new Uint8Array(digest);
  const hex = Array.from(bytes, (byte) => byte.toString(16).padStart(2, "0")).join("");
  return `sha256:${hex}`;
}
