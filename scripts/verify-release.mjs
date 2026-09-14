import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";
import { resolve, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("../", import.meta.url));
const repo = "newskool4d-sketch/spectra-ai-usage-widget";
const readJson = (path) => JSON.parse(readFileSync(path, "utf8"));

export function verifyVersions(tag) {
  const pkg = readJson(join(root, "package.json"));
  const lock = readJson(join(root, "package-lock.json"));
  const config = readJson(join(root, "src-tauri/tauri.conf.json"));
  const cargo = readFileSync(join(root, "src-tauri/Cargo.toml"), "utf8");
  const cargoLock = readFileSync(join(root, "src-tauri/Cargo.lock"), "utf8");
  const version = pkg.version;
  assert.match(version, /^\d+\.\d+\.\d+$/, "Release must use a stable SemVer");
  for (const actual of [lock.version, lock.packages[""].version, config.version,
    cargo.match(/^version = "([^"]+)"/m)?.[1],
    cargoLock.match(/name = "spectra-native"\r?\nversion = "([^"]+)"/)?.[1]]) {
    assert.equal(actual, version, "All package and lock versions must match");
  }
  if (tag) assert.equal(tag, `v${version}`, "Tag must match the package version");
  assert.equal(config.bundle.createUpdaterArtifacts, true);
  assert.ok(config.plugins.updater.pubkey);
  assert.deepEqual(config.plugins.updater.endpoints, [
    `https://github.com/${repo}/releases/latest/download/latest.json`,
  ]);
  assert.ok(readFileSync(join(root, `docs/releases/v${version}.md`), "utf8").trim());
  return version;
}

export function verifyManifest(manifest, version, signature, assetId) {
  assert.equal(manifest.version.replace(/^v/, ""), version);
  assert.ok(Number.isFinite(Date.parse(manifest.pub_date)), "Publication date is required");
  const target = manifest.platforms?.["windows-x86_64"];
  assert.ok(target, "Windows x64 updater target is required");
  const allowedUrls = [`https://github.com/${repo}/releases/download/v${version}/SPECTRA_${version}_x64-setup.exe`];
  // tauri-action v1 emits GitHub's asset API URL. Bind its ID to the named
  // installer in this exact release, not just to an arbitrary GitHub URL.
  if (Number.isSafeInteger(assetId) && assetId > 0) {
    allowedUrls.push(`https://api.github.com/repos/${repo}/releases/assets/${assetId}`);
  }
  assert.ok(allowedUrls.includes(target.url), "Updater URL must identify this release's installer");
  assert.ok(signature.trim(), "Signature must not be empty");
  assert.equal(target.signature.trim(), signature.trim(), "Manifest must embed the uploaded signature");
  const nsis = manifest.platforms["windows-x86_64-nsis"];
  if (nsis) assert.deepEqual(nsis, target, "NSIS-specific target must match the Windows updater target");
}

export function verifyArtifacts(directory, version) {
  const name = `SPECTRA_${version}_x64-setup.exe`;
  const release = JSON.parse(execFileSync("gh", ["api", `repos/${repo}/releases/tags/v${version}`], { encoding: "utf8" }));
  assert.equal(release.tag_name, `v${version}`);
  const installer = release.assets.find((asset) => asset.name === name);
  assert.ok(installer, "Installer must be uploaded to the intended release");
  const signature = readFileSync(join(directory, `${name}.sig`), "utf8");
  verifyManifest(readJson(join(directory, "latest.json")), version, signature, installer.id);
  const bytes = readFileSync(join(directory, name));
  assert.ok(bytes.length > 100_000 && bytes.subarray(0, 2).toString() === "MZ", "NSIS executable is required");
  const checksums = [name, `${name}.sig`, "latest.json"].map((file) => {
    const hash = createHash("sha256").update(readFileSync(join(directory, file))).digest("hex");
    const asset = release.assets.find((item) => item.name === file);
    assert.equal(asset?.digest, `sha256:${hash}`, `Uploaded digest must match ${file}`);
    return `${hash}  ${file}`;
  });
  writeFileSync(join(directory, "SHA256SUMS.txt"), `${checksums.join("\n")}\n`);
  return checksums;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const version = verifyVersions(process.env.GITHUB_REF_TYPE === "tag" ? process.env.GITHUB_REF_NAME : undefined);
  if (process.argv[2]) console.log(verifyArtifacts(resolve(process.argv[2]), version).join("\n"));
  console.log(`Release preflight PASS: v${version}`);
}
