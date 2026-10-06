# VHS tape: options
# 'o' opens the options: toggle the chart panels, switch auto-update off or
# change how often it refreshes (5-300 s).
#
# Runs on the throw-away data from fixture.sh with HOME and the config dir
# redirected into /tmp, so it never reads your real logs or settings.

Output examples/vhs/generated/options.gif

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
Type "target/debug/tokenburn-tui --all --daily --theme 'Gruvbox Dark'"
Enter
Sleep 3s
Show

Sleep 1.5s
Type "o"
Sleep 2s
Down
Sleep 700ms
Space
Sleep 1.5s
Space
Sleep 1.5s
Down
Sleep 700ms
Space
Sleep 1.5s
Space
Sleep 1.5s
Down
Sleep 700ms
Right
Right
Right
Sleep 1.5s
Left
Sleep 1.5s
Escape
Sleep 2s
Type "q"
Sleep 500ms
