import { TFile, Vault } from "obsidian";

import { LocalFileFact, readLocalFileFact } from "./local-file-facts";
import { ExcludedVaultPath, classifyVaultPath } from "./vault-paths";

export interface VaultScanError {
  path: string;
  message: string;
}

export interface VaultScanResult {
  scannedAt: string;
  facts: LocalFileFact[];
  excluded: ExcludedVaultPath[];
  errors: VaultScanError[];
}

export class VaultScanner {
  constructor(private readonly vault: Vault) {}

  async scan(): Promise<VaultScanResult> {
    const scannedAt = new Date().toISOString();
    const facts: LocalFileFact[] = [];
    const excluded: ExcludedVaultPath[] = [];
    const errors: VaultScanError[] = [];

    for (const file of this.vault.getFiles()) {
      const classification = classifyVaultPath(file.path);
      if (!classification.included) {
        excluded.push(classification);
        continue;
      }

      try {
        const result = await readLocalFileFact(this.vault, file as TFile);
        if (result.fact !== undefined) {
          facts.push(result.fact);
        }
        if (result.excluded !== undefined) {
          excluded.push(result.excluded);
        }
      } catch {
        errors.push({
          path: classification.path,
          message: "Could not read file through Obsidian vault API.",
        });
      }
    }

    facts.sort((left, right) => left.path.localeCompare(right.path));
    excluded.sort((left, right) => left.originalPath.localeCompare(right.originalPath));
    errors.sort((left, right) => left.path.localeCompare(right.path));

    return {
      scannedAt,
      facts,
      excluded,
      errors,
    };
  }
}
