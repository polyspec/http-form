import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
// Times hyper's Node reader and the binding on the bodies of generate.py. Arguments: a directory with the TypeScript
// sources of hyper's server whose imports name .ts files, the directory of the bodies and the path of the addon.
const [source, data, addon] = process.argv.slice(2);
const { Fields } = await import(`${source}/fields.ts`);
const { Multipart } = await import(`${source}/multipart.ts`);
const { RequestBody: HyperBody } = await import(`${source}/request-body.ts`);
const forms = createRequire(import.meta.url)(addon);

const kind = readFileSync(`${data}/type.txt`, 'utf8');
const boundary = kind.split('=')[1];
const text = readFileSync(`${data}/form.txt`);
const form = readFileSync(`${data}/form.bin`);
const large = readFileSync(`${data}/large.bin`);
const CHUNK = 65536, BIG = 2 ** 40;
const chunks = (d: Buffer) => { const out = []; for (let i = 0; i < d.length; i += CHUNK) out.push(d.subarray(i, i + CHUNK)); return out; };
async function median(work: () => unknown, runs: number) {
  const t: number[] = [];
  for (let i = 0; i < runs; i++) { const s = performance.now(); await work(); t.push(performance.now() - s); }
  t.sort((a, b) => a - b); return t[Math.floor(t.length / 2)];
}
async function hyperStream(d: Buffer) {
  const b = new HyperBody(kind, BIG, true, BIG, BIG, BIG);
  for (const c of chunks(d)) b.feed(c);
  await b.written(); if (b.parts() === null) throw new Error('no parts'); await b.remove();
}
async function coreStream(d: Buffer) {
  const b = new forms.RequestBody(kind, BIG, true, BIG, BIG, BIG);
  for (const c of chunks(d)) await b.feed(c);
  if (b.parts() === null) throw new Error('no parts'); b.remove();
}
function coreStreamSync(d: Buffer) {
  const b = new forms.RequestBody(kind, BIG, true, BIG, BIG, BIG);
  for (const c of chunks(d)) b.feedSync(c);
  if (b.parts() === null) throw new Error('no parts'); b.remove();
}
const urlText = text.toString('latin1');
const out: Record<string, number[]> = {
  'urlencoded 1000 pairs': [await median(() => Fields.parse(text), 300), await median(() => forms.parseUrlencoded(text), 300)],
  'multipart form, whole': [await median(() => Multipart.read(form, boundary), 300), await median(() => forms.parseMultipartChunks([form], boundary), 300)],
  'multipart form, streamed': [await median(() => hyperStream(form), 100), await median(() => coreStream(form), 100), await median(() => coreStreamSync(form), 100)],
  '50 MiB attachment, streamed': [await median(() => hyperStream(large), 7), await median(() => coreStream(large), 7), await median(() => coreStreamSync(large), 7)],
};
console.log(JSON.stringify({ node: out }));
