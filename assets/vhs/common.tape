# Shared framing; build examples with assets/record.sh before recording.
Set Shell "bash"
Set Theme "Catppuccin Mocha"
Set FontFamily "monospace"
Set FontSize 18
Set Width 1000
Set Height 480
Set Padding 24
Set TypingSpeed 45ms
Set Framerate 20
Set CursorBlink false
Env PS1 "$ "
Env TERM "xterm-256color"
Env CLICOLOR_FORCE "1"

Hide
Type "export PS1='$ '; clear" Enter
Sleep 500ms
Show
