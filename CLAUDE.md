# Strata (Tauri 2 launcher for the KytyPS5 engine)

- Backend: src-tauri/src (Rust). Frontend: ui/ (plain ES modules, no build step).
- Engine is driven only via its CLI; engine.rs::build_args mirrors KytyPS5's official launcher.
- GPU index = Vulkan enumeration order (ash), same as the engine's --gpu.
- RAM limit: memcap.rs (Windows FFI; soft = working-set cap, strict = job object).
- Tests: `cd src-tauri && cargo test --lib`. UI preview without the app: serve ui/ and open index.html (api.js mocks).
- Build for Windows: push to main -> .github/workflows/build.yml (artifact Strata-Windows).
- All UI text in US English. Never mention console brands in the UI. Keep GPL notices.
