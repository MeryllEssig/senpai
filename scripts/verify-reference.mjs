#!/usr/bin/env node

import { readFileSync } from "node:fs";
import { posix, resolve } from "node:path";
import Ajv2020 from "ajv/dist/2020.js";
import addFormats from "ajv-formats";
import { parse as parseShellWords } from "shell-quote";

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

function templatePlaceholders(command, id) {
  assert(typeof command === "string" && command.trim().length > 0, `capsule ${id} must have a non-empty command`);
  const placeholders = [];
  const remainder = command.replace(/\{([A-Za-z][A-Za-z0-9_-]*)\}/g, (_, name) => {
    placeholders.push(name);
    return "";
  });
  assert(!/[{}]/.test(remainder), `capsule ${id} has malformed placeholder braces`);
  assert(!/`|\$(?:\(|\{|[A-Za-z_])/.test(remainder), `capsule ${id} must not contain shell expansion syntax`);
  let argv;
  try {
    argv = parseShellWords(command, () => "");
  } catch (error) {
    throw new Error(`capsule ${id} is not a valid shell-words template: ${error.message}`);
  }
  assert(argv.length > 0 && argv.every((element) => typeof element === "string"), `capsule ${id} must contain argv words only, without shell operators`);
  assert(argv[0].length > 0 && !/[{}]/.test(argv[0]), `capsule ${id} must have a literal executable`);
  for (const element of argv) {
    const occurrences = [...element.matchAll(/\{[A-Za-z][A-Za-z0-9_-]*\}/g)];
    assert(occurrences.length <= 1, `capsule ${id} has multiple placeholders in one argv element`);
  }
  assert(new Set(placeholders).size === placeholders.length, `capsule ${id} repeats a placeholder`);
  return new Set(placeholders);
}

function assertReference(manifest) {
  assert(manifest.version === 1, "reference manifest must be version 1");
  assert(manifest.$schema === "https://aimanager.dev/schema/v1/aimanager.schema.json", "reference manifest must declare the v1 schema URI");

  const repoIds = new Set(Object.keys(manifest.repos ?? {}));
  const componentIds = new Set(Object.keys(manifest.components ?? {}));
  const environmentIds = new Set(Object.keys(manifest.environments ?? {}));
  const hostingIds = new Set(Object.keys(manifest.code_hosting?.instances ?? {}));
  const repos = manifest.repos ?? {};
  const components = manifest.components ?? {};
  const repoPaths = new Set();
  const componentPaths = new Set();

  function effectiveRepo(scope) {
    return scope.component ? components[scope.component]?.repo : scope.repo;
  }

  for (const [id, repo] of Object.entries(manifest.repos ?? {})) {
    assertRelativePath(repo.path, `repo ${id} path`);
    assert(!repoPaths.has(repo.path), `repo ${id} duplicates path ${repo.path}`);
    repoPaths.add(repo.path);
    for (const dependency of repo.depends_on ?? []) assert(repoIds.has(dependency), `repo ${id} depends on unknown repo ${dependency}`);
    for (const hostingId of Object.keys(repo.hosting ?? {})) assert(hostingIds.has(hostingId), `repo ${id} names unknown hosting instance ${hostingId}`);
  }
  assertAcyclic(repos, "repo");

  for (const [id, component] of Object.entries(manifest.components ?? {})) {
    assert(repoIds.has(component.repo), `component ${id} names unknown repo ${component.repo}`);
    assertRelativePath(component.path, `component ${id} path`);
    const fullPath = posix.join(repos[component.repo].path, component.path);
    assert(!componentPaths.has(fullPath), `component ${id} duplicates path ${fullPath}`);
    componentPaths.add(fullPath);
    for (const dependency of component.depends_on ?? []) assert(componentIds.has(dependency), `component ${id} depends on unknown component ${dependency}`);
  }
  assertAcyclic(components, "component");

  for (const [id, environment] of Object.entries(manifest.environments ?? {})) {
    if (environment.repo) assert(repoIds.has(environment.repo), `environment ${id} names unknown repo ${environment.repo}`);
    if (environment.component) assert(componentIds.has(environment.component), `environment ${id} names unknown component ${environment.component}`);
  }

  for (const [id, capsule] of Object.entries(manifest.capsules ?? {})) {
    assert(typeof capsule.label === "string" && capsule.label.length > 0, `capsule ${id} needs a label`);
    if (capsule.repo) assert(repoIds.has(capsule.repo), `capsule ${id} names unknown repo ${capsule.repo}`);
    if (capsule.component) assert(componentIds.has(capsule.component), `capsule ${id} names unknown component ${capsule.component}`);
    if (capsule.environment) assert(environmentIds.has(capsule.environment), `capsule ${id} names unknown environment ${capsule.environment}`);
    if (capsule.cwd) assertRelativePath(capsule.cwd, `capsule ${id} cwd`);
    if (capsule.environment) {
      const environment = manifest.environments[capsule.environment];
      const capsuleRepo = effectiveRepo(capsule);
      const environmentRepo = effectiveRepo(environment);
      if (capsuleRepo && environmentRepo) assert(capsuleRepo === environmentRepo, `capsule ${id} scope conflicts with environment ${capsule.environment}`);
      if (capsule.component && environment.component) assert(capsule.component === environment.component, `capsule ${id} component conflicts with environment ${capsule.environment}`);
    }
    if (capsule.timeout_seconds !== undefined) assert(Number.isInteger(capsule.timeout_seconds) && capsule.timeout_seconds > 0, `capsule ${id} needs a positive timeout_seconds`);
    if (capsule.max_output_bytes !== undefined) assert(Number.isInteger(capsule.max_output_bytes) && capsule.max_output_bytes > 0, `capsule ${id} needs a positive max_output_bytes`);
  }

  for (const [id, docs] of Object.entries(manifest.docs ?? {})) {
    const locations = [docs.path, docs.url, docs.repository_url].filter(Boolean);
    assert(locations.length === 1, `docs ${id} must have exactly one location`);
    assert(!Object.hasOwn(docs, "repo") || repoIds.has(docs.repo), `docs ${id} repo must be a declared repo id, never a URL`);
    assert(!Object.hasOwn(docs, "component") || componentIds.has(docs.component), `docs ${id} component must be a declared component id`);
    if (docs.path) assertRelativePath(docs.path, `docs ${id} path`);
  }
}

function assertCapsuleValues(manifest, values) {
  assert(values && typeof values === "object" && !Array.isArray(values), "local capsule values must be an object");
  const expectedCapsules = new Set();
  for (const [id, capsule] of Object.entries(manifest.capsules ?? {})) {
    const placeholders = templatePlaceholders(capsule.command, id);
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

const schema = JSON.parse(readFileSync(resolve(root, "schema/aimanager.schema.json"), "utf8"));
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
