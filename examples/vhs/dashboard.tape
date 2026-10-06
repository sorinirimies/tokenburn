# VHS tape: dashboard
# The report table, the stacked token chart and the share-by-project pie,
# then the per-tool window/bucket cycling and the projects view.
#
# Runs on the throw-away data from fixture.sh with HOME and the config dir
# redirected into /tmp, so it never reads your real logs or settings.

Output examples/vhs/generated/dashboard.gif

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
Type "target/debug/tokenburn-tui --all --theme 'Tokyo Night'"
Enter
Sleep 3s
Show

Sleep 2s
# Bucket per day: the bar chart fills with one stacked bar per day
Type "3"
Sleep 3s
# Move through the rows
Down
Down
Down
Sleep 800ms
Down
Down
Sleep 1.5s
# Per month, then back to the total
Type "4"
Sleep 2.5s
Type "1"
Sleep 2s
# One tool at a time
Type "t"
Sleep 1.5s
Type "t"
Sleep 1.5s
Type "t"
Sleep 1.5s
# Projects view
Type "p"
Sleep 3s
Down
Down
Sleep 1s
Type "p"
Sleep 1.5s
Type "q"
Sleep 500ms
