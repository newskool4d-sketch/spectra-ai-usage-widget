import { createServer } from "node:http";
import { readFile, realpath } from "node:fs/promises";
import { extname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = await realpath(fileURLToPath(new URL(".", import.meta.url)));
const port = Number(process.env.SPECTRA_PORT || 4173);
const publicFiles = new Map([
  ["/", "prototype.html"],
  ["/index.html", "prototype.html"],
  ["/app.js", "app.js"],
  ["/styles.css", "styles.css"],
  ["/styles/tokens.css", "styles/tokens.css"],
  ["/styles/prototype.css", "styles/prototype.css"],
  ["/styles/fonts/Pretendard-Regular.subset.woff2", "styles/fonts/Pretendard-Regular.subset.woff2"],
  ["/styles/fonts/Pretendard-SemiBold.subset.woff2", "styles/fonts/Pretendard-SemiBold.subset.woff2"],
  ["/styles/fonts/Pretendard-Bold.subset.woff2", "styles/fonts/Pretendard-Bold.subset.woff2"]
]);
const mime = {
  ".html": "text/html; charset=utf-8",
  ".css": "text/css; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".woff2": "font/woff2"
};

const server = createServer(async (request, response) => {
  try {
    const { pathname } = new URL(request.url, "http://127.0.0.1");
    const asset = publicFiles.get(pathname);
    if (asset) {
      const filePath = join(root, asset);
      // A linked asset must not turn this route into a reader for another file.
      if (await realpath(filePath) === filePath) {
        const body = await readFile(filePath);
        response.writeHead(200, { "Content-Type": mime[extname(filePath)], "Cache-Control": "no-store" });
        response.end(body);
        return;
      }
    }
  } catch {
    // Missing assets and malformed URLs are handled like unknown routes.
  }
  response.writeHead(404, { "Content-Type": "text/plain; charset=utf-8" });
  response.end("Not found");
});

server.listen(port, "127.0.0.1", () => {
  console.log(`SPECTRA prototype: http://127.0.0.1:${server.address().port}/?variant=A`);
});
