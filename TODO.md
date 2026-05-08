# TODO

## Icons And Assets

- Keep `Resources/Info.plist`; it is required for macOS bundle metadata.
- Keep `Resources/Settings.png` while the Preferences menu action uses it.

## v1.0.0 Release

- Create release packaging workflows for macOS ARM, Linux x64/ARM, and source archives using the names in `RELEASES.md`.
- Confirm macOS ARM packaging and dependency behavior on a clean Apple Silicon machine.
- Confirm Linux x64/ARM AppImage packaging in GitHub Actions.

## Post-v1.0.0 Modernization

- Continue isolating core extraction logic from Qt UI code.
- Expand the Rust `inviska-core` prototype with MKVToolNix discovery, command planning, and parser coverage.
- Decide whether the production app remains Qt/C++ during modernization or moves to a Rust UI stack after the core is stable.
- Add focused tests around MKVToolNix version parsing, tool discovery, track parsing, and extraction command generation.
- Add Windows x64 MSI packaging after the Rust port.
- Add Windows ARM packaging after the Rust port.
