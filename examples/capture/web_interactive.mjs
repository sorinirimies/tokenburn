// Drives the live web dashboard like a user and saves a numbered screenshot after
// every step, so they can be stitched into a GIF (see all.sh):
//
//   node examples/capture/web_interactive.mjs <port> <out-dir>
//
// load (the bars grow in) → click a bar → click a donut slice → click a legend
// row → clear.  Needs the live mode (asset bundle) and a Chromium started with
// --remote-debugging-port=$DEBUG_PORT (default 9335).
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
const ev = async (expression) => (await send('Runtime.evaluate', { expression, returnByValue: true })).result?.result?.value;
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

let n = 0;
const shot = async () => {
  const s = await send('Page.captureScreenshot', { format: 'png', clip: { x: 100, y: 20, width: 1200, height: 560, scale: 1 } });
  writeFileSync(`${out}/step-${String(++n).padStart(2, '0')}.png`, Buffer.from(s.result.data, 'base64'));
};
const click = async (x, y) => {
  for (const type of ['mouseMoved', 'mousePressed', 'mouseReleased']) {
    await send('Input.dispatchMouseEvent', { type, x, y, button: 'left', clickCount: 1 });
  }
};
const center = (sel, i = 0) => ev(`(() => { const e = document.querySelectorAll(${JSON.stringify(sel)})[${i}]; if (!e) return null; const r = e.getBoundingClientRect(); return [r.x + r.width / 2, r.y + r.height / 2]; })()`);
const step = async (sel, i, wait = 1300) => {
  const c = await center(sel, i);
  if (!c) throw new Error(`nothing to click: ${sel}[${i}]`);
  await send('Input.dispatchMouseEvent', { type: 'mouseMoved', x: c[0], y: c[1] });
  await sleep(350);
  await shot(); // hovering: the outline / pop-out
  await click(...c);
  await sleep(wait);
  await shot();
};

await send('Page.enable');
await send('Emulation.setDeviceMetricsOverride', { width: 1400, height: 1000, deviceScaleFactor: 1, mobile: false });
await send('Page.navigate', { url: `http://localhost:${port}/?bucket=daily&window=all&theme=Tokyo%20Night` });
await sleep(700); // the runtime connects and the entry animation starts
await shot(); await sleep(150); await shot(); await sleep(250); await shot();
await sleep(1500); await shot();

await step('.bars .col', 14);      // a bar
await step('.donut .slice', 1);    // a slice (on top of the bar: intersect)
await step('.legend.vertical .item', 0); // a legend row
await step('.selbar .clear', 0);   // clear
console.log(`wrote ${n} frames to ${out}`);
process.exit(0);
