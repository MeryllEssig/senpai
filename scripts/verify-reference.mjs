#!/usr/bin/env node

import { readFileSync } from "node:fs";
import { resolve } from "node:path";

const root = resolve(import.meta.dirname, "..");

function parseJsonc(path) {
  const source = readFileSync(path, "utf8");
  let output = "";
  let quoted = false;
  let escaped = false;

  for (let index = 0; index < source.length; index += 1) {
    const character = source[index];
    const next = source[index + 1];

    if (quoted) {
      output += character;
      if (escaped) escaped = false;
      else if (character === "\\") escaped = true;
      else if (character === '"') quoted = false;
      continue;
    }
    if (character === '"') {
      quoted = true;
      output += character;
    } else if (character === "/" && next === "/") {
      while (index < source.length && source[index] !== "\n") index += 1;
      output += "\n";
    } else if (character === "/" && next === "*") {
      index += 2;
      while (index < source.length && !(source[index] === "*" && source[index + 1] === "/")) index += 1;
      index += 1;
    } else {
      output += character;
    }
  }
  return JSON.parse(output.replace(/,\s*([}\]])/g, "$1"));
}

function assert(condition, message) {
  if (!condition) throw new Error(message);
}

function assertReference(manifest) {
  assert(manifest.version === 1, "reference manifest must be version 1");
  assert(manifest.$schema === "https://aimanager.dev/schema/v1/aimanager.schema.json", "reference manifest must declare the v1 schema URI");

  const repoIds = new Set(Object.keys(manifest.repos ?? {}));
  const environmentIds = new Set(Object.keys(manifest.environments ?? {}));
  const hostingIds = new Set(Object.keys(manifest.code_hosting?.instances ?? {}));
  const storeIds = new Set(Object.keys(manifest.data_stores ?? {}));

  for (const [id, repo] of Object.entries(manifest.repos ?? {})) {
    assert(!repo.path.startsWith("/") && !repo.path.split("/").includes(".."), `repo ${id} must have a relative, normalized path`);
    for (const dependency of repo.depends_on ?? []) assert(repoIds.has(dependency), `repo ${id} depends on unknown repo ${dependency}`);
    for (const hostingId of Object.keys(repo.hosting ?? {})) assert(hostingIds.has(hostingId), `repo ${id} names unknown hosting instance ${hostingId}`);
  }

  const visiting = new Set();
  const visited = new Set();
  function visitRepo(id) {
    assert(!visiting.has(id), `repo dependency cycle includes ${id}`);
    if (visited.has(id)) return;
    visiting.add(id);
    for (const dependency of manifest.repos[id].depends_on ?? []) visitRepo(dependency);
    visiting.delete(id);
    visited.add(id);
  }
  for (const id of repoIds) visitRepo(id);

  for (const [id, environment] of Object.entries(manifest.environments ?? {})) {
    if (environment.repo) assert(repoIds.has(environment.repo), `environment ${id} names unknown repo ${environment.repo}`);
  }

  for (const [id, store] of Object.entries(manifest.data_stores ?? {})) {
    assert(environmentIds.has(store.environment), `data store ${id} names unknown environment ${store.environment}`);
    if (store.repo) assert(repoIds.has(store.repo), `data store ${id} names unknown repo ${store.repo}`);
  }

  for (const [id, capsule] of Object.entries(manifest.capsules ?? {})) {
    if (capsule.connector) {
      assert(storeIds.has(capsule.connector), `capsule ${id} names unknown connector ${capsule.connector}`);
      const connector = manifest.data_stores[capsule.connector];
      if (capsule.environment) assert(capsule.environment === connector.environment, `capsule ${id} environment must match connector ${capsule.connector}`);
    }
    assert(Number.isInteger(capsule.timeout_seconds) && capsule.timeout_seconds > 0, `capsule ${id} needs a positive timeout_seconds`);
    assert(Number.isInteger(capsule.max_output_bytes) && capsule.max_output_bytes > 0, `capsule ${id} needs a positive max_output_bytes`);
  }

  for (const [id, docs] of Object.entries(manifest.docs ?? {})) {
    const locations = [docs.path, docs.url, docs.repository_url].filter(Boolean);
    assert(locations.length === 1, `docs ${id} must have exactly one location`);
    assert(!Object.hasOwn(docs, "repo") || repoIds.has(docs.repo), `docs ${id} repo must be a declared repo id, never a URL`);
  }
}

JSON.parse(readFileSync(resolve(root, "schema/aimanager.schema.json"), "utf8"));
assertReference(parseJsonc(resolve(root, "doc/reference-manifest.jsonc")));
parseJsonc(resolve(root, "doc/reference-capsule.jsonc"));
console.log("Reference schema and JSONC examples are structurally consistent.");
