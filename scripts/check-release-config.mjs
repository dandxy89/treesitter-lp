// Checks the release-please setup without any dependency:
//   node scripts/check-release-config.mjs
//
// - the config has the single root package and release type `rust`;
// - the manifest version equals the version in every extra-file, and every
//   extra-file jsonpath resolves;
// - with TAG_NAME set, the tag is `v<version>`.
//
// CI also validates the config against release-please's JSON schema.
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const read = (path) => readFileSync(join(root, path), "utf8");
const problems = [];
const check = (condition, message) => {
  if (!condition) problems.push(message);
};

const config = JSON.parse(read("release-please-config.json"));
const manifest = JSON.parse(read(".release-please-manifest.json"));

check(
  Object.keys(config.packages ?? {}).join() === ".",
  "config must declare exactly one package, `.`",
);
const pkg = config.packages?.["."] ?? {};
check(pkg["release-type"] === "rust", "release-type must be `rust`");
check(
  pkg["include-component-in-tag"] === false,
  "tags must be plain `v<version>` (include-component-in-tag false)",
);
check(
  typeof manifest["."] === "string",
  "manifest must hold a version for `.`",
);
const version = manifest["."];

/** Read the version a TOML file declares at `path`. */
function tomlVersion(text, table, key = "version") {
  let inTable = false;
  for (const line of text.split("\n")) {
    const header = line.match(/^\[(.+)\]\s*$/);
    if (header) inTable = header[1] === table;
    else if (inTable) {
      const found = line.match(new RegExp(`^${key}\\s*=\\s*"([^"]+)"`));
      if (found) return found[1];
    }
  }
  return undefined;
}

/** The version of `name` in Cargo.lock. */
function lockVersion(text, name) {
  const block = text
    .split("[[package]]")
    .find((entry) => entry.includes(`name = "${name}"`));
  return block?.match(/version = "([^"]+)"/)?.[1];
}

// jsonpath -> value, for the handful of shapes the config uses.
const resolvers = {
  "Cargo.toml $.package.version": () =>
    tomlVersion(read("Cargo.toml"), "package"),
  "Cargo.lock $.package[?(@.name=='tree-sitter-lp')].version": () =>
    lockVersion(read("Cargo.lock"), "tree-sitter-lp"),
  "package.json $.version": () => JSON.parse(read("package.json")).version,
  "package-lock.json $.version": () =>
    JSON.parse(read("package-lock.json")).version,
  "package-lock.json $.packages[''].version": () =>
    JSON.parse(read("package-lock.json")).packages?.[""]?.version,
  "tree-sitter.json $.metadata.version": () =>
    JSON.parse(read("tree-sitter.json")).metadata?.version,
  "pyproject.toml $.project.version": () =>
    tomlVersion(read("pyproject.toml"), "project"),
};

const declared = new Set();
for (const file of pkg["extra-files"] ?? []) {
  const key = `${file.path} ${file.jsonpath}`;
  declared.add(key);
  const resolve = resolvers[key];
  check(resolve !== undefined, `unexpected extra-file ${key}`);
  if (resolve)
    check(
      resolve() === version,
      `${key} is ${resolve()}, manifest says ${version}`,
    );
}
// Cargo.toml and Cargo.lock are bumped by the "rust" release type, not
// extra-files (the TOML updater cannot match Cargo.lock's [[package]] filter).
for (const key of Object.keys(resolvers)) {
  const native = key.startsWith("Cargo.");
  if (native) {
    check(!declared.has(key), `${key} must not be an extra-file`);
    check(
      resolvers[key]() === version,
      `${key} is ${resolvers[key]()}, manifest says ${version}`,
    );
  } else check(declared.has(key), `missing extra-file ${key}`);
}

const tag = process.env.TAG_NAME;
if (tag !== undefined)
  check(tag === `v${version}`, `tag ${tag} does not match version v${version}`);

if (problems.length > 0) {
  console.error(problems.map((p) => `release config: ${p}`).join("\n"));
  process.exit(1);
}
console.log(
  `release config ok (version ${version}, ${declared.size} extra files)`,
);
