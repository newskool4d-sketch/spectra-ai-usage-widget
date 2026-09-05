import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(fileURLToPath(new URL("..", import.meta.url)));
const sourceFiles = [
  "src/App.tsx",
  "src/main.tsx",
  "src/components/Icon.tsx",
  "src/components/Sparkline.tsx",
  "src/data/providers.ts",
  "src/data/next-action.ts"
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
let cssBytes = 0;
let fontBytes = 0;
let fontFiles = 0;
if (existsSync(distAssets)) {
  for (const file of readdirSync(distAssets)) {
    const path = join(distAssets, file);
    if (!statSync(path).isFile()) continue;
    const size = statSync(path).size;
    if (file.endsWith(".js")) {
      jsBytes += size;
      jsFiles += 1;
      const bundle = readFileSync(path, "utf8");
      if (/setInterval\s*\(|requestAnimationFrame\s*\(|localStorage|sessionStorage/.test(bundle)) failures.push(`forbidden runtime pattern in ${file}`);
      if (/43,58,49,70,64,83/.test(bundle)) failures.push(`example chart data shipped in ${file}`);
    } else if (file.endsWith(".css")) {
      cssBytes += size;
      const css = readFileSync(path, "utf8");
      for (const forbidden of ["orbit-", "prototype-switcher", "widget-lab", "@keyframes drift", "bar-chart", "barRise"]) {
        if (css.includes(forbidden)) failures.push(`prototype-only CSS shipped in ${file}: ${forbidden}`);
      }
    } else if (file.endsWith(".woff2")) {
      fontBytes += size;
      fontFiles += 1;
    }
  }
}

const maxJsBytes = 500_000;
const maxCssBytes = 34_000;
const maxFontBytes = 820_000;
const maxFontFiles = 3;
if (jsBytes > maxJsBytes) failures.push(`JavaScript bundle ${jsBytes} bytes exceeds ${maxJsBytes} byte structural budget`);
if (jsFiles > 3) failures.push(`JavaScript asset count ${jsFiles} exceeds 3`);
if (cssBytes > maxCssBytes) failures.push(`CSS bundle ${cssBytes} bytes exceeds ${maxCssBytes}`);
if (fontBytes > maxFontBytes) failures.push(`font payload ${fontBytes} bytes exceeds ${maxFontBytes}`);
if (fontFiles > maxFontFiles) failures.push(`font file count ${fontFiles} exceeds ${maxFontFiles}`);

if (failures.length > 0) {
  console.error("MEMORY BUDGET: FAIL");
  for (const failure of failures) console.error(`- ${failure}`);
  process.exitCode = 1;
} else {
  console.log(`MEMORY BUDGET: PASS (js=${jsBytes} bytes, css=${cssBytes} bytes, fonts=${fontBytes} bytes/${fontFiles} files, assets=${jsFiles}, one-layout mount, no polling/persistence)`);
}
