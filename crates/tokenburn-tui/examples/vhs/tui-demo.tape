# VHS tape — TokenBurn TUI · report browser
# Run: vhs crates/tokenburn-tui/examples/vhs/tui-demo.tape

Output crates/tokenburn-tui/examples/vhs/generated/tui-demo.gif

Set Theme "Catppuccin Mocha"
Set Width 1400
Set Height 900
Set FontSize 14
Set PlaybackSpeed 1.5

Hide
Type "cargo run -q -p tokenburn-tui"
Enter
Sleep 4s
Show

Sleep 2s
# cycle bucket: total → hourly → daily → monthly
Type "2"
Sleep 1500ms
Type "3"
Sleep 1500ms
Type "4"
Sleep 1500ms
# scroll
Type "j"
Sleep 300ms
Type "j"
Sleep 300ms
# per-project view
Type "p"
Sleep 2s
Type "p"
Sleep 1s
# cycle tool
Type "t"
Sleep 1500ms
Type "t"
Sleep 1500ms

Type "q"
Sleep 500ms
