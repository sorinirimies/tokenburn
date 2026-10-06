// Captures screenshots of the running web dashboard through a Chromium-based
// browser's remote-debugging port (Node ≥ 22: global fetch + WebSocket).
//
//   node examples/capture/web.mjs <port> <out-dir> [theme ...]
//
// For each theme: select it in the live page (exactly what a user does), wait for
// the shard to re-render, and save <out-dir>/web-<theme>.png.
import { writeFileSync, mkdirSync } from 'node:fs';

const [, , port, outDir, ...themes] = process.argv;
const DEBUG = process.env.DEBUG_PORT || '9335';
mkdirSync(outDir, { recursive: true });

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

await send('Page.enable');
await send('Emulation.setDeviceMetricsOverride', { width: 1400, height: 1000, deviceScaleFactor: 1, mobile: false });
await send('Page.navigate', { url: `http://localhost:${port}/?bucket=daily&window=all` });
await sleep(3500);

for (const name of themes.length ? themes : ['Default']) {
  await ev(`(() => { const s = [...document.querySelectorAll('select')].pop(); s.value = ${JSON.stringify(name)}; s.dispatchEvent(new Event('change', { bubbles: true })); })()`);
  await sleep(2200);
  const shot = await send('Page.captureScreenshot', { format: 'png' });
  const file = `${outDir}/web-${name.toLowerCase().replace(/[^a-z0-9]+/g, '-')}.png`;
  writeFileSync(file, Buffer.from(shot.result.data, 'base64'));
  console.log('wrote', file);
}
process.exit(0);
