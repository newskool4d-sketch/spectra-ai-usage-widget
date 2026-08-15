import { createHash } from "node:crypto";
import { readFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));
const projectRoot = path.resolve(scriptDirectory, "..");
const manifestPath = path.join(projectRoot, "docs", "design-baseline", "baseline.json");
const manifest = JSON.parse(await readFile(manifestPath, "utf8"));
const failures = [];

function requireCondition(condition, message) {
  if (!condition) failures.push(message);
}

requireCondition(manifest.schemaVersion === 1, "지원하지 않는 기준선 스키마입니다.");
requireCondition(manifest.status === "approved", "기준선 상태가 approved가 아닙니다.");
requireCondition(manifest.sourceOfTruth === "code-and-screenshots", "코드·스크린샷 기준선이 아닙니다.");
requireCondition(manifest.figmaRequired === false, "Figma 비의존성 선언이 깨졌습니다.");
requireCondition(manifest.decision?.combination === "A+C", "A+C 결정이 바뀌었습니다.");
requireCondition(manifest.decision?.desktop === "A", "데스크톱 기준이 A가 아닙니다.");
requireCondition(manifest.decision?.mobile === "C", "모바일 기준이 C가 아닙니다.");

for (const asset of manifest.assets ?? []) {
  try {
    const bytes = await readFile(path.join(projectRoot, asset.path));
    const actualHash = createHash("sha256").update(bytes).digest("hex").toUpperCase();
    requireCondition(actualHash === asset.sha256, `${asset.path} 해시가 기준선과 다릅니다.`);
  } catch (error) {
    failures.push(`${asset.path}을 읽을 수 없습니다: ${error.message}`);
  }
}

const packageJson = JSON.parse(await readFile(path.join(projectRoot, "package.json"), "utf8"));
const dependencySections = ["dependencies", "devDependencies", "optionalDependencies", "peerDependencies"];
const dependencyNames = dependencySections.flatMap(section => Object.keys(packageJson[section] ?? {}));
requireCondition(!dependencyNames.some(name => /figma/i.test(name)), "Figma 관련 패키지가 등록되어 있습니다.");

const sourceFiles = ["app.js", "index.html", "styles.css", "server.mjs", "src/main.tsx", "src/App.tsx", "src/data/providers.ts"];
const forbiddenRuntimeReference = /plugin:\/\/figma|figma\.com|@figma\//i;

for (const sourceFile of sourceFiles) {
  const source = await readFile(path.join(projectRoot, sourceFile), "utf8");
  requireCondition(!forbiddenRuntimeReference.test(source), `${sourceFile}에 Figma 런타임 참조가 있습니다.`);
}

if (failures.length > 0) {
  console.error("DESIGN BASELINE: FAIL");
  failures.forEach(failure => console.error(`- ${failure}`));
  process.exitCode = 1;
} else {
  console.log("DESIGN BASELINE: PASS");
  console.log(`- decision: ${manifest.decision.combination}`);
  console.log(`- assets: ${manifest.assets.length}`);
  console.log("- Figma runtime dependencies: 0");
}
