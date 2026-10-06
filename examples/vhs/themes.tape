# VHS tape: themes
# 43 built-in colour themes: 'T' opens the picker, '[' / ']' cycle.
# The choice is remembered and shared with the GUI and web dashboard.
#
# Runs on the throw-away data from fixture.sh with HOME and the config dir
# redirected into /tmp, so it never reads your real logs or settings.

Output examples/vhs/generated/themes.gif

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
Type "target/debug/tokenburn-tui --all --daily"
Enter
Sleep 3s
Show

Sleep 1.5s
Type "T"
Sleep 1.5s
Down
Sleep 700ms
Down
Sleep 700ms
Down
Sleep 700ms
Down
Sleep 700ms
Down
Sleep 700ms
Enter
Sleep 2s
# Cycle without opening the picker
Type "]"
Sleep 1.5s
Type "]"
Sleep 1.5s
Type "]"
Sleep 1.5s
Type "["
Sleep 1.5s
Type "["
Sleep 1.5s
Type "q"
Sleep 500ms
