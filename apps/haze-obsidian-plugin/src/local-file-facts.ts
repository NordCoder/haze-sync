import { TFile, Vault } from "obsidian";

import { ExcludedVaultPath, IncludedVaultPath, classifyVaultPath } from "./vault-paths";

export interface LocalFileFact {
  path: string;
  extension: string;
  contentHash: string;
  sizeBytes: number;
  mtime: number;
}

export interface LocalFileFactResult {
  fact?: LocalFileFact;
  excluded?: ExcludedVaultPath;
}

export async function readLocalFileFact(vault: Vault, file: TFile): Promise<LocalFileFactResult> {
  const classification = classifyVaultPath(file.path);
  if (!classification.included) {
    return { excluded: classification };
  }

  const body = await vault.readBinary(file);

  return {
    fact: {
      path: classification.path,
      extension: classification.extension,
      contentHash: await sha256Hex(body),
      sizeBytes: file.stat.size,
      mtime: file.stat.mtime,
    },
  };
}

export function factFromEventPath(rawPath: string): IncludedVaultPath | ExcludedVaultPath {
  return classifyVaultPath(rawPath);
}

async function sha256Hex(body: ArrayBuffer): Promise<string> {
  const digest = await crypto.subtle.digest("SHA-256", body);
  const bytes = new Uint8Array(digest);

  return Array.from(bytes, (byte) => byte.toString(16).padStart(2, "0")).join("");
}
