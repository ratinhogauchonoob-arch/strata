// Boot sequence: the five strata bars fly in from alternating sides, lock into the mark,
// then the wordmark settles. "duo" uses accent 1 + 2, "spectrum" all six palette colors.

export const PALETTE = {
  ember: "#FF7A2F",
  cobalt: "#5163F5",
  signal: "#2FCB72",
  fog: "#D5D8DE",
  slate: "#7D838E",
  onyx: "#050506",
};

const BARS = [[14, 56], [8, 30], [14, 50], [34, 56], [8, 50]];
const ease = (t) => 1 - Math.pow(1 - Math.min(Math.max(t, 0), 1), 4);
const clamp01 = (t) => Math.min(Math.max(t, 0), 1);

export function playBoot({ style = "duo", accent1 = "ember", accent2 = "cobalt", version = "", reduceMotion = false } = {}) {
  const el = document.getElementById("boot");
  if (style === "off") {
    el.remove();
    return Promise.resolve();
  }
  const canvas = el.querySelector("canvas");
  const ctx = canvas.getContext("2d");
  const a1 = PALETTE[accent1] || PALETTE.ember;
  const a2 = PALETTE[accent2] || PALETTE.cobalt;
  const barColors = style === "spectrum"
    ? [PALETTE.ember, PALETTE.cobalt, PALETTE.fog, PALETTE.signal, PALETTE.slate]
    : [a1, a2, PALETTE.fog, a2, a1];
  const lineColors = style === "spectrum" ? Object.values(PALETTE) : [a1, a2, PALETTE.slate];
  const DURATION = reduceMotion ? 900 : 2600;

  let w = 0, h = 0, dpr = 1;
  const resize = () => {
    dpr = window.devicePixelRatio || 1;
    w = canvas.clientWidth; h = canvas.clientHeight;
    canvas.width = w * dpr; canvas.height = h * dpr;
  };
  resize();
  window.addEventListener("resize", resize);

  // background drift lines (deterministic so every boot looks the same)
  const lines = Array.from({ length: 26 }, (_, i) => {
    const r = Math.sin(i * 91.7) * 0.5 + 0.5;
    const r2 = Math.sin(i * 13.3) * 0.5 + 0.5;
    return { y: (i + 0.5) / 26, len: 0.15 + r * 0.45, x: r2, speed: 0.012 + r * 0.03, color: lineColors[i % lineColors.length], alpha: 0.05 + r2 * 0.08, thick: 1 + (i % 3) };
  });

  let done = false, resolveFn;
  const finish = () => {
    if (done) return;
    done = true;
    el.classList.add("out");
    window.removeEventListener("keydown", finish);
    el.removeEventListener("pointerdown", finish);
    setTimeout(() => { el.remove(); resolveFn(); }, 520);
  };
  window.addEventListener("keydown", finish);
  el.addEventListener("pointerdown", finish);

  const start = performance.now();
  const frame = (now) => {
    if (done) return;
    const t = ((now - start) / 1000) * (2600 / DURATION); // seconds on the 2.6 s timeline
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.clearRect(0, 0, w, h);

    // drifting strata in the background
    const bgIn = clamp01(t / 0.6);
    for (const l of lines) {
      const x = (((l.x + l.speed * t) % 1.4) - 0.2) * w;
      ctx.globalAlpha = l.alpha * bgIn;
      ctx.fillStyle = l.color;
      ctx.fillRect(x, l.y * h, l.len * w, l.thick);
    }
    ctx.globalAlpha = 1;

    // the mark
    const unit = Math.min(w, h) * 0.0052;
    const barH = 9 * unit, gap = 3.5 * unit;
    const markW = 56 * unit, markH = 5 * barH + 4 * gap;
    const ox = w / 2 - (markW + 8 * unit) / 2;
    const oy = h / 2 - markH / 2 - h * 0.06;
    BARS.forEach(([a, b], i) => {
      const k = ease((t - 0.18 - i * 0.09) / 0.75);
      if (k <= 0) return;
      const fromLeft = i % 2 === 0;
      const tx = ox + a * unit;
      const width = (b - a) * unit;
      const travel = (fromLeft ? -1 : 1) * (w * 0.6);
      const x = tx + travel * (1 - k);
      // squash while flying, round when locked
      const sx = 1 + (1 - k) * 0.9;
      const y = oy + i * (barH + gap);
      ctx.save();
      ctx.translate(x + width / 2, y + barH / 2);
      ctx.scale(sx, 1 - (1 - k) * 0.35);
      ctx.beginPath();
      ctx.roundRect(-width / 2, -barH / 2, width, barH, barH / 2);
      ctx.fillStyle = barColors[i];
      ctx.globalAlpha = 0.25 + 0.75 * k;
      ctx.fill();
      if (barColors[i] === PALETTE.onyx) { ctx.strokeStyle = "rgba(255,255,255,.35)"; ctx.lineWidth = 1; ctx.stroke(); }
      ctx.restore();
    });
    ctx.globalAlpha = 1;

    // light sweep across the locked mark
    const sw = clamp01((t - 1.15) / 0.55);
    if (sw > 0 && sw < 1) {
      const sx = ox - 20 * unit + (markW + 40 * unit) * sw;
      const g = ctx.createLinearGradient(sx - 14 * unit, 0, sx + 14 * unit, 0);
      g.addColorStop(0, "rgba(255,255,255,0)");
      g.addColorStop(0.5, "rgba(255,255,255,0.55)");
      g.addColorStop(1, "rgba(255,255,255,0)");
      ctx.save();
      ctx.beginPath();
      BARS.forEach(([a, b], i) => ctx.roundRect(ox + a * unit, oy + i * (barH + gap), (b - a) * unit, barH, barH / 2));
      ctx.clip();
      ctx.fillStyle = g;
      ctx.fillRect(ox, oy, markW + 10 * unit, markH);
      ctx.restore();
    }

    // wordmark
    const wk = ease((t - 1.25) / 0.7);
    if (wk > 0) {
      const size = Math.max(18, unit * 9);
      ctx.font = `760 ${size}px Archivo, "Segoe UI", sans-serif`;
      try { ctx.fontStretch = "expanded"; } catch { /* older engines */ }
      ctx.textAlign = "center";
      ctx.textBaseline = "top";
      const spacing = (0.62 - 0.3 * wk) * size;
      try { ctx.letterSpacing = spacing + "px"; } catch { /* ignore */ }
      ctx.fillStyle = `rgba(236,238,241,${wk})`;
      ctx.fillText("STRATA", w / 2 + spacing / 2, oy + markH + unit * 14);
      try { ctx.letterSpacing = "0px"; ctx.fontStretch = "normal"; } catch { /* ignore */ }

      ctx.font = `500 ${Math.max(10, unit * 2.4)}px "Plex Mono", monospace`;
      try { ctx.letterSpacing = unit * 0.9 + "px"; } catch { /* ignore */ }
      ctx.fillStyle = `rgba(166,171,180,${wk * 0.9})`;
      ctx.fillText(`LAUNCHER${version ? "  ·  V" + version : ""}`, w / 2, oy + markH + unit * 14 + size * 1.5);
      try { ctx.letterSpacing = "0px"; } catch { /* ignore */ }
    }

    // progress hairline
    const pk = clamp01(t / 2.4);
    const lw = Math.min(260, w * 0.2);
    ctx.fillStyle = "rgba(255,255,255,.08)";
    ctx.fillRect(w / 2 - lw / 2, h * 0.86, lw, 1);
    ctx.fillStyle = a1;
    ctx.fillRect(w / 2 - lw / 2, h * 0.86, lw * ease(pk), 1);

    if (now - start >= DURATION) finish();
    else requestAnimationFrame(frame);
  };

  // skip with any gamepad button
  const padSkip = () => {
    if (done) return;
    const pads = navigator.getGamepads ? [...navigator.getGamepads()].filter(Boolean) : [];
    if (pads.some((p) => p.buttons.some((b) => b.pressed))) finish();
    else requestAnimationFrame(padSkip);
  };

  return new Promise((resolve) => {
    resolveFn = resolve;
    document.fonts.ready.then(() => {
      requestAnimationFrame(frame);
      requestAnimationFrame(padSkip);
    });
  });
}
