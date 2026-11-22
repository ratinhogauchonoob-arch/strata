// Settings definitions. Each field: key path, control type, label, help, impact tag.
// Tags: "fps-up" (faster), "fps-down" (costs speed), "fix" (fixes some games), "risk" (may crash), "debug".

const RES = ["1280x720", "1600x900", "1920x1080", "2560x1440", "3840x2160"];
const LANGS = [
  [0, "Japanese"], [1, "English (US)"], [2, "French"], [3, "Spanish (Spain)"], [4, "German"], [5, "Italian"],
  [6, "Dutch"], [7, "Portuguese (Portugal)"], [8, "Russian"], [9, "Korean"], [10, "Chinese (Traditional)"],
  [11, "Chinese (Simplified)"], [12, "Finnish"], [13, "Swedish"], [14, "Danish"], [15, "Norwegian"], [16, "Polish"],
  [17, "Portuguese (Brazil)"], [18, "English (UK)"], [19, "Turkish"], [20, "Spanish (Latin America)"], [21, "Arabic"],
  [22, "French (Canada)"], [23, "Czech"], [24, "Hungarian"], [25, "Greek"], [26, "Romanian"], [27, "Thai"],
  [28, "Vietnamese"], [29, "Indonesian"],
];

export const SECTIONS = [
  { id: "profile", group: "Setup", title: "This PC", lead: "Strata reads your hardware and picks settings that fit. You can re-run it any time; everything stays editable.", custom: "profile", perGame: false },
  {
    id: "display", group: "Engine", title: "Display", perGame: true,
    lead: "Window size and how frames reach the screen.",
    fields: [
      { key: "emu.resolution", type: "select", label: "Resolution", options: RES, tag: "fps-down", help: "Higher sizes look sharper but cost the most performance. 1280×720 is the safest." },
      { key: "emu.present_mode", type: "select", label: "Presentation", options: [["Mailbox", "Mailbox — smooth, no tearing"], ["Immediate", "Immediate — fastest, may tear"], ["Fifo", "Fifo — locked to refresh rate"]], help: "Immediate gives the most frames; Fifo adds input delay." },
      { key: "emu.gpu_index", type: "gpu", label: "Graphics card", help: "On laptops pick the dedicated GPU, never the integrated one." },
      { key: "emu.fullscreen", type: "toggle", label: "Fullscreen", help: "Borderless fullscreen on the current monitor." },
      { key: "emu.stretch", type: "toggle", label: "Stretch to window", help: "Fill the window instead of keeping the aspect ratio." },
      { key: "emu.hdr", type: "toggle", label: "HDR output", help: "Only for HDR monitors with HDR enabled in Windows." },
      { key: "emu.hide_cursor", type: "toggle", label: "Hide mouse cursor", help: "Hides the cursor after 2 seconds without movement." },
      { key: "emu.vblank_frequency", type: "refresh", label: "Virtual refresh rate", tag: "risk", help: "Auto matches the monitor Strata is on (144 Hz, 280 Hz…). If a game runs too fast, set this game to 60 Hz." },
      { key: "emu.perf_overlay", type: "toggle", label: "Performance overlay", help: "KytyPS5: FPS, frame time and memory (F2 toggles it). In shadPS4 and AnyPS5 press F10 while playing." },
    ],
  },
  {
    id: "rendering", group: "Engine", title: "Rendering", perGame: true,
    lead: "Speed versus accuracy. Leave fixes off unless a game shows black screens or missing textures.",
    fields: [
      { key: "emu.shader_cache", type: "toggle", label: "Shader cache", tag: "fps-up", help: "Saves compiled shaders, so the second run stutters much less. Keep on." },
      { key: "emu.relaxed_shader_float", type: "toggle", label: "Relaxed shader math", tag: "fps-up", help: "Faster on weak GPUs; colors or shadows can be slightly wrong." },
      { key: "emu.flush_draws", type: "number", label: "Submit GPU work every N draws", min: 0, max: 5000, step: 50, tag: "fps-up", help: "Can lower frame time in some games. 0 = off. Watch the overlay; set back to 0 if it gets worse." },
      { key: "emu.readback_linear", type: "toggle", label: "Readback (linear images)", tag: "fix", help: "Fixes black screens in some games. Costs speed." },
      { key: "emu.readback_tiled", type: "toggle", label: "Readback (tiled images)", tag: "fix", help: "Next step if linear readback did not help." },
      { key: "emu.readback_storage", type: "toggle", label: "Readback (shader-written images)", tag: "fix", help: "Last readback step; the heaviest." },
      { key: "emu.sync_raw_image_buffers", type: "toggle", label: "Accurate texture sync", tag: "fix", help: "Fixes missing textures and wrong transparency. Costs speed." },
      { key: "emu.occlusion_queries", type: "toggle", label: "Occlusion queries", tag: "fps-down", help: "Keeps hidden lights and objects hidden. Costs speed." },
      { key: "emu.tessellation", type: "toggle", label: "Tessellation", tag: "risk", help: "Experimental. Can crash games that use it." },
      { key: "emu.native_rt", type: "toggle", label: "Native ray tracing", tag: "risk", help: "Experimental. Needs an RT-capable GPU." },
      { key: "emu.pro_mode", type: "toggle", label: "Enhanced console mode", tag: "risk", help: "Experimental mode for games that support it. Required for upscaling." },
      { key: "emu.upscaler", type: "select", label: "Upscaler", options: [["Linear", "Linear"], ["DLSS", "DLSS (NVIDIA RTX)"], ["FSR", "FSR (any GPU)"]], tag: "fps-up", requires: "emu.pro_mode", help: "Renders smaller and scales up. DLSS and FSR need their files in a folder." },
      { key: "emu.dlss_folder", type: "folder", label: "DLSS files folder", requires: "emu.pro_mode", help: "Folder with nvngx_dlss.dll." },
      { key: "emu.fsr_folder", type: "folder", label: "FSR files folder", requires: "emu.pro_mode", help: "Folder with the AMD FidelityFX loader and upscaler DLLs (SDK 2.4+)." },
    ],
  },
  {
    id: "performance", group: "Engine", title: "Performance", perGame: true, custom: "ram",
    lead: "Memory limit, process priority and CPU compatibility.",
    fields: [
      { key: "perf.priority", type: "select", label: "Process priority", options: [["normal", "Normal"], ["above_normal", "Above normal"], ["high", "High"]], tag: "fps-up", help: "Higher priority can smooth frame times when other apps are open." },
      { key: "emu.amd_cpu_compat", type: "toggle", label: "CPU compatibility mode", tag: "fix", help: "Turn on with Intel CPUs if a game closes right after starting." },
      { key: "emu.guest_tsc", type: "toggle", label: "Console timer rate", tag: "fix", help: "Fixes games that run too fast. Turn off if a game becomes slow." },
      { key: "emu.red_zone", type: "toggle", label: "Stack red-zone protection", tag: "fix", help: "Prevents some crashes on Windows. Turn on only for games that crash." },
    ],
  },
  {
    id: "system", group: "Engine", title: "System", perGame: true,
    lead: "What games see as the console user, language and network.",
    fields: [
      { key: "emu.user_name", type: "text", label: "User name", help: "Name shown inside games." },
      { key: "emu.user_id", type: "number", label: "User ID", min: 1, max: 99999, step: 1, help: "Changing it gives games a fresh save profile." },
      { key: "emu.console_language", type: "select", label: "Console language", options: LANGS, help: "Games use this to pick their language." },
      { key: "emu.skip_notice_screen", type: "toggle", label: "Skip startup logos", help: "Skips logos and notices in games that allow it." },
      { key: "emu.trophies", type: "toggle", label: "Achievement pop-ups", help: "Shows a notification when you unlock something." },
      { key: "emu.trophy_duration", type: "number", label: "Pop-up duration", min: 1, max: 30, step: 1, suffix: "s", requires: "emu.trophies", help: "Seconds each notification stays on screen." },
      { key: "emu.network", type: "toggle", label: "Network access", tag: "risk", help: "Lets games use your local network. Some games hang with it on." },
      { key: "emu.emulate_online", type: "toggle", label: "Emulate online services", tag: "risk", help: "Compatibility shim for games that check online status." },
    ],
  },
  {
    id: "controller", group: "Engine", title: "Controller", perGame: true, custom: "controls",
    lead: "Gamepads work out of the box. Keyboard and mouse keys can be changed here.",
    fields: [
      { key: "emu.keyboard_player", type: "select", label: "Keyboard player", options: [[0, "Shares player 1"], [1, "Player 1"], [2, "Player 2"], [3, "Player 3"], [4, "Player 4"]], help: "Gamepads fill the other players in connection order." },
      { key: "emu.vibration", type: "range", label: "Vibration", min: 0, max: 100, step: 5, suffix: "%", help: "Scales controller rumble." },
      { key: "emu.speaker_volume", type: "range", label: "Controller speaker", min: 0, max: 100, step: 5, suffix: "%", help: "50% is the normal level." },
      { key: "emu.controller_color", type: "color", label: "Light bar color", help: "Empty = the game decides." },
    ],
  },
  { id: "library", group: "Launcher", title: "Library", custom: "library", perGame: false, lead: "Folders Strata watches for games. Each game is a folder with eboot.bin inside, or a .zar archive." },
  { id: "engine", group: "Launcher", title: "Engines", custom: "engine", perGame: true, lead: "Strata runs games through open-source engines (all GPL-2.0), downloaded from their official GitHub releases. If one engine cannot run a game, Strata tries the next." },
  { id: "community", group: "Launcher", title: "Community", custom: "community", perGame: false, lead: "Share results with other players and show what you play. Everything here is off unless you turn it on." },
  { id: "backups", group: "Launcher", title: "Saves & backups", custom: "backups", perGame: true, lead: "Copies game saves (and, if you want, shader caches) into one file you can put back later." },
  {
    id: "appearance", group: "Launcher", title: "Appearance", custom: "appearance", perGame: false,
    lead: "Colors, boot animation and motion.",
    fields: [
      { key: "ui.boot_style", type: "select", label: "Boot animation", options: [["duo", "Duo — your two accents"], ["spectrum", "Spectrum — all six colors"], ["off", "Off"]], help: "Shown every time Strata starts. Any key skips it." },
      { key: "ui.reduce_motion", type: "toggle", label: "Reduce motion", help: "Shorter, calmer animations." },
      { key: "ui.ui_sounds", type: "toggle", label: "Interface sounds", help: "Soft clicks when moving around." },
      { key: "ui.clock_24h", type: "toggle", label: "24-hour clock" },
      { key: "ui.tv_mode", type: "toggle", label: "TV mode", help: "Bigger text and controls for playing on a TV with a controller. F9 turns it on and off." },
    ],
  },
  {
    id: "developer", group: "Launcher", title: "Developer", perGame: false,
    lead: "Logs, diagnostics and engine debug switches. Debug options slow games down a lot.",
    fields: [
      { key: "dev.enabled", type: "toggle", label: "Developer mode", help: "Adds the Developer tab with live logs, self-tests and diagnostics." },
      { key: "dev.log_level", type: "select", label: "Log detail", options: [["info", "Normal"], ["debug", "Debug"], ["trace", "Everything"]], help: "More detail helps when reporting a problem." },
      { key: "dev.capture_engine_log", type: "toggle", label: "Capture engine log", help: "Shows the engine's own output in the console and saves it per session." },
      { key: "dev.keep_sessions", type: "number", label: "Session logs to keep", min: 1, max: 200, step: 1, help: "Older session logs are deleted automatically." },
      { key: "dev.show_command_line", type: "toggle", label: "Show engine command line", help: "Displays the exact command used to start a game." },
      { key: "emu.vulkan_validation", type: "toggle", label: "Vulkan validation", tag: "debug", help: "Needs the Vulkan SDK. Very slow." },
      { key: "emu.shader_validation", type: "toggle", label: "Shader validation", tag: "debug", help: "Validates generated SPIR-V." },
      { key: "emu.shader_log", type: "select", label: "Shader log", options: [["Silent", "Off"], ["Console", "Console"], ["File", "File (logs/shaders)"]], tag: "debug", help: "Dumps shaders. Slow." },
      { key: "emu.command_buffer_dump", type: "toggle", label: "Command buffer dump", tag: "debug", help: "Writes GPU command buffers to logs/buffers." },
      { key: "emu.profiler", type: "toggle", label: "Tracy profiler", tag: "debug", help: "Connect the Tracy app to inspect frames." },
      { key: "emu.renderdoc", type: "toggle", label: "RenderDoc capture", tag: "debug", help: "Allows frame captures with RenderDoc." },
    ],
  },
  { id: "compat", group: "Launcher", title: "Compatibility", custom: "compat", perGame: "only", lead: "What this game needs and how likely each engine is to run it." },
  { id: "extras", group: "Launcher", title: "Update, DLC & mods", custom: "extras", perGame: "only", ps4Only: true, lead: "Extra content installed next to this game." },
  { id: "gamedev", group: "Launcher", title: "Developer", custom: "gamedev", perGame: "only", devOnly: true, lead: "Logs and details for this game." },
  { id: "about", group: "Launcher", title: "About", custom: "about", perGame: false, lead: "" },
];

export const TAG_TEXT = { "fps-up": "+ FPS", "fps-down": "− FPS", fix: "Fix", risk: "Experimental", debug: "Debug" };

export const ACCENTS = ["ember", "cobalt", "signal", "fog", "slate"];
