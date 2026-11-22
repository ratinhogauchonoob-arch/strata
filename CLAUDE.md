# Strata (Tauri 2 launcher: KytyPS5, SharpEmu, AnyPS5 for PS5 · shadPS4 for PS4)

- Backend: src-tauri/src (Rust). Frontend: ui/ (plain ES modules, no build step).
- Engine is driven only via its CLI; engine.rs::build_args mirrors KytyPS5's official launcher.
- GPU index = Vulkan enumeration order (ash), same as the engine's --gpu.
- RAM limit: memcap.rs (Windows FFI; soft = working-set cap, strict = job object).
- Tests: `cd src-tauri && cargo test --lib`. UI preview without the app: serve ui/ and open index.html (api.js mocks).
- Build for Windows: push to main -> .github/workflows/build.yml (artifact Strata-Windows).
- All UI text in US English. Never mention console brands in the UI. Keep GPL notices.

## 0.10 multi-engine notes
- engines.rs is the registry (ENGINES), chain() decides order (per-game forced > last good > settings order), attempt_failed() decides fallback.
- lib.rs start_attempt()/on_engine_exit(): fallback runs from the exit thread; running slot is filled while holding its lock.
- Engine data: data/engines/<id>/bin (release, replaced on update) and data/engines/<id>/user (saves). KytyPS5 stays in data/engine.
- shadPS4 runs with cwd = data/engines/shadps4 (its user dir = cwd/user). SharpEmu gets SHARPEMU_USER_DIR. AnyPS5 converts once into data/engines/anyps5/apps/<TITLEID>/.
- Games need eboot.bin + sce_sys/param.json (PS5) or param.sfo (PS4).
- vblank_frequency 0 = Auto (monitor of Strata's window via EnumDisplaySettingsW).

## 0.11 PS4 packages
- pkginstall.rs uses orbis-pkg-util (MIT/Apache, choose MIT). Only fake PKGs open; retail → friendly error from OpenError.
- Destinations follow shadPS4: game → <library>/<TITLEID>, update → <game>-UPDATE, DLC → data/engines/shadps4/user/addcont/<TITLEID>/<label>. Extract to <dest>.part then rename.
- games::is_companion_dir skips -UPDATE/-patch/-mods/-mods.off/.part folders; scan_full also returns loose .pkg files.
- Mods = shadPS4's <game>-mods overlay (off = renamed to -mods.off). Patch file → shadPS4 --patch.

## Release switch
- lib.rs PS4_ENABLED = false for 0.10.0 (PS5 only). Full PS4 tree saved as Strata-0.11-ps4.zip. UI reads bootstrap.ps4; mock: index.html?ps4.

## Compatibility (compat.rs)
- analyze(): scans eboot.bin bytes for NID strings "XXXXXXXXXXX#l#m", lib names, engine strings; cached in data/compat/<ID>.json (fresh = same size+mtime).
- engine.rs feeds every log line to compat::scan_line → Findings in ExitInfo; on_engine_exit records Attempt and learns missing NIDs per engine release (engines.json).
- launch_game sorts the chain by compat::score (worked=100 > high > medium > low > failed); forced engine keeps order; fallback-off truncates after sorting.
- SUPPORT links in app.js (empty = hidden). open_url only allows ko-fi/github/youtube/paypal/strata site.

## Community / Discord / UI extras
- community.rs: Report built from Attempt (title id must match AAAA00000, release must be a plain folder name); send() and refresh() run on threads; community.json cached 12 h; merged into scoring (others>0 → "high").
- Site API: POST /api/compat/report (throttle 60/h), GET /api/compat/knowledge. Public /compatibility shows worked only; /admin/compatibility shows errors.
- discord.rs APP_ID empty = off. Settings: community.{share,use_results,asked,discord}, ui.tv_mode (html.tv zoom 1.32, F9).
