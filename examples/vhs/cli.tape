# VHS tape: cli
# The CLI: a table by default, buckets, one tool, and --json for scripts.
#
# Runs on the throw-away data from fixture.sh with HOME and the config dir
# redirected into /tmp, so it never reads your real logs or settings.

Output examples/vhs/generated/cli.gif

Set Shell "bash"
Set FontSize 14
Set Width 1600
Set Height 900
Set PlaybackSpeed 1.0
Set Theme "Catppuccin Mocha"

Hide
Type 'cargo build -p tokenburn-tui -p tokenburn 2>/dev/null && bash examples/vhs/fixture.sh && echo DONE-$((6*7))'
Enter
Wait+Screen@300s /DONE-42/
Type 'eval "$(examples/vhs/fixture.sh env)" && clear'
Enter
Show

Type "target/debug/tokenburn"
Enter
Sleep 3s
Type "clear"
Enter
Type "target/debug/tokenburn --daily --all | head -24"
Enter
Sleep 4s
Type "clear"
Enter
Type "target/debug/tokenburn --tool claude --monthly"
Enter
Sleep 3s
Type "clear"
Enter
Type "target/debug/tokenburn --monthly --json | head -30"
Enter
Sleep 4s
Type "clear"
Enter
Type `target/debug/tokenburn --daily --all --json | jq -r '.[] | select(.tool=="pi") | [.bucket, .cost_usd] | @tsv' | tail -5`
Enter
Sleep 4s
