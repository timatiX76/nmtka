// Дополнение к render.mjs: предрендер формул из вариантов НМТ (data/nmt/**.json).
// Рендерит только те формулы, чей <sha1>.svg ещё не существует в assets/latex.
//
//   node render-nmt.mjs [путь-к-датасету] [куда-писать]

import { createHash } from 'node:crypto';
import { existsSync, mkdirSync, readFileSync, readdirSync, writeFileSync } from 'node:fs';
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
  const nmtDir = join(root, 'data', 'nmt');
  const uniq = new Map();
  let files = 0;
  for (const entry of readdirSync(nmtDir, { withFileTypes: true })) {
    if (!entry.isDirectory()) continue;
    for (const f of readdirSync(join(nmtDir, entry.name))) {
      if (!f.endsWith('.json')) continue;
      files += 1;
      let doc1;
      try {
        doc1 = JSON.parse(readFileSync(join(nmtDir, entry.name, f), 'utf8'));
      } catch {
        continue;
      }
      for (const q of doc1.questions ?? []) {
        const sources = [
          q.question_html ?? '',
          q.explanation_html ?? '',
          q.answer_table_html ?? '',
          ...((q.answers ?? []).map((a) => a.html ?? '')),
          ...((q.groups ?? []).flatMap((g) => (g.answers ?? []).map((a) => a.html ?? ''))),
        ];
        for (const t of q.latex ?? []) {
          const v = String(t).trim();
          if (v) uniq.set(v, (uniq.get(v) ?? 0) + 1);
        }
        for (const src of sources) {
          for (const m of String(src).matchAll(/<latex>([\s\S]*?)<\/latex>/g)) {
            const v = m[1].trim();
            if (v) uniq.set(v, (uniq.get(v) ?? 0) + 1);
          }
        }
      }
    }
  }
  return { uniq, files };
}

function renderFormula(t) {
  const node = doc.convert(t, { display: false, em: 16, ex: 8, containerWidth: 1200 });
  return adaptor.outerHTML(node);
}

const t0 = Date.now();
const { uniq, files } = collect();
console.log(`датасет: ${root}`);
console.log(`вариантов-файлов: ${files}, уникальных формул НМТ: ${uniq.size}`);

mkdirSync(outDir, { recursive: true });
const manifestPath = join(outDir, 'manifest.json');
const manifest = existsSync(manifestPath)
  ? JSON.parse(readFileSync(manifestPath, 'utf8'))
  : {};
const errorsPath = join(outDir, 'errors.json');
const errors = existsSync(errorsPath) ? JSON.parse(readFileSync(errorsPath, 'utf8')) : {};

let done = 0;
let skipped = 0;
let bytes = 0;
for (const [formula, count] of uniq) {
  const h = hash(formula);
  manifest[h] = formula;
  const file = join(outDir, `${h}.svg`);
  if (existsSync(file) && count !== null) {
    skipped += 1;
    continue;
  }
  try {
    const body = renderFormula(formula);
    writeFileSync(file, body, 'utf8');
    bytes += Buffer.byteLength(body);
    delete errors[h];
  } catch (e) {
    errors[h] = { formula, error: String(e && e.message ? e.message : e), count };
  }
  done += 1;
  if (done % 200 === 0) console.log(`  отрендерено ${done}…`);
}

writeFileSync(manifestPath, JSON.stringify(manifest), 'utf8');
writeFileSync(errorsPath, JSON.stringify(errors, null, 2), 'utf8');

const secs = ((Date.now() - t0) / 1000).toFixed(1);
console.log(
  `готово: новых ${done}, уже было ${skipped}, ${(bytes / 1024).toFixed(0)} КБ SVG, ` +
    `ошибок ${Object.keys(errors).length}, ${secs} c`,
);
console.log(`каталог: ${outDir}`);
