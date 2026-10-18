# Setup

## Requirements

- Node.js 26 or newer, with npm.
- Rust (stable, MSVC toolchain) and the [Tauri v2 prerequisites](https://v2.tauri.app/start/prerequisites/) for Windows, which include WebView2.

## Commands

| Command                                   | What it does                                                                                |
| ----------------------------------------- | ------------------------------------------------------------------------------------------- |
| `npm install`                             | Installs the UI and Tauri CLI dependencies.                                                 |
| `npm run tauri dev`                       | Runs the app in a development window. Vite serves the UI on port 5174 with hot reload.      |
| `npm run tauri build`                     | Release build, see [releasing.md](releasing.md).                                            |
| `npm run build`                           | Builds the UI into `dist/`, the step the Tauri build runs first.                            |
| `npm run typecheck`                       | `tsc` over the app and the scripts.                                                         |
| `npm run format` / `npm run format:check` | Prettier over the repo. `format:check` is what CI runs.                                     |
| `npm run knip`                            | Finds unused files, exports and dependencies.                                               |
| `npm run extract`                         | Rebuilds `src/data/halloween-maps.json` and `public/maps/`. See [map-list.md](map-list.md). |

In `src-tauri`, run `cargo test`, `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check`. CI runs the same.

Environment variables: `TF2_DIR` overrides where TF2 is, if the lookup through Steam's registry key and `libraryfolders.vdf` gets it wrong.

Dev builds optimise dependencies (the `[profile.dev]` sections of `src-tauri/Cargo.toml`), so the overlay draws smoothly under `tauri dev`.

## Testing safely

The app writes into the TF2 folder and puts the original casual queue back when it quits, so do not point a test build at your real install while the release build is running. Make a fake game folder with a `tf/console.log` and a `tf/casual_criteria.vdf`, set `TF2_DIR` to it, and run `npm run tauri dev`. Append lines to the fake `console.log` to simulate queueing, joining a match, and map changes.

The app is single instance, so close the release build first. Otherwise the dev start only shows the open window.

## Style

- TypeScript: two-space indent, single quotes, 120 columns, enforced by Prettier (`.prettierrc`).
- Rust: formatted with `cargo fmt`, linted with clippy.
- Comments explain intent or a non-obvious constraint, not what the code already says.
