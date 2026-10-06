#!/usr/bin/env bash
# Builds the throw-away data the TokenBurn demos (VHS tapes, screenshots) run on,
# so no real session log ever ends up in a recording.
#
#   examples/vhs/fixture.sh            create /tmp/tokenburn-demo-home (35 days of
#                                      logs for all eight tools) — deterministic
#   examples/vhs/fixture.sh append     add one fresh pi turn "now" (live-refresh demo)
#   examples/vhs/fixture.sh env        print the environment the demos run with
#
# Run the apps against it with:   eval "$(examples/vhs/fixture.sh env)"
set -euo pipefail

HOME_DIR=/tmp/tokenburn-demo-home

if [ "${1:-}" = "env" ]; then
    cat <<ENV
export HOME=$HOME_DIR
export TOKENBURN_CONFIG_DIR=$HOME_DIR/.config/tokenburn
export TOKENBURN_ZED_DB=$HOME_DIR/zed/threads.db
export XDG_DATA_HOME=$HOME_DIR/.local/share
ENV
    exit 0
fi

if [ "${1:-}" = "append" ]; then
    f=$(ls -t "$HOME_DIR"/.pi/agent/sessions/*/*.jsonl | head -1)
    python3 - "$f" <<'PY'
import json, sys, random, datetime
now = datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%S.000Z")
inp, out = random.randint(2, 12), random.randint(400, 2400)
cr, cw = random.randint(40_000, 160_000), random.randint(0, 9_000)
cost = round(cr * 0.0000003 + out * 0.000015 + cw * 0.00000375, 4)
line = {"timestamp": now, "message": {"role": "assistant", "usage": {
    "input": inp, "output": out, "cacheRead": cr, "cacheWrite": cw, "cost": {"total": cost}}}}
open(sys.argv[1], "a").write(json.dumps(line) + "\n")
PY
    exit 0
fi

rm -rf "$HOME_DIR"
mkdir -p "$HOME_DIR/.config/tokenburn" "$HOME_DIR/zed"

python3 - "$HOME_DIR" <<'PY'
import datetime as dt, json, os, random, sqlite3, sys
root = sys.argv[1]
rnd = random.Random(20260506)          # deterministic: same demo every time
now = dt.datetime.now(dt.timezone.utc).replace(microsecond=0)
DAYS = 35

def iso(t): return t.strftime("%Y-%m-%dT%H:%M:%S.000Z")
def write(path, text, mode="w"):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, mode) as f: f.write(text)

def day_weight(d):
    # busier on weekdays, a gentle upward trend, the odd quiet day
    w = 0.25 if d.weekday() >= 5 else 1.0
    w *= 0.6 + 0.8 * (DAYS - (now.date() - d).days) / DAYS
    return w * rnd.choice([0.2, 0.7, 1, 1, 1.2, 1.6])

def times(d, n):
    base = dt.datetime.combine(d, dt.time(8, 0), tzinfo=dt.timezone.utc)
    return sorted(base + dt.timedelta(minutes=rnd.randint(0, 600)) for _ in range(n))

PROJECTS = ["atlas", "orbit", "nova", "helix"]
days = [now.date() - dt.timedelta(days=i) for i in range(DAYS - 1, -1, -1)]

# ── pi ───────────────────────────────────────────────────────────────────────
for d in days:
    for proj, share in zip(PROJECTS[:3], (0.5, 0.3, 0.2)):
        n = int(round(40 * day_weight(d) * share))
        lines = []
        for t in times(d, n):
            if t > now: continue
            out = rnd.randint(300, 3200); cr = rnd.randint(30_000, 220_000)
            cw = rnd.randint(0, 14_000); inp = rnd.randint(2, 30)
            cost = round(cr * 3e-7 + out * 1.5e-5 + cw * 3.75e-6 + inp * 3e-6, 4)
            lines.append(json.dumps({"timestamp": iso(t), "message": {"role": "assistant", "usage": {
                "input": inp, "output": out, "cacheRead": cr, "cacheWrite": cw, "cost": {"total": cost}}}}))
        if lines:
            write(f"{root}/.pi/agent/sessions/--Users-demo-Projects-{proj}--/{d}_{proj}.jsonl", "\n".join(lines) + "\n")

# ── Claude Code ──────────────────────────────────────────────────────────────
for d in days:
    n = int(round(22 * day_weight(d)))
    lines = []
    for k, t in enumerate(times(d, n)):
        if t > now: continue
        lines.append(json.dumps({"timestamp": iso(t), "requestId": f"req-{d}-{k}", "sessionId": str(d),
            "message": {"id": f"msg-{d}-{k}", "model": "claude-sonnet", "usage": {
                "input_tokens": rnd.randint(2, 40), "output_tokens": rnd.randint(200, 2600),
                "cache_creation_input_tokens": rnd.randint(0, 12_000),
                "cache_read_input_tokens": rnd.randint(20_000, 180_000)}}}))
    if lines:
        write(f"{root}/.claude/projects/-Users-demo-Projects-atlas/{d}.jsonl", "\n".join(lines) + "\n")

# ── Codex CLI (cumulative totals) ────────────────────────────────────────────
for d in days:
    if d.weekday() >= 5 and rnd.random() < 0.7: continue
    n = int(round(14 * day_weight(d)))
    inp = cached = out = 0
    lines = [json.dumps({"timestamp": iso(dt.datetime.combine(d, dt.time(9), tzinfo=dt.timezone.utc)),
                         "type": "session_meta", "payload": {"cwd": "/Users/demo/Projects/orbit"}})]
    for t in times(d, n):
        if t > now: continue
        di = rnd.randint(8_000, 60_000); dc = int(di * rnd.uniform(0.3, 0.8)); do = rnd.randint(150, 1800)
        inp += di; cached += dc; out += do
        lines.append(json.dumps({"timestamp": iso(t), "type": "event_msg", "payload": {"type": "token_count", "info": {
            "total_token_usage": {"input_tokens": inp, "cached_input_tokens": cached, "output_tokens": out}}}}))
    if len(lines) > 1:
        write(f"{root}/.codex/sessions/{d.year}/{d.month:02d}/{d.day:02d}/rollout-{d}.jsonl", "\n".join(lines) + "\n")

# ── Copilot CLI (one shutdown event per session) ─────────────────────────────
for d in days:
    if rnd.random() < 0.45: continue
    t = dt.datetime.combine(d, dt.time(15), tzinfo=dt.timezone.utc)
    if t > now: continue
    write(f"{root}/.copilot/session-state/{d}.jsonl", json.dumps({"type": "session.shutdown", "timestamp": iso(t),
        "data": {"modelMetrics": {"gpt-5-mini": {"usage": {
            "inputTokens": rnd.randint(60_000, 400_000), "outputTokens": rnd.randint(2_000, 20_000),
            "cacheReadTokens": rnd.randint(10_000, 120_000), "cacheWriteTokens": 0}}}}}) + "\n")

# ── Gemini CLI ───────────────────────────────────────────────────────────────
for d in days:
    if rnd.random() < 0.5: continue
    msgs = []
    for t in times(d, rnd.randint(3, 12)):
        if t > now: continue
        inp = rnd.randint(8_000, 90_000)
        msgs.append({"type": "gemini", "timestamp": iso(t), "model": "gemini-2.5-pro", "tokens": {
            "input": inp, "output": rnd.randint(200, 2400), "cached": int(inp * rnd.uniform(0, 0.6)),
            "thoughts": rnd.randint(0, 1500), "tool": 0}})
    if msgs:
        write(f"{root}/.gemini/tmp/a1b2c3/chats/session-{d}.json",
              json.dumps({"sessionId": str(d), "startTime": iso(dt.datetime.combine(d, dt.time(10), tzinfo=dt.timezone.utc)), "messages": msgs}))

# ── OpenCode (sqlite) ────────────────────────────────────────────────────────
db = f"{root}/.local/share/opencode/opencode.db"
os.makedirs(os.path.dirname(db), exist_ok=True)
c = sqlite3.connect(db)
c.execute("CREATE TABLE message (id TEXT, session_id TEXT, time_created INTEGER, data TEXT)")
k = 0
for d in days:
    for t in times(d, int(round(18 * day_weight(d)))):
        if t > now: continue
        k += 1
        ms = int(t.timestamp() * 1000)
        data = {"role": "assistant", "modelID": "claude-sonnet", "providerID": "anthropic", "time": {"created": ms},
                "path": {"cwd": "/Users/demo/Projects/nova"}, "cost": round(rnd.uniform(0.01, 0.35), 4),
                "tokens": {"input": rnd.randint(2, 60), "output": rnd.randint(200, 2400), "reasoning": rnd.randint(0, 600),
                           "cache": {"read": rnd.randint(10_000, 120_000), "write": rnd.randint(0, 9_000)}}}
        c.execute("INSERT INTO message VALUES (?,?,?,?)", (f"m{k}", f"s{d}", ms, json.dumps(data)))
c.commit(); c.close()

# ── Amp ──────────────────────────────────────────────────────────────────────
for d in days:
    if rnd.random() < 0.55: continue
    msgs = [{"role": "assistant", "usage": {"model": "amp-smart", "timestamp": iso(t),
             "inputTokens": rnd.randint(1_000, 30_000), "outputTokens": rnd.randint(100, 1500),
             "cacheCreationInputTokens": rnd.randint(0, 5_000), "cacheReadInputTokens": rnd.randint(5_000, 60_000)}}
            for t in times(d, rnd.randint(2, 9)) if t <= now]
    if msgs:
        write(f"{root}/.local/share/amp/threads/T-{d}.json", json.dumps({"messages": msgs}))

# ── Zed (zstd JSON threads) ──────────────────────────────────────────────────
import subprocess, shutil
zdb = f"{root}/zed/threads.db"
c = sqlite3.connect(zdb)
c.execute("CREATE TABLE threads (id TEXT PRIMARY KEY, summary TEXT, updated_at TEXT, data_type TEXT, data BLOB, created_at TEXT)")
have_zstd = shutil.which("zstd") is not None
for d in days:
    for k, t in enumerate(times(d, rnd.randint(0, 3))):
        if t > now: continue
        doc = json.dumps({"model": {"model": "claude-sonnet-4"}, "cumulative_token_usage": {
            "input_tokens": rnd.randint(5_000, 80_000), "output_tokens": rnd.randint(500, 9_000)}}).encode()
        if have_zstd:
            blob, kind = subprocess.run(["zstd", "-q", "-c"], input=doc, capture_output=True).stdout, "zstd"
        else:
            blob, kind = doc, "json"
        c.execute("INSERT INTO threads VALUES (?,?,?,?,?,?)", (f"z-{d}-{k}", "demo", iso(t), kind, blob, iso(t)))
c.commit(); c.close()
print("demo data written")
PY
echo "demo home: $HOME_DIR"
