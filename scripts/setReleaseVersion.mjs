import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const requestedVersion = process.argv[2]?.trim() || "";
const version = requestedVersion.startsWith("v")
  ? requestedVersion.slice(1)
  : requestedVersion;

if (!/^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-[0-9A-Za-z.-]+)?$/.test(version)) {
  console.error("Usage: npm run release:set-version -- 0.1.5");
  console.error("The version must be a semantic version such as 0.1.5 or 1.0.0-beta.1.");
  process.exit(1);
}

const paths = {
  packageJson: path.join(root, "package.json"),
  packageLock: path.join(root, "package-lock.json"),
  tauriConfig: path.join(root, "src-tauri/tauri.conf.json"),
  cargoToml: path.join(root, "src-tauri/Cargo.toml"),
  cargoLock: path.join(root, "src-tauri/Cargo.lock"),
};

const packageJson = JSON.parse(fs.readFileSync(paths.packageJson, "utf8"));
const packageLock = JSON.parse(fs.readFileSync(paths.packageLock, "utf8"));
const cargoToml = fs.readFileSync(paths.cargoToml, "utf8");
const cargoLock = fs.readFileSync(paths.cargoLock, "utf8");

if (!packageLock.packages?.[""]) {
  throw new Error("Could not find the root package entry in package-lock.json");
}

const replacePackageVersion = (contents, fileName) => {
  const pattern = /^(\[package\][\s\S]*?^version\s*=\s*")[^"]+(")/m;
  if (!pattern.test(contents)) {
    throw new Error(`Could not find the package version in ${fileName}`);
  }
  return contents.replace(pattern, `$1${version}$2`);
};

const cargoLockPattern =
  /^(\[\[package\]\]\nname = "smalltalk"\nversion = ")[^"]+(")/m;
if (!cargoLockPattern.test(cargoLock)) {
  throw new Error("Could not find the Smalltalk package entry in src-tauri/Cargo.lock");
}

packageJson.version = version;
packageLock.version = version;
packageLock.packages[""].version = version;

const tauriVersionPattern = /^(\s*"version":\s*")[^"]+(",)$/m;
if (!tauriVersionPattern.test(fs.readFileSync(paths.tauriConfig, "utf8"))) {
  throw new Error("Could not find the version in src-tauri/tauri.conf.json");
}

fs.writeFileSync(paths.packageJson, `${JSON.stringify(packageJson, null, 2)}\n`);
fs.writeFileSync(paths.packageLock, `${JSON.stringify(packageLock, null, 2)}\n`);
fs.writeFileSync(
  paths.tauriConfig,
  fs
    .readFileSync(paths.tauriConfig, "utf8")
    .replace(tauriVersionPattern, `$1${version}$2`),
);
fs.writeFileSync(
  paths.cargoToml,
  replacePackageVersion(cargoToml, "src-tauri/Cargo.toml"),
);
fs.writeFileSync(
  paths.cargoLock,
  cargoLock.replace(cargoLockPattern, `$1${version}$2`),
);

console.log(`Updated all Smalltalk release versions to ${version}.`);
const check = spawnSync(
  process.execPath,
  [path.join(root, "scripts/checkReleaseVersion.mjs"), `v${version}`],
  { stdio: "inherit" },
);
if (check.status !== 0) {
  process.exit(check.status ?? 1);
}
console.log(`Create the Git tag v${version} only after committing these files.`);
