// Screenshots the web dashboard at the three layout classes (wide / medium / narrow;
// the breakpoints are the ones in tokenburn-core's `LayoutClass`).
//
//   node examples/capture/responsive.mjs <port> <out-dir>
//
// Needs a Chromium started with --remote-debugging-port=$DEBUG_PORT (default 9335).
import { writeFileSync, mkdirSync } from 'node:fs';

const [, , port, out] = process.argv;
mkdirSync(out, { recursive: true });
const DEBUG = process.env.DEBUG_PORT || '9335';
const targets = await (await fetch(`http://localhost:${DEBUG}/json`)).json();
const page = targets.find((t) => t.type === 'page');
const ws = new WebSocket(page.webSocketDebuggerUrl);
let id = 0;
const pending = new Map();
ws.onmessage = (e) => {
  const m = JSON.parse(e.data);
  if (m.id && pending.has(m.id)) { pending.get(m.id)(m); pending.delete(m.id); }
};
await new Promise((r) => (ws.onopen = r));
const send = (method, params = {}) => new Promise((r) => { const i = ++id; pending.set(i, r); ws.send(JSON.stringify({ id: i, method, params })); });
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

await send('Page.enable');
// [name, width, height, mobile]
for (const [name, w, h, mobile] of [['wide', 1280, 860, false], ['medium', 820, 1000, false], ['narrow', 390, 1400, true]]) {
  await send('Emulation.setDeviceMetricsOverride', { width: w, height: h, deviceScaleFactor: 1, mobile });
  await send('Page.navigate', { url: `http://localhost:${port}/?bucket=daily&window=all&theme=Tokyo%20Night` });
  await sleep(3500);
  const shot = await send('Page.captureScreenshot', { format: 'png' });
  writeFileSync(`${out}/web-${name}.png`, Buffer.from(shot.result.data, 'base64'));
  const r = await send('Runtime.evaluate', { expression: 'document.documentElement.scrollWidth - document.documentElement.clientWidth', returnByValue: true });
  // A horizontal overflow would mean the layout is broken at this width.
  if (r.result.result.value > 0) { console.error(`${name}: page overflows horizontally by ${r.result.result.value}px`); process.exit(1); }
  console.log('wrote', `${out}/web-${name}.png`);
}
process.exit(0);
