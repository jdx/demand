Output assets/grid-select.gif
Require target/debug/examples/grid_select
Source assets/vhs/common.tape

Type "target/debug/examples/grid_select" Enter
Wait+Screen /Choose a version/
Sleep 1s
# Right skips the unavailable Range cell in the first row.
Right
Sleep 900ms
Down
Right
Sleep 900ms
Down
Right 2
Sleep 900ms
# Change one filtered row, then reveal the retained choices.
Type "/chalk"
Sleep 1s
Enter
Right
Sleep 900ms
Escape
Sleep 1s
Enter
Sleep 2s
