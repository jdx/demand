# Changelog

## [2.0.0](https://github.com/jdx/demand/compare/v1.8.2...v2.0.0) - 2026-02-20

### Added

- add Wizard component for multi-step wizard flows ([#127](https://github.com/jdx/demand/pull/127))

### Other

- *(deps)* update dependency cargo:cargo-release to v1 ([#135](https://github.com/jdx/demand/pull/135))
- *(deps)* update release-plz/action digest to f708778 ([#134](https://github.com/jdx/demand/pull/134))
- *(deps)* update actions/checkout action to v6 ([#133](https://github.com/jdx/demand/pull/133))
- *(deps)* update dependency hk to v1.36.0 ([#132](https://github.com/jdx/demand/pull/132))
- *(deps)* update actions/checkout digest to de0fac2 ([#131](https://github.com/jdx/demand/pull/131))
- *(deps)* pin dependencies ([#130](https://github.com/jdx/demand/pull/130))

## [1.8.2](https://github.com/jdx/demand/compare/v1.8.1...v1.8.2) - 2026-01-31

### Fixed

- improve non-TTY support, input parsing, and ambiguous key handling for confirm dialog ([#115](https://github.com/jdx/demand/pull/115))
- *(deps)* update rust crate signal-hook to 0.4 ([#124](https://github.com/jdx/demand/pull/124))

### Other

- add release-plz for automated releases ([#125](https://github.com/jdx/demand/pull/125))

## [1.8.1] - 2025-12-21

### Bug Fixes

- Select panics when ENTER is pressed with all options filtered out (#122)

## [1.8.0] - 2025-11-27

### Bug Fixes

- Add custom vhs Docker image and fix tasks for vhs recordings

### Documentation

- Add example for input autocompletion

### Features

- Add custom autocompleter api (#116)

### Miscellaneous Tasks

- Release demand version 1.8.0
- Losen git-cliff version

## [1.7.2] - 2025-09-30

### Bug Fixes

- Check stdin instead of stderr for TTY detection

### Miscellaneous Tasks

- Release demand version 1.7.2

## [1.7.1] - 2025-09-30

### Bug Fixes

- Support non-TTY input for automated testing (#113)
- Update rust crate console to 0.16 (#105)

### Miscellaneous Tasks

- Release demand version 1.7.1
- Fix uninlined_format_args linter errors

## [1.7.0] - 2025-05-02

### Bug Fixes

- Demand 1.6.4 clears screen on Select::run (#102)

### Features

- Set input default value (#103)
- Add validation input trait (#101)

### Miscellaneous Tasks

- Release demand version 1.7.0

### Refactor

- Remove `once_cell` dependency in favour of `std::sync::LazyLock` (#98)

## [1.6.5] - 2025-03-15

### Bug Fixes

- Prompt shows input history on every typed character (#95)

### Miscellaneous Tasks

- Release demand version 1.6.5
- Edition 2024 (#96)

## [1.6.4] - 2025-02-26

### Bug Fixes

- Clear entire screen instead of removing lines by widget height (#88)

### Documentation

- Align and fix doc comments (#92)

### Miscellaneous Tasks

- Release demand version 1.6.4

## [1.6.3] - 2025-02-17

### Bug Fixes

- Ctrl-c doesn't restore cursor after Select::run (#90)

### Miscellaneous Tasks

- Release demand version 1.6.3

## [1.6.2] - 2025-01-24

### Bug Fixes

- Add .DS_Store, .vscode to .gitignore (#86)
- White flash in select when filtering: true (#83)

### Features

- Add filtering option for multiselect (#85)

### Miscellaneous Tasks

- Release demand version 1.6.2

## [1.6.1] - 2025-01-07

### Bug Fixes

- Select with a specific option set as selected is not shown as selected in the output
- Update rust crate itertools to 0.14 (#79)

### Miscellaneous Tasks

- Release demand version 1.6.1
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

## [0.1.0] - 2023-12-21


