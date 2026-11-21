"""UI regression run against the browser mock (ui/js/api.js).
Usage: serve ui/ on :8765, then  python tests/ui_smoke.py  [chrome path]
Fails on any page error or console error."""
import asyncio, sys
from playwright.async_api import async_playwright

CHROME = sys.argv[1] if len(sys.argv) > 1 else None

async def main():
    problems = []
    async with async_playwright() as p:
        b = await p.chromium.launch(executable_path=CHROME, args=["--no-sandbox"]) if CHROME else await p.chromium.launch()
        for vw, vh in [(1366, 800), (1024, 640), (1920, 1080)]:
            pg = await b.new_page(viewport={"width": vw, "height": vh})
            pg.on("pageerror", lambda e: problems.append(f"[{vw}x{vh}] page error: {e}"))
            pg.on("console", lambda m: problems.append(f"[{vw}x{vh}] console {m.type}: {m.text}") if m.type == "error" and "favicon" not in m.text else None)
            await pg.goto("http://localhost:8765/index.html")
            await pg.wait_for_timeout(3600)
            step = lambda n: print(f"  {vw}x{vh} · {n}")
            # wizard
            step("wizard"); await pg.click("#wizard >> text=Apply"); await pg.wait_for_timeout(300)
            await pg.click("#wizard >> text=Start"); await pg.wait_for_timeout(400)
            # home keyboard
            step("home keys")
            for k in ["ArrowRight", "ArrowRight", "ArrowLeft", "ArrowUp", "ArrowDown", "f", "i", "Escape", "s", "Escape", "q", "e", "e", "e", "e"]:
                await pg.keyboard.press(k); await pg.wait_for_timeout(120)
            await pg.click("#tab-home"); await pg.wait_for_timeout(300)
            # overlap check: Play button must sit above the shelf
            play = await pg.locator("#play").bounding_box()
            shelf = await pg.locator(".shelf-wrap").bounding_box()
            if play and shelf and play["y"] + play["height"] > shelf["y"] + 2:
                problems.append(f"[{vw}x{vh}] Play button overlaps the shelf")
            # run + stop
            step("run/stop"); await pg.click("#play"); await pg.wait_for_timeout(800)
            await pg.click("#running >> text=Stop"); await pg.wait_for_timeout(600)
            if await pg.locator("#running").is_visible():
                problems.append(f"[{vw}x{vh}] running banner still visible after stop")
            # library
            step("library"); await pg.click("#tab-library"); await pg.wait_for_timeout(300)
            await pg.fill("#search", "lumen"); await pg.wait_for_timeout(200)
            if await pg.locator(".card").count() != 1: problems.append(f"[{vw}x{vh}] search filter wrong")
            await pg.fill("#search", ""); await pg.click(".seg button:text-is(\"List\")"); await pg.wait_for_timeout(200)
            await pg.click(".seg button:text-is(\"Grid\")"); await pg.click(".seg button:text-is(\"Favorites\")"); await pg.click(".seg button:text-is(\"All\")")
            await pg.locator(".card").first.focus(); await pg.keyboard.press("f"); await pg.wait_for_timeout(200)
            await pg.locator(".card").first.click(); await pg.wait_for_timeout(400)
            await pg.click("#drawer >> text=Playable"); await pg.fill("#drawer textarea", "runs fine"); await pg.keyboard.press("Escape"); await pg.keyboard.press("Escape"); await pg.wait_for_timeout(400)
            # per-game settings, locked: keyboard Enter on controls must not crash
            step("per-game locked"); await pg.locator(".card").nth(1).click(); await pg.wait_for_timeout(300)
            await pg.click("#drawer >> text=Game settings"); await pg.wait_for_timeout(400)
            # locked fields: force focus + Enter on each control; the guard must ignore it
            for c in (await pg.locator("#pane .field.disabled .ctl [data-nav]").all())[:5]:
                await c.focus(); await pg.keyboard.press("Enter"); await pg.wait_for_timeout(60)
            if await pg.locator("#pane .banner.accent").count():
                problems.append(f"[{vw}x{vh}] locked per-game settings changed state")
            if not await pg.locator("#pane .banner .switch").count():  # Enter may have hit "Back to global settings"
                await pg.click("#tab-library"); await pg.wait_for_timeout(300)
                await pg.locator(".card").nth(1).click(); await pg.wait_for_timeout(300)
                await pg.click("#drawer >> text=Game settings"); await pg.wait_for_timeout(400)
            if not await pg.locator("#pane .banner.accent").count():
                await pg.click("#pane .banner .switch"); await pg.wait_for_timeout(200)  # custom on
            await pg.click('.sections button >> text=\"Performance\"'); await pg.wait_for_timeout(200)
            await pg.locator("#pane .switch").nth(1).click(); await pg.wait_for_timeout(200)  # RAM limit on (per game)
            await pg.click("text=Back to global settings"); await pg.wait_for_timeout(300)
            # every settings section
            step("settings sections")
            for name in ["This PC", "Display", "Rendering", "Performance", "System", "Controller", "Library", "Engines", "Saves & backups", "Appearance", "Developer", "About"]:
                await pg.click(f'.sections button >> text="{name}"'); await pg.wait_for_timeout(150)
                for sw in (await pg.locator("#pane .field:not(.disabled) .stepper button").all())[:2]:
                    await sw.click()
            await pg.click('.sections button >> text=\"Engines\"'); await pg.locator("#pane >> text=Update").first.click(); await pg.wait_for_timeout(1700)
            await pg.click('.sections button >> text=\"Appearance\"'); await pg.locator(".sw").nth(2).click(); await pg.click("#pane >> text=Play"); await pg.wait_for_timeout(500); await pg.keyboard.press("Escape"); await pg.wait_for_timeout(700)
            # developer
            step("developer"); await pg.click("#tab-dev"); await pg.wait_for_timeout(300)
            await pg.click("text=Run self-test"); await pg.wait_for_timeout(500)
            for f in ["info", "warn", "debug"]:
                await pg.click(f".filters >> text={f}"); await pg.click(f".filters >> text={f}")
            await pg.click("text=Export diagnostics"); await pg.wait_for_timeout(300)
            # bug fix: the Settings tab always opens the global settings
            step("settings tab resets game settings")
            await pg.click("#tab-home"); await pg.wait_for_timeout(300)
            await pg.keyboard.press("s"); await pg.wait_for_timeout(300)
            if "game settings" not in (await pg.locator("#view-settings .page-head").inner_text()).lower():
                problems.append(f"[{vw}x{vh}] S did not open game settings")
            await pg.click("#tab-settings"); await pg.wait_for_timeout(300)
            if "game settings" in (await pg.locator("#view-settings .page-head").inner_text()).lower():
                problems.append(f"[{vw}x{vh}] Settings tab still shows game settings")
            # controls + backups
            step("controls/backups")
            await pg.click('.sections button >> text="Controller"'); await pg.wait_for_timeout(300)
            await pg.locator("#pane .switch").first.click(); await pg.wait_for_timeout(200)
            await pg.locator("#pane .keymap select").first.select_option("kp2"); await pg.wait_for_timeout(200)
            await pg.click('.sections button >> text="Saves & backups"'); await pg.wait_for_timeout(300)
            await pg.click("#pane >> text=Back up now"); await pg.wait_for_timeout(400)
            if not await pg.locator("#pane >> text=Restore").count():
                problems.append(f"[{vw}x{vh}] backup not listed")
            # PS4 game without a PS4 engine: friendly prompt, download, then it starts
            step("ps4 prompt")
            await pg.click("#tab-library"); await pg.wait_for_timeout(300)
            await pg.locator(".card", has_text="Tidebreaker").click(); await pg.wait_for_timeout(300)
            await pg.click("#drawer >> text=Play"); await pg.wait_for_timeout(500)
            if not await pg.locator(".dialog >> text=This is a PS4 game").count() and vw == 1366:
                problems.append(f"[{vw}x{vh}] no PS4 engine prompt")
            if await pg.locator(".dialog").count():
                await pg.click(".dialog .btn.primary"); await pg.wait_for_timeout(2200)
            if not await pg.locator("#running").is_visible():
                problems.append(f"[{vw}x{vh}] PS4 game did not start after the download")
            else:
                await pg.click("#running >> text=Stop"); await pg.wait_for_timeout(500)
            # 0.11: install a PS4 .pkg found in the library folders, then update/DLC/mods page
            step("pkg install")
            await pg.click("#tab-library"); await pg.wait_for_timeout(300)
            if await pg.locator("#pkgbar >> text=Install").count():
                await pg.click("#pkgbar .btn"); await pg.wait_for_timeout(400)
                await pg.click(".dialog .btn.primary"); await pg.wait_for_timeout(1800)
                if await pg.locator("#pkgbar").count():
                    problems.append(f"[{vw}x{vh}] pkg banner still shown after install")
            step("extras")
            await pg.locator(".card", has_text="Tidebreaker").click(); await pg.wait_for_timeout(300)
            await pg.click("#drawer >> text=Game settings"); await pg.wait_for_timeout(300)
            await pg.click('.sections button >> text="Update, DLC & mods"'); await pg.wait_for_timeout(500)
            await pg.locator("#pane .switch").first.click(); await pg.wait_for_timeout(400)
            if not await pg.locator("#pane .switch.on").count():
                problems.append(f"[{vw}x{vh}] mods switch did not turn on")
            await pg.click("#pane >> text=Choose file"); await pg.wait_for_timeout(300)
            if not await pg.locator("#pane >> text=60fps.xml").count():
                problems.append(f"[{vw}x{vh}] patch file not shown")
            await pg.screenshot(path=f"/tmp/smoke_{vw}.png")
            await pg.close()
        await b.close()
    if problems:
        print("FAILED:\n" + "\n".join(dict.fromkeys(problems)))
        sys.exit(1)
    print("UI smoke: all good")

asyncio.run(main())
