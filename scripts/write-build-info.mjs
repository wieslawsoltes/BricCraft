import { writeFile } from "node:fs/promises";
import { join } from "node:path";

const site = process.argv[2];
const commit = process.env.GITHUB_SHA;
if (!site || !/^[0-9a-f]{40}$/.test(commit ?? "")) {
  throw new Error("Expected a site directory and the exact GITHUB_SHA.");
}
await writeFile(join(site, "build-info.json"), JSON.stringify({
  commit,
  repository: process.env.GITHUB_REPOSITORY,
  workflowRun: Number(process.env.GITHUB_RUN_ID),
}, null, 2) + "\n");
