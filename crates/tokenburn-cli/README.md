# tokenburn

Local AI token/cost usage report for your AI coding agents (**pi**, **Zed**, **Claude Code**, **Codex**, **Copilot CLI**, **Gemini CLI**, **OpenCode**, **Amp**) — a [clap](https://docs.rs/clap) CLI.
Part of [TokenBurn](https://github.com/sorinirimies/tokenburn); see also `tokenburn-tui` and `tokenburn-gui`.

```sh
tokenburn                  # this month, all tools combined
tokenburn --tool pi        # pi only
tokenburn --hourly         # today by hour
tokenburn --daily          # this month by day
tokenburn --monthly        # all history by month
tokenburn --daily --all    # all history by day
tokenburn --json           # JSON output
```
