# tokenburn-web

Server-rendered web dashboard ([Topcoat](https://github.com/tokio-rs/topcoat)) for [TokenBurn](https://github.com/sorinirimies/tokenburn) — local AI token/cost usage for your AI coding agents (**pi**, **Zed**, **Claude Code**, **Codex**, **Copilot CLI**, **Gemini CLI**, **OpenCode**, **Amp**).

```sh
just run-web                       # live dashboard, http://127.0.0.1:3000
tokenburn-web --port 8080 --no-open
HOST=0.0.0.0 PORT=8080 tokenburn-web   # env works too (exposes it on the network, no auth!)
tokenburn-web                      # static fallback when no asset bundle is present

# flags: --host (default 127.0.0.1)  --port (default 3000)  --no-open (skip opening the browser)
```

**Live mode** (needs `cargo install topcoat-cli` + the asset bundle, see `just bundle-web`): filters are signals bound to the selects, and the data region re-reads the logs every 5 s over a WebSocket — no reloads. **Static mode** (no bundle): GET-form filters and a 60 s `<meta refresh>`.

| URL | Description |
|-----|-------------|
| `/?tool=pi&bucket=daily&window=month` | HTML dashboard (all params optional) |
| `/api/report?tool=all&bucket=monthly` | same report as JSON |
| `/api/health` | liveness probe |

Logs are read locally on every request; nothing leaves the machine. The server binds to localhost by default — token/cost data is private.
