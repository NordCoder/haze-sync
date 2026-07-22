export type VaultPath = string;

export type VaultPathExclusionReason =
  | "empty_path"
  | "absolute_path"
  | "parent_traversal"
  | "null_byte"
  | "internal_directory"
  | "temporary_file"
  | "unsupported_extension";

export interface IncludedVaultPath {
  included: true;
  path: VaultPath;
  extension: string;
}

export interface ExcludedVaultPath {
  included: false;
  originalPath: string;
  reason: VaultPathExclusionReason;
}

export type VaultPathClassification = IncludedVaultPath | ExcludedVaultPath;

const INTERNAL_TOP_LEVEL_SEGMENTS = new Set([
  ".git",
  ".haze-sync",
  ".obsidian",
  ".trash",
  "_haze",
  "_haze_conflicts",
  "_haze_internal",
  "node_modules",
]);

const SUPPORTED_EXTENSIONS = new Set(["canvas", "md", "txt"]);
const TEMPORARY_SUFFIXES = [".bak", ".part", ".swp", ".tmp", "~"];
const TEMPORARY_NAMES = new Set([".DS_Store", "Thumbs.db"]);

export function classifyVaultPath(rawPath: string): VaultPathClassification {
  if (rawPath.includes("\0")) {
    return exclude(rawPath, "null_byte");
  }

  if (rawPath.startsWith("/") || /^[A-Za-z]:[\\/]/u.test(rawPath)) {
    return exclude(rawPath, "absolute_path");
  }

  const normalized = rawPath.replace(/\\/gu, "/");
  if (normalized.length === 0) {
    return exclude(rawPath, "empty_path");
  }

  const segments = normalized.split("/").filter((segment) => segment.length > 0);
  if (segments.length === 0) {
    return exclude(rawPath, "empty_path");
  }

  if (segments.some((segment) => segment === "." || segment === "..")) {
    return exclude(rawPath, "parent_traversal");
  }

  if (INTERNAL_TOP_LEVEL_SEGMENTS.has(segments[0])) {
    return exclude(rawPath, "internal_directory");
  }

  const fileName = segments[segments.length - 1];
  if (isTemporaryFileName(fileName)) {
    return exclude(rawPath, "temporary_file");
  }

  const extension = extensionOf(fileName);
  if (!SUPPORTED_EXTENSIONS.has(extension)) {
    return exclude(rawPath, "unsupported_extension");
  }

  return {
    included: true,
    path: segments.join("/"),
    extension,
  };
}

export function isSupportedVaultPath(rawPath: string): boolean {
  return classifyVaultPath(rawPath).included;
}

export function exclusionReasonLabel(reason: VaultPathExclusionReason): string {
  switch (reason) {
    case "empty_path":
      return "empty path";
    case "absolute_path":
      return "absolute path";
    case "parent_traversal":
      return "parent traversal";
    case "null_byte":
      return "null byte";
    case "internal_directory":
      return "internal directory";
    case "temporary_file":
      return "temporary file";
    case "unsupported_extension":
      return "unsupported extension";
  }
}

function exclude(originalPath: string, reason: VaultPathExclusionReason): ExcludedVaultPath {
  return {
    included: false,
    originalPath,
    reason,
  };
}

function isTemporaryFileName(fileName: string): boolean {
  if (TEMPORARY_NAMES.has(fileName)) {
    return true;
  }

  return TEMPORARY_SUFFIXES.some((suffix) => fileName.endsWith(suffix));
}

function extensionOf(fileName: string): string {
  const lastDot = fileName.lastIndexOf(".");
  if (lastDot < 0 || lastDot === fileName.length - 1) {
    return "";
  }

  return fileName.slice(lastDot + 1).toLowerCase();
}
