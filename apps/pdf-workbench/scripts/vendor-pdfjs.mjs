// Copies the pdf.js runtime out of node_modules into public/vendor so the web
// UI can render real page thumbnails offline (no CDN).
import { copyFileSync, existsSync, mkdirSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

const packageRoot = fileURLToPath(new URL("..", import.meta.url));

function findPdfjs() {
  let dir = packageRoot;
  for (let i = 0; i < 6; i += 1) {
    const candidate = join(dir, "node_modules", "pdfjs-dist");
    if (existsSync(candidate)) return candidate;
    const parent = join(dir, "..");
    if (parent === dir) break;
    dir = parent;
  }
  return null;
}

const pdfjs = findPdfjs();
if (!pdfjs) {
  console.warn("[vendor-pdfjs] pdfjs-dist not found — thumbnails will be disabled");
  process.exit(0);
}

const outDir = join(packageRoot, "public", "vendor", "pdfjs");
mkdirSync(outDir, { recursive: true });

const wanted = ["pdf.mjs", "pdf.worker.mjs"];
let copied = 0;
for (const file of wanted) {
  const from = join(pdfjs, "build", file);
  if (!existsSync(from)) {
    console.warn(`[vendor-pdfjs] missing ${file}`);
    continue;
  }
  copyFileSync(from, join(outDir, file));
  copied += 1;
}
console.log(`[vendor-pdfjs] copied ${copied}/${wanted.length} files to public/vendor/pdfjs`);
