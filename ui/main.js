// Renders token usage emitted by the Rust backend.
// Global Tauri API (withGlobalTauri = true) — no bundler needed.

const { event, window: tauriWindow } = window.__TAURI__;

// --- number formatting: 1234 -> "1.2K", 3.4e6 -> "3.4M" ---
function fmt(n) {
  if (n < 1000) return String(n);
  const units = [
    [1e9, "B"],
    [1e6, "M"],
    [1e3, "K"],
  ];
  for (const [div, suffix] of units) {
    if (n >= div) {
      const v = n / div;
      return (v >= 100 ? v.toFixed(0) : v.toFixed(1)).replace(/\.0$/, "") + suffix;
    }
  }
  return String(n);
}

// --- rotating header titles (cycled on a loop) ---
const TITLES = [
  "TOKEN MAXXING 🔥",
  "burning the budget 💸",
  "feeding the machine 🤖",
  "context go brrr 🌀",
  "tokens go yeet 🚀",
  "just one more prompt 🤏",
  "maxxing responsibly 😇",
  "GPU goes brrr 🔥",
  "number must go up 📈",
];

function startTitleLoop() {
  const el = document.getElementById("status");
  let i = 0;
  const tick = () => {
    el.style.opacity = "0";
    setTimeout(() => {
      el.textContent = TITLES[i % TITLES.length];
      el.style.opacity = "1";
      i++;
    }, 300);
  };
  tick();
  setInterval(tick, 3000);
}

// --- funny status tiers based on tokens used today (drives flame intensity) ---
function tier(today) {
  if (today === 0) return { text: "idle 💤", maxing: false };
  if (today < 10_000) return { text: "warming up 🌱", maxing: false };
  if (today < 100_000) return { text: "cooking 🍳", maxing: false };
  if (today < 1_000_000) return { text: "TOKEN MAXING 🚀", maxing: true };
  if (today < 5_000_000) return { text: "going nuclear 🤯", maxing: true };
  return { text: "TOKEN MAXXING", maxing: true };
}

const shortModel = (m) =>
  m.replace(/^claude-/, "").replace(/-\d{8}$/, "");

// --- count-up tween so numbers roll instead of snap ---
function tweenTo(el, from, to) {
  const dur = 700;
  const start = performance.now();
  function step(now) {
    const p = Math.min((now - start) / dur, 1);
    const eased = 1 - Math.pow(1 - p, 3); // ease-out cubic
    el.textContent = fmt(Math.round(from + (to - from) * eased));
    if (p < 1) requestAnimationFrame(step);
  }
  requestAnimationFrame(step);
}

const shown = { today: 0, total: 0 };

function render(p) {
  const panel = document.querySelector(".panel");
  const todayEl = document.getElementById("today-num");
  const totalEl = document.getElementById("total-num");
  const statusEl = document.getElementById("status");

  if (p.today !== shown.today) {
    tweenTo(todayEl, shown.today, p.today);
    todayEl.classList.remove("pulse");
    void todayEl.offsetWidth; // restart animation
    todayEl.classList.add("pulse");
    shown.today = p.today;
  }
  if (p.total !== shown.total) {
    tweenTo(totalEl, shown.total, p.total);
    shown.total = p.total;
  }

  // Title text rotates on its own loop; here we only set flame intensity.
  panel.classList.toggle("maxing", tier(p.today).maxing);

  const list = document.getElementById("rows");
  if (!p.rows.length) {
    list.innerHTML = '<li class="empty">waiting for activity…</li>';
    return;
  }
  list.innerHTML = p.rows
    .map(
      (r) => `
      <li class="row">
        <span class="tag ${r.client}"></span>
        <span class="name">${shortModel(r.model)} <small>· ${r.client}</small></span>
        <span class="num">${fmt(r.total)}</span>
      </li>`
    )
    .join("");
}

startTitleLoop();

event.listen("usage-update", (e) => render(e.payload));

document.getElementById("close").addEventListener("click", () => {
  tauriWindow.getCurrentWindow().close();
});
