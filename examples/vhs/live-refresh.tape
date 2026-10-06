# VHS tape: live-refresh
# Auto-update: while the dashboard is open, a fresh turn is logged and the
# numbers change by themselves (no key pressed). The 'turns' count ticks up.
#
# Runs on the throw-away data from fixture.sh with HOME and the config dir
# redirected into /tmp, so it never reads your real logs or settings.

Output examples/vhs/generated/live-refresh.gif

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
Type "(sleep 9; bash examples/vhs/fixture.sh append) >/dev/null 2>&1 &"
Enter
Type "clear"
Enter
Type "target/debug/tokenburn-tui --daily"
Enter
Sleep 3s
Show

# Watch the header and the status line: the refresh lands a few seconds after
# the new log line is written.
Sleep 16s
Type "q"
Sleep 500ms
