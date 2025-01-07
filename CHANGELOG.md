# Changelog

## [1.6.1] - 2025-01-07

### Bug Fixes

- Select with a specific option set as selected is not shown as selected in the output
- Update rust crate itertools to 0.14 (#79)

### Miscellaneous Tasks

- Include specific files

## [1.6.0] - 2024-12-18

### Bug Fixes

- Ctrl-c doesn't restore cursor

### Features

- Use prettier cursor for select

### Miscellaneous Tasks

- Release demand version 1.6.0
- Bump mise tools

### Testing

- Run CI on windows/mac

## [1.5.0] - 2024-12-14

### Bug Fixes

- [select] arrow keys should work while filtering

### Features

- [select] allow left/right navigation on filter input
- [select] highlight matched characters when filtering

### Miscellaneous Tasks

- Release demand version 1.5.0

## [1.4.1] - 2024-12-11

### Bug Fixes

- Allow pressing enter to select

### Miscellaneous Tasks

- Release demand version 1.4.1

## [1.4.0] - 2024-12-11

### Features

- Added select descriptions, drop-in filtering, and fuzzy matching

### Miscellaneous Tasks

- Release demand version 1.4.0

## [1.3.0] - 2024-12-09

### Bug Fixes

- Use vhs Docker image to build screencasts (#61)

### Features

- Quit interactive menus with single-key shortcut escape

### Miscellaneous Tasks

- Release demand version 1.3.0

## [1.2.4] - 2024-06-02

### Bug Fixes

- [security] password input renders password on success (#60)

### Miscellaneous Tasks

- Release demand version 1.2.4

## [1.2.3] - 2024-05-23

### Miscellaneous Tasks

- Release demand version 1.2.3
- Update screen recordings (#58)
- Add example and docs for list (#56)

### Update

- Remove leading space from rendered output (#57)

## [1.2.2] - 2024-05-15

### Bug Fixes

- Multiselect clear when filtering
- Multiselect clear when change page
- Select clear when change page
- Select clear when filter could change size
- List clear when stop filtering
- Select not reseting cur page while filtering
- List clear when filtering, for running in spinner
- Multiselect making spinner flicker
- List help had dot while filtering
- Select making spinner flicker
- Input making spinner flicker
- Confirm making spinner flicker
- Select name going off screen
- List name being off screen sometimes
- Typo

### Features

- Multiselect show pages without description
- Show pages even when there is no descroption
- Render help when multiselect filtering
- Multiselect filter uses custom cursor
- Select help renders while filtering
- Select filter uses custom cursor
- SpinnerActionRunner.title now accepts into<string>

### Miscellaneous Tasks

- Release demand version 1.2.2
- Release demand version 1.2.1
- Update tests
- Add list to spinner prompts example
- Remove space that was really annoying me

## [1.2.0] - 2024-05-15

### Features

- Add dialog with variable buttons (#54)
- List (#51)

### Miscellaneous Tasks

- Release demand version 1.2.0

## [1.1.2] - 2024-04-27

### Miscellaneous Tasks

- Release demand version 1.1.2

## [1.1.1] - 2024-04-23

### Bug Fixes

- Clippy warnings
- Do not reveal whitespace when masked (#42)
- Remove unused variables from examples

### Features

- DemandOption no longer requires item to impl Display, Select and MultiSelect trait bounds updated to reflect that (#47)

### Miscellaneous Tasks

- Release demand version 1.1.1

## [1.1.0] - 2024-02-22

### Features

- Add input autocompletion (#39)

### Miscellaneous Tasks

- Release demand version 1.1.0
- Update example gifs
- Remove unnecessary println statements from examples

## [1.0.2] - 2024-02-15

### Bug Fixes

- Input - panics if charaters with more than one unicode points are used (#37)
- Input - always renders default prompt if inline (#33)

### Features

- Add input validation (#34)

### Miscellaneous Tasks

- Release demand version 1.0.2
- Update README.md

## [1.0.1] - 2024-01-25

### Miscellaneous Tasks

- Release demand version 1.0.1

## [1.0.0] - 2024-01-25

### Bug Fixes

- Input - remove unnecessary code (#26)
- Select - incorrect number of pages when filtering (#24)
- Input - set default prompt to '> ' (#20)
- Indent input by a space like the other inputs (#18)
- Input - handle arrow keys (#11)

### Features

- Spinner - expose SpinnerStyle and add example (#28)
- Input - add support for ctrl+w and ctrl-u (#27)
- Add tests to verify initial rendering (#25)
- Add themes dracula, catppuccin, base16 (#19)
- Implement spinner (#13)

### Miscellaneous Tasks

- Release demand version 1.0.0
- Add changelog via git-cliff (#29)
- Align and update examples (#14)

## [0.4.0] - 2024-01-18

### Bug Fixes

- Select might panic when filtering multi-page options (#5)

### Features

- Implement simple text input (#10)

### Miscellaneous Tasks

- Release demand version 0.4.0

## [0.3.0] - 2023-12-21

### Miscellaneous Tasks

- Release demand version 0.3.0

## [0.2.0] - 2023-12-21

### Miscellaneous Tasks

- Release demand version 0.2.0

## [0.1.1] - 2023-12-21

### Miscellaneous Tasks

- Release demand version 0.1.1


