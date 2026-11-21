import { createHash } from 'node:crypto';
import { mkdirSync, readFileSync, readdirSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import { mathjax } from 'mathjax-full/js/mathjax.js';
import { TeX } from 'mathjax-full/js/input/tex.js';
import { SVG } from 'mathjax-full/js/output/svg.js';
import { liteAdaptor } from 'mathjax-full/js/adaptors/liteAdaptor.js';
import { RegisterHTMLHandler } from 'mathjax-full/js/handlers/html.js';
import { AllPackages } from 'mathjax-full/js/input/tex/AllPackages.js';

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(process.argv[2] ?? join(here, '..', '..', '..'));
const outDir = resolve(process.argv[3] ?? join(here, '..', '..', 'assets', 'latex'));

const adaptor = liteAdaptor();
RegisterHTMLHandler(adaptor);
const tex = new TeX({ packages: AllPackages });
const svg = new SVG({ fontCache: 'local' });
const doc = mathjax.document('', { InputJax: tex, OutputJax: svg });

const hash = (s) => createHash('sha1').update(s).digest('hex');

function collect() {
  const dir = join(root, 'data', 'questions');
  const uniq = new Map();
  let files = 0;
  let lines = 0;
  for (const name of readdirSync(dir)) {
    if (!name.endsWith('.jsonl')) continue;
    files += 1;
    const text = readFileSync(join(dir, name), 'utf8');
    for (const line of text.split('\n')) {
      if (!line.trim()) continue;
      lines += 1;
      let q;
      try {
        q = JSON.parse(line);
      } catch {
        continue;
      }
      for (const f of q.latex ?? []) {
        const t = String(f).trim();
        if (t) uniq.set(t, (uniq.get(t) ?? 0) + 1);
      }
    }
  }
  return { uniq, files, lines };
}

function renderFormula(t) {
  const node = doc.convert(t, {
    display: false,
    em: 16,
    ex: 8,
    containerWidth: 1200,
  });
  let html = adaptor.outerHTML(node);
  const i = html.indexOf('<svg');
  const j = html.lastIndexOf('</svg>');
  if (i !== -1 && j !== -1) {
    html = html.slice(i, j + 6);
  }
  return html;
}

const t0 = Date.now();
const { uniq, files, lines } = collect();
console.log(`датасет: ${root}`);
console.log(`файлов: ${files}, заданий: ${lines}, уникальных формул: ${uniq.size}`);

mkdirSync(outDir, { recursive: true });
const manifest = {};
const errors = {};
let done = 0;
let bytes = 0;

for (const [formula, count] of uniq) {
  const h = hash(formula);
  manifest[h] = formula;
  const file = join(outDir, `${h}.svg`);
  try {
    const body = renderFormula(formula);
    writeFileSync(file, body, 'utf8');
    bytes += Buffer.byteLength(body);
  } catch (e) {
    errors[h] = { formula, error: String(e && e.message ? e.message : e), count };
  }
  done += 1;
  if (done % 500 === 0) {
    console.log(`  ${done}/${uniq.size}…`);
  }
}

writeFileSync(join(outDir, 'manifest.json'), JSON.stringify(manifest), 'utf8');
writeFileSync(
  join(outDir, 'errors.json'),
  JSON.stringify(errors, null, 2),
  'utf8',
);

const secs = ((Date.now() - t0) / 1000).toFixed(1);
console.log(
  `готово: ${done} формул, ${(bytes / 1024 / 1024).toFixed(1)} МБ SVG, ` +
    `ошибок ${Object.keys(errors).length}, ${secs} c`,
);
console.log(`каталог: ${outDir}`);
