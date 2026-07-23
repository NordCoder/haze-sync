import { mkdirSync, writeFileSync } from "node:fs";
import path from "node:path";

const obsidianStubDir = path.join(process.cwd(), ".test-dist", "node_modules", "obsidian");
mkdirSync(obsidianStubDir, { recursive: true });
writeFileSync(
  path.join(obsidianStubDir, "index.js"),
  '"use strict";\nclass TFile {}\nmodule.exports = { TFile };\n',
  "utf8",
);

require("./api-compatibility.test");
require("./api-client.test");
require("./server-compatibility-e2e.test");
require("./sync-contracts.test");
