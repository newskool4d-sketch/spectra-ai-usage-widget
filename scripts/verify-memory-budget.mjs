import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(fileURLToPath(new URL("..", import.meta.url)));
const sourceFiles = [
  "src/App.tsx",
  "src/main.tsx",
  "src/components/Icon.tsx",
  "src/components/Sparkline.tsx",
  "src/data/providers.ts"
];
const source = sourceFiles.map(file => readFileSync(join(root, file), "utf8")).join("\n");
const failures = [];

if (!source.includes("matchMedia")) failures.push("responsive mount gate is missing");
if (!source.includes("isMobile ? <VariantCMobile") || !source.includes(": <VariantADesktop")) failures.push("A/C conditional mount is missing");
if (!source.includes("Object.freeze")) failures.push("provider data is not frozen at module scope");
if (!source.includes("clearTimeout")) failures.push("refresh timer cleanup is missing");
for (const pattern of [/setInterval\s*\(/, /requestAnimationFrame\s*\(/, /localStorage/, /sessionStorage/]) {
  if (pattern.test(source)) failures.push(`forbidden runtime pattern: ${pattern}`);
}

const distAssets = join(root, "dist", "assets");
if (!existsSync(distAssets)) failures.push("dist/assets is missing; run npm run build first");

let jsBytes = 0;
let jsFiles = 0;
if (existsSync(distAssets)) {
  for (const file of readdirSync(distAssets)) {
    const path = join(distAssets, file);
    if (!statSync(path).isFile()) continue;
    if (file.endsWith(".js")) {
      jsBytes += statSync(path).size;
      jsFiles += 1;
      const bundle = readFileSync(path, "utf8");
      if (/setInterval\s*\(|requestAnimationFrame\s*\(|localStorage|sessionStorage/.test(bundle)) failures.push(`forbidden runtime pattern in ${file}`);
    }
  }
}

const maxJsBytes = 500_000;
if (jsBytes > maxJsBytes) failures.push(`JavaScript bundle ${jsBytes} bytes exceeds ${maxJsBytes} byte structural budget`);
if (jsFiles > 3) failures.push(`JavaScript asset count ${jsFiles} exceeds 3`);

if (failures.length > 0) {
  console.error("MEMORY BUDGET: FAIL");
  for (const failure of failures) console.error(`- ${failure}`);
  process.exitCode = 1;
} else {
  console.log(`MEMORY BUDGET: PASS (js=${jsBytes} bytes, assets=${jsFiles}, one-layout mount, no polling/persistence)`);
}
