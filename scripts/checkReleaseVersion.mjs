import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const packageJson = JSON.parse(fs.readFileSync(path.join(root, "package.json"), "utf8"));
const tauriConfig = JSON.parse(fs.readFileSync(path.join(root, "src-tauri/tauri.conf.json"), "utf8"));
const cargoToml = fs.readFileSync(path.join(root, "src-tauri/Cargo.toml"), "utf8");
const cargoVersion = cargoToml.match(/^\[package\][\s\S]*?^version\s*=\s*"([^"]+)"/m)?.[1];

if (!cargoVersion) {
  throw new Error("Could not read the package version from src-tauri/Cargo.toml");
}

const versions = new Map([
  ["package.json", packageJson.version],
  ["src-tauri/tauri.conf.json", tauriConfig.version],
  ["src-tauri/Cargo.toml", cargoVersion],
]);
const uniqueVersions = new Set(versions.values());

if (uniqueVersions.size !== 1) {
  const detail = [...versions].map(([file, version]) => `${file}: ${version}`).join("\n");
  throw new Error(`Smalltalk release versions do not match:\n${detail}`);
}

const version = [...uniqueVersions][0];
const requestedTag = process.argv[2] || process.env.GITHUB_REF_NAME || "";
if (requestedTag) {
  const tagVersion = requestedTag.startsWith("v") ? requestedTag.slice(1) : requestedTag;
  if (tagVersion !== version) {
    throw new Error(`Release tag ${requestedTag} does not match Smalltalk version ${version}`);
  }
}

console.log(`Smalltalk release version ${version} is synchronized.`);
