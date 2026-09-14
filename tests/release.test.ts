import { it } from "node:test";
import assert from "node:assert/strict";
import { verifyVersions, verifyManifest } from "../scripts/verify-release.mjs";

it("release versions and both lockfiles agree", () => {
  const current = verifyVersions();
  assert.equal(verifyVersions(`v${current}`), current);
  assert.throws(() => verifyVersions("v99.0.0"));
});

const manifest = () => ({
  version: "0.2.2", pub_date: "2026-09-14T00:00:00Z",
  platforms: { "windows-x86_64": {
    url: "https://github.com/newskool4d-sketch/spectra-ai-usage-widget/releases/download/v0.2.2/SPECTRA_0.2.2_x64-setup.exe",
    signature: "test-signature",
  } },
});

it("release manifest contains the intended installer and uploaded signature", () => {
  assert.doesNotThrow(() => verifyManifest(manifest(), "0.2.2", "test-signature\n"));
});

it("release manifest binds a GitHub asset API URL to the verified installer ID", () => {
  const value = manifest();
  value.platforms["windows-x86_64"].url = "https://api.github.com/repos/newskool4d-sketch/spectra-ai-usage-widget/releases/assets/123";
  assert.doesNotThrow(() => verifyManifest(value, "0.2.2", "test-signature", 123));
  assert.throws(() => verifyManifest(value, "0.2.2", "test-signature", 456));
  assert.throws(() => verifyManifest(value, "0.2.2", "test-signature"));
});

it("release manifest rejects a wrong version, URL, signature, date, or missing platform", () => {
  for (const mutate of [
    (m: any) => { m.version = "0.2.1"; },
    (m: any) => { m.platforms["windows-x86_64"].url = "https://example.com/update.exe"; },
    (m: any) => { m.platforms["windows-x86_64"].signature = "wrong"; },
    (m: any) => { m.platforms["windows-x86_64-nsis"] = { ...m.platforms["windows-x86_64"], signature: "wrong" }; },
    (m: any) => { m.pub_date = "invalid"; },
    (m: any) => { m.platforms = {}; },
  ]) {
    const value = manifest();
    mutate(value);
    assert.throws(() => verifyManifest(value, "0.2.2", "test-signature"));
  }
});
