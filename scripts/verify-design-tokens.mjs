import { readFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));
const projectRoot = path.resolve(scriptDirectory, "..");
const tokenPath = path.join(projectRoot, "styles", "tokens.css");
const stylePath = path.join(projectRoot, "styles.css");
const indexPath = path.join(projectRoot, "index.html");
const prototypePath = path.join(projectRoot, "styles", "prototype.css");
const manifestPath = path.join(projectRoot, "docs", "design-baseline", "baseline.json");
const [tokens, styles, prototype, indexHtml, manifest] = await Promise.all([
  readFile(tokenPath, "utf8"),
  readFile(stylePath, "utf8"),
  readFile(prototypePath, "utf8"),
  readFile(indexPath, "utf8"),
  readFile(manifestPath, "utf8").then(JSON.parse)
]);

const failures = [];
const requiredTokens = [
  "--font-ui",
  "--space-1",
  "--space-6",
  "--radius-lg",
  "--shadow-card",
  "--glass-nav-blur",
  "--color-cyan",
  "--color-coral",
  "--color-surface-subtle"
];

for (const token of requiredTokens) {
  if (!new RegExp(`${token.replaceAll("-", "\\-")}\\s*:`).test(tokens)) {
    failures.push(`토큰이 없습니다: ${token}`);
  }
}

const tokenLink = indexHtml.indexOf("styles/tokens.css");
const styleLink = indexHtml.indexOf("styles.css");
if (tokenLink === -1 || styleLink === -1 || tokenLink > styleLink) {
  failures.push("tokens.css가 styles.css보다 먼저 로드되지 않습니다.");
}
if (/:root\s*\{/.test(styles)) failures.push("styles.css에 토큰 선언 블록이 남아 있습니다.");
if (/:root\s*\{/.test(prototype)) failures.push("prototype.css에 토큰 선언 블록이 있습니다.");
if (!/orbit-app/.test(prototype)) failures.push("B 시안 규칙이 prototype.css에 없습니다.");
if (/orbit-|prototype-switcher|widget-lab/.test(styles)) failures.push("제품 styles.css에 프로토타입 전용 규칙이 남아 있습니다.");
if (!/font-family:\s*var\(--font-ui\)/.test(styles)) failures.push("본문이 --font-ui 토큰을 사용하지 않습니다.");
if (!/var\(--glass-nav-blur\)/.test(styles)) failures.push("내비게이션이 --glass-nav-blur 토큰을 사용하지 않습니다.");
if (/\.glass-card\s*\{[^}]*backdrop-filter/.test(styles)) failures.push("카드에 backdrop-filter가 남아 있습니다.");
if (!/var\(--space-6\)/.test(styles)) failures.push("화면 간격이 --space-6 토큰을 사용하지 않습니다.");
if (!/var\(--shadow-card\)/.test(styles)) failures.push("카드가 --shadow-card 토큰을 사용하지 않습니다.");
if (manifest.tokenSource !== "styles/tokens.css" || manifest.tokenRevision !== 1) {
  failures.push("기준선 매니페스트의 토큰 출처 또는 버전이 맞지 않습니다.");
}

const hardcodedWhite = /#fff(?:f|fff|ffff)?\b|rgba?\(\s*255\s*,\s*255\s*,\s*255/i;
const whiteLines = styles.split("\n").flatMap((line, index) => hardcodedWhite.test(line) ? [index + 1] : []);
if (whiteLines.length > 0) failures.push(`styles.css에 하드코딩 화이트가 남아 있습니다 (lines: ${whiteLines.join(", ")})`);

const backdropCount = (styles.match(/backdrop-filter/g) ?? []).length;
if (backdropCount > 4) failures.push(`styles.css의 backdrop-filter가 ${backdropCount}건으로 4건을 초과합니다.`);

const tokenCount = (tokens.match(/--[a-z0-9-]+\s*:/g) ?? []).length;
if (tokenCount < 40) failures.push(`토큰 수가 너무 적습니다: ${tokenCount}`);

if (failures.length > 0) {
  console.error("DESIGN TOKENS: FAIL");
  failures.forEach(failure => console.error(`- ${failure}`));
  process.exitCode = 1;
} else {
  console.log("DESIGN TOKENS: PASS");
  console.log(`- token source: ${manifest.tokenSource}`);
  console.log(`- declared tokens: ${tokenCount}`);
  console.log("- A+C style consumers: PASS");
}
