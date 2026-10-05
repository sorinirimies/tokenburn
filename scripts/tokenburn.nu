#!/usr/bin/env nu
# tokenburn.nu — local AI token/cost usage report for pi + Zed
# https://github.com/sorinirimies (dotfiles: homeDirectoryMac / homeDirectoryLinux)
#
# Reads LOCAL logs only, no network calls:
#   - pi (github.com/earendil-works/pi-coding-agent) session jsonl files:
#       ~/.pi/agent/sessions/**/*.jsonl        (per-turn "message.usage" blocks:
#       input/output/cache read/write tokens + estimated cost)
#   - Zed's agent thread database (sqlite + zstd-compressed json blobs):
#       macOS : ~/Library/Application Support/Zed/threads/threads.db
#       Linux : ~/.local/share/zed/threads/threads.db
#       (per-thread "cumulative_token_usage": input/output tokens only, Zed
#       does not store a $ cost locally, so zed rows always show cost 0)
#
# Requires on PATH: `python3` and `zstd` (only needed for the zed source).
#
# Drop this ONE self-contained file into your Nushell autoload dir (nushell
# sources every file there at startup) and `tokenburn` becomes globally available.
# Autoload files are executed as plain scripts, so this file intentionally
# uses only plain `def`s (no `export def`, no cross-file `use`).
#
# Usage:
#   tokenburn                          -> current month total, pi+zed combined (+ per-tool breakdown)
#   tokenburn --tool pi                -> pi only
#   tokenburn --tool zed               -> zed only
#   tokenburn --hourly                 -> today, bucketed by hour
#   tokenburn --daily                  -> this month, bucketed by day
#   tokenburn --monthly                -> all logged history, bucketed by month
#   tokenburn --daily --all            -> all logged history, bucketed by day
#   tokenburn --monthly --tool pi      -> pi only, bucketed by month, all history

# ── pi: one row per assistant turn, from ~/.pi/agent/sessions/**/*.jsonl ────
# `start`: skip any session file not modified since then (cheap pre-filter;
# exact per-turn filtering by real timestamp still happens by the caller).
def tokenburn--collect-pi [start: datetime] {
    let sessions_dir = ($nu.home-dir | path join ".pi" "agent" "sessions")
    if not ($sessions_dir | path exists) { return [] }

    glob ($sessions_dir | path join "**" "*.jsonl")
    | where {|f| (ls $f | get 0.modified) >= $start }
    | each {|f|
        let project = ($f | path dirname | path basename | str trim --char '-')
        open --raw $f
        | lines
        | each {|line|
            let rec = (try { $line | from json } catch { null })
            let usage = ($rec.message?.usage? | default null)
            if $rec == null or $usage == null or ($rec.timestamp? | default null) == null {
                null
            } else {
                {
                    tool: "pi"
                    project: $project
                    id: ($f | path basename)
                    ts: ($rec.timestamp | into datetime)
                    input: ($usage.input? | default 0)
                    output: ($usage.output? | default 0)
                    cache_read: ($usage.cacheRead? | default 0)
                    cache_write: ($usage.cacheWrite? | default 0)
                    cost: ($usage.cost?.total? | default 0.0)
                }
            }
        }
        | where {|r| $r != null }
    }
    | flatten
}

# ── zed: one row per agent thread, from Zed's threads.db ────────────────────
# (thread-level granularity only: Zed does not store a per-message timestamp
# locally, just cumulative_token_usage + created_at/updated_at per thread.)
def tokenburn--collect-zed [start: datetime] {
    let candidates = [
        ($nu.home-dir | path join "Library" "Application Support" "Zed" "threads" "threads.db")
        ($nu.home-dir | path join ".local" "share" "zed" "threads" "threads.db")
        ($nu.home-dir | path join ".config" "zed" "threads" "threads.db")
    ]
    let matches = ($candidates | where {|p| $p | path exists })
    if ($matches | is-empty) { return [] }
    let db = ($matches | first)
    if (which python3 | is-empty) or (which zstd | is-empty) {
        print "tokenburn: skipping zed (needs `python3` and `zstd` on PATH)"
        return []
    }

    let start_iso = ($start | date to-timezone "UTC" | format date "%Y-%m-%dT%H:%M:%S")
    let py = '
import sqlite3, subprocess, json, sys
db_path = sys.argv[1]
start_iso = sys.argv[2]
con = sqlite3.connect(f"file:{db_path}?mode=ro", uri=True)
cur = con.cursor()
cur.execute("SELECT id, created_at, data_type, data FROM threads WHERE created_at >= ?", (start_iso,))
for id_, created_at, data_type, data in cur.fetchall():
    try:
        if data_type == "zstd":
            raw = subprocess.run(["zstd", "-d", "-c", "-q"], input=data, capture_output=True, check=True).stdout
        else:
            raw = data
        obj = json.loads(raw)
    except Exception:
        continue
    usage = obj.get("cumulative_token_usage") or {}
    model = obj.get("model") or {}
    print(json.dumps({
        "id": id_,
        "created_at": created_at,
        "model": model.get("model"),
        "input_tokens": usage.get("input_tokens", 0),
        "output_tokens": usage.get("output_tokens", 0),
    }))
'
    ^python3 -c $py $db $start_iso
    | lines
    | each {|line|
        let rec = (try { $line | from json } catch { null })
        if $rec == null {
            null
        } else {
            {
                tool: "zed"
                project: ($rec.model? | default "unknown")
                id: $rec.id
                ts: ($rec.created_at | into datetime)
                input: ($rec.input_tokens? | default 0)
                output: ($rec.output_tokens? | default 0)
                cache_read: 0
                cache_write: 0
                cost: 0.0
            }
        }
    }
    | where {|r| $r != null }
}

def tokenburn--summarize [rows: table] {
    if ($rows | is-empty) {
        return { turns: 0, input: 0, output: 0, cache_read: 0, cache_write: 0, total_tokens: 0, cost_usd: 0.0 }
    }
    let input = ($rows | get input | math sum)
    let output = ($rows | get output | math sum)
    let cache_read = ($rows | get cache_read | math sum)
    let cache_write = ($rows | get cache_write | math sum)
    let cost = ($rows | get cost | math sum)
    {
        turns: ($rows | length)
        input: $input
        output: $output
        cache_read: $cache_read
        cache_write: $cache_write
        total_tokens: ($input + $output + $cache_read + $cache_write)
        cost_usd: ($cost | math round -p 4)
    }
}

# Local AI token/cost usage report for pi + Zed session logs on disk (aka "tokenburn").
def "tokenburn" [
    --tool: string = "all"   # pi | zed | all
    --hourly                 # bucket by hour (default window: today)
    --daily                  # bucket by day (default window: this month)
    --monthly                # bucket by month (default window: all logged history)
    --all                    # ignore the default window, use all logged history
] {
    if $tool not-in ["pi" "zed" "all"] {
        error make { msg: $"--tool must be pi, zed, or all \(got '($tool)'\)" }
    }

    let bucket = if $hourly { "hour" } else if $daily { "day" } else if $monthly { "month" } else { "none" }
    let window = if $all {
        "all"
    } else if $hourly {
        "today"
    } else if $monthly {
        "all"
    } else {
        "month"
    }

    let now = (date now)
    let start = if $window == "today" {
        $now | format date "%Y-%m-%dT00:00:00" | into datetime
    } else if $window == "month" {
        $now | format date "%Y-%m-01T00:00:00" | into datetime
    } else {
        "1970-01-01T00:00:00" | into datetime
    }

    mut rows = []
    if $tool == "pi" or $tool == "all" { $rows = ($rows | append (tokenburn--collect-pi $start)) }
    if $tool == "zed" or $tool == "all" { $rows = ($rows | append (tokenburn--collect-zed $start)) }
    let rows = ($rows | where {|r| $r.ts >= $start })

    if ($rows | is-empty) {
        print $"No tokenburn results found for tool=($tool), window=($window)."
        return
    }

    let fmt = if $bucket == "hour" {
        "%Y-%m-%d %H:00"
    } else if $bucket == "day" {
        "%Y-%m-%d"
    } else {
        "%Y-%m"
    }

    if $bucket != "none" {
        let with_bucket = ($rows | each {|r| $r | merge { bucket: ($r.ts | format date $fmt) } })
        return (
            $with_bucket
            | group-by bucket tool --to-table
            | each {|g| (tokenburn--summarize $g.items) | merge { bucket: $g.bucket, tool: $g.tool } }
            | select bucket tool turns input output cache_read cache_write total_tokens cost_usd
            | sort-by bucket tool
        )
    }

    # no bucket: overall total + a per-tool breakdown when covering more than one tool
    let overall = (tokenburn--summarize $rows) | merge { tool: "TOTAL" }
    let per_tool = (
        $rows
        | group-by tool --to-table
        | each {|g| (tokenburn--summarize $g.items) | merge { tool: $g.tool } }
    )
    let out = if $tool == "all" { ($per_tool | append $overall) } else { [$overall] }
    $out | select tool turns input output cache_read cache_write total_tokens cost_usd
}

# Lets this exact file double as a standalone executable (`~/.local/bin/tokenburn`,
# run from any shell) in addition to a sourced nu command (`tokenburn` inside an
# interactive nu session, via config.nu). Confirmed safe: nushell only
# auto-invokes `main` when a file is run directly as `nu <file>.nu`/as a
# shebang script, NOT when it's `source`d while nu loads config.nu at
# interactive startup.
def main [
    --tool: string = "all"   # pi | zed | all
    --hourly                 # bucket by hour (default window: today)
    --daily                  # bucket by day (default window: this month)
    --monthly                # bucket by month (default window: all logged history)
    --all                    # ignore the default window, use all logged history
] {
    tokenburn --tool $tool --hourly=$hourly --daily=$daily --monthly=$monthly --all=$all
}
