# Releasing

## Building

`npm run tauri build` writes the exe to `src-tauri/target/release/tf2-halloween-utils.exe`. There is no installer; the exe is the release. The result is unsigned, so Windows shows a SmartScreen prompt on first run.

## Releasing

Bump the version in `package.json`, `src-tauri/Cargo.toml` and `src-tauri/tauri.conf.json`, then push a tag like `v2.0.0`. The release workflow runs `npm run tauri build` on Windows and attaches `tf2-halloween-utils.exe` to a GitHub release for the tag. Releases are Windows only for now; CI only checks that the Linux side still compiles.
