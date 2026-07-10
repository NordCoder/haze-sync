import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import path from "node:path";
import test from "node:test";

import {
  CONFLICT_RESOLUTION_ACTIONS,
  parseApiContractFixture,
} from "../src/api-client";

const VENDORED_FIXTURE_PATH = path.join(process.cwd(), "tests/fixtures/api-contract-v1.json");
const WORKSPACE_FIXTURE_PATH = path.resolve(
  process.cwd(),
  "../../crates/haze-sync-api/fixtures/api-contract-v1.json",
);

function readJson(filePath: string): unknown {
  return JSON.parse(readFileSync(filePath, "utf8")) as unknown;
}

test("vendored API V1 fixture matches the strict TypeScript DTO mirror", () => {
  const raw = readJson(VENDORED_FIXTURE_PATH);
  const parsed = parseApiContractFixture(raw);

  assert.equal(parsed.schema_version, 1);
  assert.deepEqual(JSON.parse(JSON.stringify(parsed)), raw);
  assert.deepEqual(
    new Set(parsed.vocabulary.conflict_resolution_actions),
    new Set(CONFLICT_RESOLUTION_ACTIONS),
  );
});

test("vendored fixture remains identical to the workspace canonical fixture after API fan-in", () => {
  if (!existsSync(WORKSPACE_FIXTURE_PATH)) {
    return;
  }

  const vendored = readJson(VENDORED_FIXTURE_PATH);
  const canonical = readJson(WORKSPACE_FIXTURE_PATH);
  assert.deepEqual(vendored, canonical);
  parseApiContractFixture(canonical);
});

test("fixture parser rejects unknown fields and vocabulary drift", () => {
  const raw = readJson(VENDORED_FIXTURE_PATH) as Record<string, unknown>;
  assert.throws(() => parseApiContractFixture({ ...raw, unexpected: true }));

  const vocabulary = raw.vocabulary as Record<string, unknown>;
  assert.throws(() =>
    parseApiContractFixture({
      ...raw,
      vocabulary: {
        ...vocabulary,
        operation_kinds: ["upsert_file", "unknown_operation"],
      },
    }),
  );
});
