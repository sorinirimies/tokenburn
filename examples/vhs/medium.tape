# VHS tape: medium
# A medium terminal (~95 columns): the cache columns drop out, both charts stay.
#
# Runs on the throw-away data from fixture.sh with HOME and the config dir
# redirected into /tmp, so it never reads your real logs or settings.

Output examples/vhs/generated/medium.gif

Set Shell "bash"
Set FontSize 14
Set Width 820
Set Height 760
Set PlaybackSpeed 1.0
Set Theme "Catppuccin Mocha"

Hide
Type 'cargo build -p tokenburn-tui 2>/dev/null && bash examples/vhs/fixture.sh && echo DONE-$((6*7))'
Enter
Wait+Screen@300s /DONE-42/
Type 'eval "$(examples/vhs/fixture.sh env)" && clear'
Enter
Type "target/debug/tokenburn-tui --all --daily --theme 'Tokyo Night'"
Enter
Sleep 3s
Show

Sleep 2s
Down
Down
Down
Sleep 1s
# The projects view and back: the table keeps only what fits
Type "p"
Sleep 2.5s
Type "p"
Sleep 1.5s
# The pickers fit too
Type "C"
Sleep 800ms
Down
Sleep 600ms
Down
Sleep 600ms
Down
Sleep 1.2s
Escape
Sleep 1s
Type "T"
Sleep 1s
Down
Sleep 600ms
Down
Sleep 1.2s
Escape
Sleep 1s
Type "q"
Sleep 500ms
