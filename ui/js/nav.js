// One focus system for mouse, keyboard and gamepad.
// Elements with [data-nav] are reachable; arrows/D-pad move to the nearest one in that direction.

const handlers = { back: [], menu: [], tabPrev: [], tabNext: [], action: [] };
export const on = (name, fn) => handlers[name].push(fn);
const fire = (name, e) => handlers[name].some((fn) => fn(e) === true);

let sounds = true;
let ctx = null;
export function setSounds(v) { sounds = v; }
export function blip(kind = "move") {
  if (!sounds) return;
  try {
    ctx = ctx || new AudioContext();
    const o = ctx.createOscillator();
    const g = ctx.createGain();
    const f = { move: 880, ok: 1320, back: 520, err: 220 }[kind] || 880;
    o.type = "sine";
    o.frequency.setValueAtTime(f, ctx.currentTime);
    o.frequency.exponentialRampToValueAtTime(f * (kind === "back" ? 0.7 : 1.15), ctx.currentTime + 0.06);
    g.gain.setValueAtTime(0.0001, ctx.currentTime);
    g.gain.exponentialRampToValueAtTime(kind === "move" ? 0.018 : 0.03, ctx.currentTime + 0.005);
    g.gain.exponentialRampToValueAtTime(0.0001, ctx.currentTime + 0.09);
    o.connect(g).connect(ctx.destination);
    o.start();
    o.stop(ctx.currentTime + 0.1);
  } catch { /* audio not available */ }
}

let scope = null; // limit navigation to a container (drawer / modal)
export function setScope(el) { scope = el; }

function visible(el) {
  const r = el.getBoundingClientRect();
  if (r.width === 0 || r.height === 0) return false;
  const st = getComputedStyle(el);
  if (st.visibility === "hidden") return false;
  let p = el;
  while (p) {
    if (p.classList && (p.classList.contains("hidden"))) return false;
    if (p.classList && p.classList.contains("view") && !p.classList.contains("active")) return false;
    p = p.parentElement;
  }
  return true;
}

export function candidates() {
  const root = scope || document;
  return [...root.querySelectorAll("[data-nav]")].filter((el) => !el.disabled && visible(el));
}

export function focus(el, { silent = false } = {}) {
  if (!el) return;
  document.querySelectorAll(".is-focus").forEach((x) => x.classList.remove("is-focus"));
  el.classList.add("is-focus");
  el.focus({ preventScroll: true });
  el.scrollIntoView({ block: "nearest", inline: "nearest", behavior: document.body.classList.contains("reduce-motion") ? "auto" : "smooth" });
  if (!silent) blip("move");
  el.dispatchEvent(new CustomEvent("navfocus", { bubbles: true }));
}

export function current() {
  const a = document.activeElement;
  return a && a.matches && a.matches("[data-nav]") ? a : null;
}

export function move(dir) {
  const list = candidates();
  const cur = current();
  if (!cur || !list.includes(cur)) {
    focus(list[0]);
    return;
  }
  // Rows of a shelf/grid: prefer the element explicitly marked as the next one.
  const explicit = cur.dataset["nav" + dir[0].toUpperCase() + dir.slice(1)];
  if (explicit) {
    const t = document.querySelector(explicit);
    if (t && visible(t)) return focus(t);
  }
  const c = cur.getBoundingClientRect();
  const cx = c.left + c.width / 2, cy = c.top + c.height / 2;
  let best = null, bestScore = Infinity;
  for (const el of list) {
    if (el === cur) continue;
    const r = el.getBoundingClientRect();
    const x = r.left + r.width / 2, y = r.top + r.height / 2;
    const dx = x - cx, dy = y - cy;
    let main, cross;
    if (dir === "left") { if (r.right > c.left + 1) continue; main = -dx; cross = Math.abs(dy); }
    else if (dir === "right") { if (r.left < c.right - 1) continue; main = dx; cross = Math.abs(dy); }
    else if (dir === "up") { if (r.bottom > c.top + 1) continue; main = -dy; cross = Math.abs(dx); }
    else { if (r.top < c.bottom - 1) continue; main = dy; cross = Math.abs(dx); }
    const score = main + cross * 2.2;
    if (score < bestScore) { bestScore = score; best = el; }
  }
  if (best) focus(best);
}

function activate() {
  const cur = current();
  if (!cur) return;
  blip("ok");
  if (cur.tagName === "SELECT") {
    // cycle options with A since native dropdowns don't open from a gamepad
    cur.selectedIndex = (cur.selectedIndex + 1) % cur.options.length;
    cur.dispatchEvent(new Event("change", { bubbles: true }));
  } else if (cur.tagName === "INPUT" || cur.tagName === "TEXTAREA") {
    cur.focus();
  } else {
    cur.click();
  }
}

function typing() {
  const a = document.activeElement;
  return a && (a.tagName === "INPUT" || a.tagName === "TEXTAREA");
}

document.addEventListener("keydown", (e) => {
  const k = e.key;
  if (typing()) {
    if (k === "Escape") { document.activeElement.blur(); e.preventDefault(); }
    if (k === "Enter" && document.activeElement.tagName === "INPUT") document.activeElement.blur();
    return;
  }
  const map = { ArrowLeft: "left", ArrowRight: "right", ArrowUp: "up", ArrowDown: "down" };
  if (map[k]) {
    const cur = current();
    // let selects/steppers take left/right
    if (cur && cur.tagName === "SELECT" && (k === "ArrowLeft" || k === "ArrowRight")) {
      cur.selectedIndex = Math.max(0, Math.min(cur.options.length - 1, cur.selectedIndex + (k === "ArrowRight" ? 1 : -1)));
      cur.dispatchEvent(new Event("change", { bubbles: true }));
      e.preventDefault();
      return;
    }
    move(map[k]);
    e.preventDefault();
  } else if (k === "Enter" || k === " ") {
    if (current()) { activate(); e.preventDefault(); }
  } else if (k === "Escape" || k === "Backspace") {
    if (fire("back")) blip("back");
    e.preventDefault();
  } else if (k === "q" || k === "Q") {
    fire("tabPrev");
  } else if (k === "e" || k === "E") {
    fire("tabNext");
  } else {
    fire("action", e);
  }
});

document.addEventListener("pointerdown", () => document.querySelectorAll(".is-focus").forEach((x) => x.classList.remove("is-focus")));

// ───────── gamepad ─────────
const prev = {};
const repeatAt = {};
let padName = "";
export const pad = { connected: false, name: "" };

function pressed(gp, i) { return gp.buttons[i] && (gp.buttons[i].pressed || gp.buttons[i].value > 0.5); }

function poll(t) {
  const pads = navigator.getGamepads ? [...navigator.getGamepads()].filter(Boolean) : [];
  const gp = pads[0];
  pad.connected = !!gp;
  if (gp && gp.id !== padName) {
    padName = gp.id;
    pad.name = gp.id.replace(/\(.*?\)/g, "").trim();
    document.dispatchEvent(new CustomEvent("padchange"));
  } else if (!gp && padName) {
    padName = "";
    pad.name = "";
    document.dispatchEvent(new CustomEvent("padchange"));
  }
  if (gp && document.hasFocus()) {
    const ax = gp.axes[0] || 0, ay = gp.axes[1] || 0;
    const state = {
      up: pressed(gp, 12) || ay < -0.55, down: pressed(gp, 13) || ay > 0.55,
      left: pressed(gp, 14) || ax < -0.55, right: pressed(gp, 15) || ax > 0.55,
      a: pressed(gp, 0), b: pressed(gp, 1), x: pressed(gp, 2), y: pressed(gp, 3),
      lb: pressed(gp, 4), rb: pressed(gp, 5), start: pressed(gp, 9), select: pressed(gp, 8),
    };
    for (const [k, v] of Object.entries(state)) {
      const was = prev[k];
      const dirKey = ["up", "down", "left", "right"].includes(k);
      if (v && (!was || (dirKey && t > (repeatAt[k] || 0)))) {
        repeatAt[k] = t + (was ? 90 : 380); // initial delay then fast repeat
        if (dirKey) {
          const cur = current();
          if (cur && cur.tagName === "SELECT" && (k === "left" || k === "right")) {
            cur.selectedIndex = Math.max(0, Math.min(cur.options.length - 1, cur.selectedIndex + (k === "right" ? 1 : -1)));
            cur.dispatchEvent(new Event("change", { bubbles: true }));
          } else move(k);
        } else if (k === "a") activate();
        else if (k === "b") { if (fire("back")) blip("back"); }
        else if (k === "lb") fire("tabPrev");
        else if (k === "rb") fire("tabNext");
        else if (k === "start") fire("menu");
        else fire("action", { key: "pad-" + k });
      }
      prev[k] = v;
    }
  }
  requestAnimationFrame(poll);
}
requestAnimationFrame(poll);
