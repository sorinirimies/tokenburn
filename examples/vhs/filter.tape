# VHS tape: filter
# Live filtering: '/' free text (AND terms, -exclude) and 'f' per-tool checkboxes.
# Esc clears the filter; a second Esc quits.
#
# Runs on the throw-away data from fixture.sh with HOME and the config dir
# redirected into /tmp, so it never reads your real logs or settings.

Output examples/vhs/generated/filter.gif

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
Type "target/debug/tokenburn-tui --all --daily --theme 'Dracula'"
Enter
Sleep 3s
Show

Sleep 2s
# Free text: matches tool names, projects and bucket labels
Type "/"
Sleep 600ms
Type "pi"
Sleep 2.5s
# Add a second term: both must match (AND)
Type " atlas"
Sleep 2.5s
Enter
Sleep 2s
# Esc clears the filter
Escape
Sleep 2s

# Exclusions
Type "/"
Type "-pi -zed"
Sleep 3s
Enter
Sleep 2s
Escape
Sleep 1.5s

# Per-tool checkboxes with row counts
Type "f"
Sleep 2s
Down
Space
Sleep 1s
Down
Space
Sleep 1s
Down
Down
Space
Sleep 2s
Type "n"
Sleep 1.5s
Type "a"
Sleep 2s
Escape
Sleep 1.5s
Type "q"
Sleep 500ms
