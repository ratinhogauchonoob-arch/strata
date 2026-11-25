# Changelog

## 0.10.1 Beta — 2026-11-25
- **Español y Português:** the interface speaks Spanish and Portuguese (Settings → Appearance → Language; Automatic follows Windows).
- **Update notice:** Strata checks its GitHub releases at start and shows “Update” at the top when a new version is out (Settings → Appearance to turn it off).
- **Process priority: Automatic** (new default) uses High on laptops and PCs with 16 GB of RAM or less, where the engine needs all the CPU it can get.
- **Resolution** now also sets SharpEmu's window size, not only KytyPS5's.
- Fixed: after a long session the Strata window could show “Out of Memory”. Engines print thousands of lines a second; now at most 60 a second reach the window (the session log still has every line) and the window no longer keeps an endless list.
- Fixed: engine log lines showed color codes as boxes and “[36m” in the Developer tab.
- **Why an engine stopped:** Strata now reads the engine's own fatal message (KytyPS5's “--- Fatal Error ---”, SharpEmu's “Process terminated.”, AnyPS5's “FATAL:”) and shows it in the game's Compatibility page and when a game closes. Exit codes are explained in words (321 = KytyPS5 fatal error, 0x80131623 = SharpEmu stopped itself, 0xC0000006 = the game files could not be read).
- “Share test results” also sends that message, with file paths and anything in quotes removed on your PC first.
- Fixed: “Made with” said **Decima** for Minecraft (the word “Decimal” in its program matched). Engine names now need a whole word; Minecraft shows **Bedrock**. Games are read again once after updating.
- Fixed: a session log could grow to several GB with “Capture engine log” on; it now stops at 256 MB (the game keeps running). Daily app logs older than 14 days are removed.
- Fixed: without the Discord app open, every game start and exit wrote a warning; now it warns once.
- Fixed: a game ID with an accented letter could crash the result sharing.
- Game names are never translated (a game called “Control” stays “Control”), and engine log lines stay as the engine wrote them.
- The “new version” pop-up shows once per version; the “Update” button stays at the top.
- Fixed: SharpEmu and AnyPS5 always opened in fullscreen even with **Fullscreen** off. Strata now tells every engine whether to open in a window or fullscreen (F11 still switches while playing).

## 0.10.0 Beta — 2026-11-21
- **Smart engine choice:** Strata reads each game's `eboot.bin` (system functions, libraries, which game engine made it) in the background. When an engine fails, Strata records what it was missing; other games that need the same functions start with a better engine. After the first launch, a game opens straight in the engine that worked. Everything is kept in plain files under `data/compat` that you can share.
- New per-game **Compatibility** page: what the game needs, a chance per engine, the last tests, and "Test engines again".
- **Community results (opt-in):** after a game closes Strata can send an anonymous result (game, engine, worked or not, what the engine was missing — no name, paths or logs) to strata.gbxstudios.com. Working games appear on the website's Compatibility page; Strata downloads everyone's results to start games with the engine that worked for other players. Asked once; Settings → Community to change.
- **Library filters:** Works / Untested / Failed and by engine; WORKS / FAILED tags on cards.
- **Stats:** time played, most played games, engine success rates, which game engines your games use (Library → Stats).
- **TV mode:** bigger interface for a TV and controller (F9 or Settings → Appearance).
- **Discord Rich Presence:** "Browsing the library" in the menus and the game you play on your Discord profile (on by default when the Discord desktop app is open; Settings → Community to turn it off).
- "Support Strata" section on the About page (shown once support links are set).
- **Three PS5 engines:** KytyPS5, SharpEmu and AnyPS5, each downloaded from its own GitHub releases. Setup has "Download all".
- **Automatic fallback:** when an engine closes or crashes within the first seconds, the next engine starts; every attempt is logged and the engine that worked is remembered per game. "Try next engine" while a game runs.
- **Per-game engine** choice (or Automatic), per-game controller keys and a per-game Developer page.
- **Keyboard and mouse keys** written for AnyPS5; swap Cross/Circle; stick dead zone.
- **Save backups** for one game or all games, shader caches optional, with restore.
- **Virtual refresh rate: Auto** follows the monitor Strata is on (e.g. 144 Hz or 280 Hz). Old 60 Hz settings move to Auto once.
- Fixed: the Settings tab kept showing a game's settings after opening them (needed Esc).
- Fixed: folders with an `eboot.bin` but no game info (tools, payloads) showed up as games. Developer mode lists them under "Skipped folders".
- Fixed: game settings now mark what differs from the global settings and have "Use global settings again".
- Fixed: "null" could appear in the running bar and on some settings pages.
- PS4 support (shadPS4, `.pkg` install) is built but switched off in this release while it is being tested.

## 0.9.0 Beta — 2026-11-21
First public beta.

- Fixed: a game that crashed instantly could leave the "running" banner stuck.
- Fixed: pressing Enter on locked per-game settings with a keyboard or controller could break the page.
- Fixed: focus jumped to the top bar after changing a setting.
- Fixed: the Play button could overlap the shelf with long titles or small windows.
- Automatic GPU now pins the dedicated card on PCs with two GPUs.
- Engine download uses the Windows certificate store (works behind antivirus HTTPS scanning).
- Custom cover, background and logo art per game.
