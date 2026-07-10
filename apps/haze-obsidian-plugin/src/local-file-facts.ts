import { TFile, Vault } from "obsidian";

import { sha256ContentHash } from "./content-hash";
import { ExcludedVaultPath, IncludedVaultPath, classifyVaultPath } from "./vault-paths";

export { sha256ContentHash } from "./content-hash";

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
      contentHash: await sha256ContentHash(body),
      sizeBytes: file.stat.size,
      mtime: file.stat.mtime,
    },
  };
}

export function factFromEventPath(rawPath: string): IncludedVaultPath | ExcludedVaultPath {
  return classifyVaultPath(rawPath);
}
