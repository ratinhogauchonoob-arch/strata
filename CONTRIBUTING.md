# Contributing to Strata

Thanks for helping! Strata is in beta, so the priority is **fixing bugs before adding features**.

## Ground rules
- No games, firmware, BIOS files, keys or links to them — in code, issues or pull requests.
- UI text is US English and never uses console brand names or logos.
- Keep the GPL-2.0 notices and the KytyPS5 credits.

## Development
- Backend: `src-tauri/src` (Rust). Run `cargo test --lib` inside `src-tauri`.
- Frontend: `ui/` (plain ES modules, no build step). Serve the folder and open `index.html` to use the built-in demo data.
- UI smoke test: `python tests/ui_smoke.py` (needs Playwright and Chromium).
- The engine is driven only through its command line; `engine.rs::build_args` mirrors the official KytyPS5 launcher.

## Reporting bugs
Use the website form (https://strata.gbxstudios.com/report) or the GitHub issue template, with the diagnostics file from *Developer → Export diagnostics*.
