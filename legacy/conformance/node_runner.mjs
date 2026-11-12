// Runs contracts/fields.json through the Node binding as hyper's runners do and prints the actual results as JSON.
// A urlencoded text is the UTF-8 encoding of its JSON string; a multipart type and body are text whose characters are
// their bytes. The runner holds no rule: it calls the binding and records what it returns. A body is fed through the
// promise of RequestBody.feed, which reads on the libuv pool.

import { createRequire } from 'node:module';
import { mkdtempSync, readdirSync, readFileSync, rmSync, statSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

const forms = createRequire(import.meta.url)(process.argv[3]);

const text = (bytes) => Buffer.from(bytes).toString('latin1');
const bytes = (value) => Buffer.from(value, 'latin1');
const fieldsOf = (pairs) => (pairs === null ? null : pairs.map(([name, values]) => [name, [...values]]));
const filesOf = (pairs, size = false) =>
  pairs === null
    ? null
    : pairs.flatMap(([, items]) =>
        items.map((item) => ({
          name: item.name(),
          filename: item.filename(),
          contentType: item.contentType(),
          content: text(item.content()),
          ...(size ? { size: item.size() } : {}),
        })),
      );

function failure(work) {
  try {
    work();
    return null;
  } catch (error) {
    if (error instanceof TypeError) return [error.message, 'type'];
    if (error instanceof RangeError) return [error.message, 'value'];
    throw error;
  }
}

function urlencoded(c) {
  const pairs = forms.parseUrlencoded(Buffer.from(c.text, 'utf8'));
  return { fields: fieldsOf(pairs), names: pairs === null ? null : pairs.map(([name]) => name) };
}

function multipart(c) {
  const body = bytes(c.body);
  const whole = [fieldsOf(forms.fieldsFromBody(c.type, body)), filesOf(forms.filesFromBody(c.type, body))];
  let result = whole;
  const boundary = forms.multipartBoundary(c.type);
  if (boundary !== null) {
    for (let at = 0; at <= body.length; at++) {
      const parts = forms.parseMultipartChunks([body.subarray(0, at), body.subarray(at)], boundary);
      const divided = [fieldsOf(parts === null ? null : parts[0]), filesOf(parts === null ? null : parts[1])];
      if (JSON.stringify(divided) !== JSON.stringify(whole)) {
        result = divided;
        break;
      }
    }
  }
  return { fields: result[0], files: result[1] };
}

async function streamed(c) {
  const body = bytes(c.body);
  const read = new forms.RequestBody(c.type, c.limit, c.stream === true, c.valueLimit, c.partLimit, c.fileLimit);
  const scratch = mkdtempSync(join(tmpdir(), 'http-forms-case-'));
  const out = {};
  try {
    let stopped = false;
    for (let at = 0; at < body.length && !stopped; at += c.chunk) {
      stopped = !(await read.feed(body.subarray(at, at + c.chunk)));
    }
    const held = read.held();
    let fields, files;
    if (read.streamed()) {
      const parts = read.parts();
      fields = parts === null ? null : parts[0];
      files = parts === null ? null : parts[1];
    } else {
      fields = stopped ? null : forms.fieldsFromBody(c.type, held);
      files = stopped ? null : forms.filesFromBody(c.type, held);
    }
    const directory = read.directory();
    const count = () => (directory === null ? null : readdirSync(directory).length);
    out.stopped = stopped;
    out.exceeded = read.exceeded();
    out.held = text(held);
    out.fields = fieldsOf(fields);
    out.files = filesOf(files, true);
    out.entries = count();
    const pairs = files === null ? [] : files;
    if (c.move === 'none' || pairs.length === 0) {
      out.moved = null;
    } else {
      const first = pairs[0][1][0];
      const target = join(scratch, 'target');
      if (c.move === 'exists') writeFileSync(target, 'taken');
      let failed = failure(() => first.moveTo(c.move === 'type' ? 7 : target));
      let after = null;
      let content;
      if (failed === null) {
        content = text(readFileSync(target));
        failed = failure(() => first.moveTo(join(scratch, 'second')));
        const readFailure = failure(() => first.content());
        after = readFailure === null ? null : readFailure[0];
      } else {
        content = text(first.content());
      }
      out.moved = { content, left: count(), again: failed?.[0] ?? null, read: after, class: failed?.[1] ?? null };
    }
    read.remove();
    let exists = false;
    try {
      exists = directory !== null && statSync(directory).isDirectory();
    } catch {
      exists = false;
    }
    out.removed = directory === null ? null : !exists;
  } finally {
    read.remove();
    rmSync(scratch, { recursive: true, force: true });
  }
  return out;
}

const cases = JSON.parse(readFileSync(process.argv[2], 'utf8'));
const out = { urlencoded: {}, multipart: {}, streamed: {} };
for (const c of cases.urlencoded) out.urlencoded[c.id] = urlencoded(c);
for (const c of cases.multipart) out.multipart[c.id] = multipart(c);
for (const c of cases.streamed) out.streamed[c.id] = await streamed(c);
console.log(JSON.stringify(out));
