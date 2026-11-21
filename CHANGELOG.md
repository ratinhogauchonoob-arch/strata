# Changelog

## 0.11.0 Beta — 2026-11-21
- **Install PS4 .pkg files** (games, updates, DLC) dumped from your own console. Strata reads the package, shows what it is and where it goes, and unpacks it with progress: games into your games folder, updates into `<game>-UPDATE`, DLC into shadPS4's `addcont`. Store/retail packages are refused with a clear message.
- Loose `.pkg` files in your library folders show up in Library with an Install button.
- New per-game page **Update, DLC & mods** (PS4): installed update and its version, DLC list, a mods folder you can switch on/off, and a patch file (XML) for shadPS4.
- Update, mods and unfinished install folders no longer appear as separate games.
- PS4 tag on library cards; the performance overlay setting explains the F10 key for shadPS4 and AnyPS5.

## 0.10.0 Beta — 2026-11-21
- **Four engines.** PS5: KytyPS5, SharpEmu, AnyPS5. PS4: shadPS4. Each one downloads from its own GitHub releases.
- **PS4 games** are recognized by their `param.sfo` and start with the PS4 engine automatically. If it is missing, Strata offers the download.
- **Automatic fallback:** when an engine closes or crashes within the first seconds, the next engine starts; every attempt is logged and the engine that worked is remembered per game. "Try next engine" while a game runs.
- **Per-game engine** choice (or Automatic), per-game controller keys and a per-game Developer page.
- **Keyboard and mouse keys** written for shadPS4 and AnyPS5; swap Cross/Circle; stick dead zone.
- **Save backups** for one game or all games, shader caches optional, with restore.
- **Virtual refresh rate: Auto** follows the monitor Strata is on (e.g. 144 Hz or 280 Hz). Old 60 Hz settings move to Auto once.
- Setup now has "Download all" for PS5 engines and an optional PS4 step.
- Fixed: the Settings tab kept showing a game's settings after opening them (needed Esc).
- Fixed: folders with an `eboot.bin` but no game info (tools, payloads) showed up as games.
- Fixed: game settings now mark what differs from the global settings and have "Use global settings again".

## 0.9.0 Beta — 2026-11-21
First public beta.

- Fixed: a game that crashed instantly could leave the "running" banner stuck.
- Fixed: pressing Enter on locked per-game settings with a keyboard or controller could break the page.
- Fixed: focus jumped to the top bar after changing a setting.
- Fixed: the Play button could overlap the shelf with long titles or small windows.
- Automatic GPU now pins the dedicated card on PCs with two GPUs.
- Engine download uses the Windows certificate store (works behind antivirus HTTPS scanning).
- Custom cover, background and logo art per game.
