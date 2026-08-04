#!/usr/bin/env node

import { readFileSync } from "node:fs";
import { posix, resolve } from "node:path";
import Ajv2020 from "ajv/dist/2020.js";
import addFormats from "ajv-formats";

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

function assertRelativePath(value, label) {
  assert(typeof value === "string" && value.length > 0, `${label} must be a non-empty path`);
  assert(!value.includes("\\"), `${label} must use POSIX separators`);
  assert(value === "." || (posix.normalize(value) === value && !value.startsWith("/") && !value.split("/").includes("..")), `${label} must be a normalized relative POSIX path`);
}

function assertAcyclic(records, kind) {
  const visiting = new Set();
  const visited = new Set();

  function visit(id) {
    assert(!visiting.has(id), `${kind} dependency cycle includes ${id}`);
    if (visited.has(id)) return;
    visiting.add(id);
    for (const dependency of records[id].depends_on ?? []) visit(dependency);
    visiting.delete(id);
    visited.add(id);
  }

  for (const id of Object.keys(records)) visit(id);
}

function templatePlaceholders(capsule, id) {
  assert(typeof capsule.program === "string" && capsule.program.trim().length > 0, `capsule ${id} must have a non-empty program`);
  assert(!/[{}]/.test(capsule.program), `capsule ${id}.program must be literal`);
  assert(Array.isArray(capsule.args) && capsule.args.every((arg) => typeof arg === "string"), `capsule ${id}.args must be a string array`);
  const placeholders = [];
  for (const element of capsule.args) {
    const occurrences = [...element.matchAll(/\{([A-Za-z][A-Za-z0-9_-]*)\}/g)];
    const remainder = element.replace(/\{([A-Za-z][A-Za-z0-9_-]*)\}/g, "");
    assert(!/[{}]/.test(remainder), `capsule ${id} has malformed placeholder braces`);
    assert(occurrences.length <= 1, `capsule ${id} has multiple placeholders in one argv element`);
    placeholders.push(...occurrences.map((match) => match[1]));
  }
  assert(new Set(placeholders).size === placeholders.length, `capsule ${id} repeats a placeholder`);
  return new Set(placeholders);
}

function assertReference(manifest) {
  assert(manifest.version === 2, "reference manifest must be version 2");
  assert(manifest.$schema === "https://raw.githubusercontent.com/MeryllEssig/senpai/main/schema/senpai.schema.json", "reference manifest must declare the raw GitHub schema URI");

  const repoIds = new Set(Object.keys(manifest.repos ?? {}));
  const environmentIds = new Set(Object.keys(manifest.environments ?? {}));
  const integrations = manifest.integrations ?? {};
  const ticketOperations = new Set(["ticket.view", "ticket.create", "ticket.edit", "ticket.comment", "ticket.change_status", "ticket.link", "ticket.log_time"]);
  const forgeOperations = new Set(["pull_merge_request.view", "pull_merge_request.create", "pull_merge_request.edit", "pull_merge_request.comment", "pull_merge_request.request_review", "pull_merge_request.merge", "pipeline.view", "pipeline.job.view_log", "pipeline.trigger"]);
  const repos = manifest.repos ?? {};
  const repoPaths = new Set();

  for (const [id, repo] of Object.entries(manifest.repos ?? {})) {
    assertRelativePath(repo.path, `repo ${id} path`);
    assert(!repoPaths.has(repo.path), `repo ${id} duplicates path ${repo.path}`);
    repoPaths.add(repo.path);
    for (const dependency of repo.depends_on ?? []) assert(repoIds.has(dependency), `repo ${id} depends on unknown repo ${dependency}`);
    for (const integrationId of Object.keys(repo.integrations ?? {})) assert(integrations[integrationId]?.kind === "forge", `repo ${id} names unknown forge integration ${integrationId}`);
  }
  assertAcyclic(repos, "repo");

  for (const [id, integration] of Object.entries(integrations)) {
    const operations = integration.kind === "ticketing" ? ticketOperations : forgeOperations;
    for (const operation of integration.provides ?? []) assert(operations.has(operation), `integration ${id} provides an operation incompatible with its kind`);
    for (const operation of integration.handles ?? []) assert(integration.provides?.includes(operation), `integration ${id} handles an unavailable operation`);
  }

  for (const [id, environment] of Object.entries(manifest.environments ?? {})) {
    if (environment.repo) assert(repoIds.has(environment.repo), `environment ${id} names unknown repo ${environment.repo}`);
  }

  for (const [id, capsule] of Object.entries(manifest.capsules ?? {})) {
    assert(typeof capsule.label === "string" && capsule.label.length > 0, `capsule ${id} needs a label`);
    if (capsule.repo) assert(repoIds.has(capsule.repo), `capsule ${id} names unknown repo ${capsule.repo}`);
    if (capsule.environment) assert(environmentIds.has(capsule.environment), `capsule ${id} names unknown environment ${capsule.environment}`);
    if (capsule.cwd) assertRelativePath(capsule.cwd, `capsule ${id} cwd`);
    if (capsule.environment) {
      const environment = manifest.environments[capsule.environment];
      const capsuleRepo = capsule.repo;
      const environmentRepo = environment.repo;
      if (capsuleRepo && environmentRepo) assert(capsuleRepo === environmentRepo, `capsule ${id} scope conflicts with environment ${capsule.environment}`);
    }
    if (capsule.timeout_seconds !== undefined) assert(Number.isInteger(capsule.timeout_seconds) && capsule.timeout_seconds > 0, `capsule ${id} needs a positive timeout_seconds`);
    if (capsule.max_output_bytes !== undefined) assert(Number.isInteger(capsule.max_output_bytes) && capsule.max_output_bytes > 0, `capsule ${id} needs a positive max_output_bytes`);
  }

  for (const [id, docs] of Object.entries(manifest.docs ?? {})) {
    const locations = [docs.path, docs.url, docs.repository_url].filter(Boolean);
    assert(locations.length === 1, `docs ${id} must have exactly one location`);
    assert(!Object.hasOwn(docs, "repo") || repoIds.has(docs.repo), `docs ${id} repo must be a declared repo id, never a URL`);
    if (docs.path) assertRelativePath(docs.path, `docs ${id} path`);
  }
}

function assertCapsuleValues(manifest, values) {
  assert(values && typeof values === "object" && !Array.isArray(values), "local capsule values must be an object");
  const expectedCapsules = new Set();
  for (const [id, capsule] of Object.entries(manifest.capsules ?? {})) {
    const placeholders = templatePlaceholders(capsule, id);
    const supplied = new Set(capsule.supplied ?? []);
    for (const name of supplied) assert(placeholders.has(name), `capsule ${id} supplies unknown placeholder ${name}`);
    const localNames = [...placeholders].filter((name) => !supplied.has(name));
    if (localNames.length === 0) {
      assert(!Object.hasOwn(values, id), `capsule ${id} requires no local values entry`);
      continue;
    }
    expectedCapsules.add(id);
    assert(values[id] && typeof values[id] === "object" && !Array.isArray(values[id]), `capsule ${id} needs a local values object`);
    const actualNames = Object.keys(values[id]);
    for (const name of localNames) assert(Object.hasOwn(values[id], name), `capsule ${id} is missing local value ${name}`);
    for (const name of actualNames) assert(localNames.includes(name), `capsule ${id} has unexpected local value ${name}`);
    for (const [name, value] of Object.entries(values[id])) {
      assert(typeof value === "string" && value.length > 0, `capsule ${id} local value ${name} must be a non-empty string`);
      if (value.startsWith("$")) assert(/^\$[A-Za-z_][A-Za-z0-9_]*$/.test(value), `capsule ${id} local value ${name} has an invalid $ENV reference`);
    }
  }
  for (const id of Object.keys(values)) assert(expectedCapsules.has(id), `local values name unknown capsule or capsule requiring no local values: ${id}`);
}

const schema = JSON.parse(readFileSync(resolve(root, "schema/senpai.schema.json"), "utf8"));
const manifest = parseJsonc(resolve(root, "doc/reference-manifest.jsonc"));
const capsuleValues = parseJsonc(resolve(root, "doc/reference-capsule.jsonc"));
const ajv = new Ajv2020({
  allErrors: true,
  strict: true,
  strictRequired: false,
  allowMatchingProperties: true,
});
addFormats(ajv);
const validateManifest = ajv.compile(schema);
assert(validateManifest(manifest), `reference manifest does not match the JSON Schema:\n${ajv.errorsText(validateManifest.errors, { separator: "\n" })}`);
assertReference(manifest);
assertCapsuleValues(manifest, capsuleValues);
console.log("Reference JSONC examples match the JSON Schema and cross-reference rules.");
