# tokenburn-core

UI-agnostic core of [TokenBurn](https://github.com/sorinirimies/tokenburn):
collectors for the local logs of **pi**, **Zed**, **Claude Code**, **Codex**,
**Copilot CLI**, **Gemini CLI**, **OpenCode** and **Amp**, plus the
aggregation logic shared by the CLI (`tokenburn`), TUI (`tokenburn-tui`) and
GUI (`tokenburn-gui`).

No network access — everything is read from local files.

## Adding a tool

1. Add a variant to `Tool` (`features/usage/types.rs`) — `ALL`, `as_str`, `display_name`.
2. Add `features/<tool>/{mod,ops}.rs` with `collect_<tool>(start) -> Result<Vec<Row>>`, a pure
   `roots()` / `resolve_roots()` (so tests can feed fake environments), and fixture tests.
3. Register it in the `jobs` array in `lib.rs` (`collect`) and in `ToolFilter::ALL`.

Rules every collector follows: honour the tool's own env override, treat a missing directory as
"no rows", skip malformed lines, split cached tokens out of `input` when the tool counts them
inside it, and de-duplicate when the tool logs a message more than once.
