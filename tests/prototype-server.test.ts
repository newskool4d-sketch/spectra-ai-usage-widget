import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { once } from "node:events";
import { test } from "node:test";
import { fileURLToPath } from "node:url";

test("the prototype command serves only its design assets", { timeout: 10_000 }, async context => {
  const entry = fileURLToPath(new URL("../server.mjs", import.meta.url));
  const child = spawn(process.execPath, [entry], {
    env: { ...process.env, SPECTRA_PORT: "0" },
    stdio: ["ignore", "pipe", "inherit"]
  });
  const exited = once(child, "exit");
  context.after(async () => {
    if (child.exitCode === null && child.signalCode === null) child.kill();
    await exited;
  });
  const startup = await Promise.race([
    once(child.stdout, "data").then(([chunk]) => String(chunk)),
    exited.then(([code]) => { throw new Error(`Prototype server exited with code ${code}`); })
  ]);
  const address = startup.match(/http:\/\/127\.0\.0\.1:\d+/)?.[0];
  assert.ok(address, "The server reports its assigned loopback port");

  const page = await fetch(`${address}/?variant=B`);
  assert.equal(page.status, 200);
  assert.match(await page.text(), /src="\/app\.js"/);

  for (const asset of ["/app.js", "/styles.css", "/styles/tokens.css", "/styles/prototype.css", "/styles/fonts/Pretendard-Regular.subset.woff2"]) {
    const response = await fetch(`${address}${asset}`);
    assert.equal(response.status, 200, asset);
    await response.arrayBuffer();
  }
  const otherFile = await fetch(`${address}/README.md`);
  assert.equal(otherFile.status, 404);
  assert.equal(await otherFile.text(), "Not found");
});
