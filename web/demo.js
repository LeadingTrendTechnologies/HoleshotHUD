import init, { Preview } from "./pkg/mxbo_web_preview.js";

const BOARD = [
  ["none", "None"],
  ["pos", "Position"],
  ["classpos", "Class position"],
  ["sess", "Session time"],
  ["race", "Race time"],
  ["lap", "Lap"],
  ["left", "Laps remaining"],
  ["track", "Track name"],
  ["air", "Air temp"],
  ["best", "Best lap"],
  ["sbest", "Session best"],
  ["local", "Local time"],
  ["riders", "Riders"],
  ["stype", "Session type"],
];

const DASH = [
  ["none", "None"],
  ["speed", "Speed"],
  ["rpm", "RPM"],
  ["gear", "Gear"],
  ["pos", "Position"],
  ["num", "Bike number"],
  ["laps", "Lap count"],
  ["left", "Laps left"],
  ["last", "Last lap"],
  ["best", "Best lap"],
  ["cur", "Current lap"],
  ["delta", "Delta"],
  ["air", "Air temp"],
  ["eng", "Engine temp"],
  ["gap", "Gap"],
  ["int", "Interval"],
  ["pen", "Penalty"],
  ["sess", "Session time"],
  ["bike", "Bike"],
  ["class", "Class"],
];

const ST_COLS = [
  ["st_pos", "Position"],
  ["st_num", "Number"],
  ["st_name", "Name"],
  ["st_gap", "Gap"],
  ["st_interval", "Interval"],
  ["st_laps", "Completed Laps"],
  ["st_current", "Current lap"],
  ["st_best", "Fastest"],
  ["st_last", "Last lap"],
  ["st_status", "Status"],
  ["st_bike", "Bike"],
  ["st_penalty", "Penalty"],
  ["st_crashed", "Crashed"],
];

const REL_COLS = [
  ["rel_num", "Number"],
  ["rel_name", "Name"],
  ["rel_gap", "Gap"],
  ["rel_laps", "Completed Laps"],
  ["rel_current", "Current lap"],
  ["rel_pos", "Position"],
  ["rel_bike", "Bike"],
  ["rel_penalty", "Penalty"],
  ["rel_interval", "Interval"],
  ["rel_crashed", "Crashed"],
  ["rel_best", "Fastest"],
  ["rel_last", "Last lap"],
];

const MAP_TOGGLES = [
  ["map_others", "Other riders"],
  ["map_sf", "Start / finish"],
  ["map_arrows", "Track arrows"],
  ["map_crown", "Leader crown"],
  ["map_place", "Nearest ahead / behind"],
  ["map_numbers", "Numbers in dots"],
];

const MINI_TOGGLES = [
  ["mini_others", "Other riders"],
  ["mini_sf", "Start / finish"],
  ["mini_arrows", "Track arrows"],
  ["mini_crown", "Leader crown"],
  ["mini_place", "Nearest ahead / behind"],
  ["mini_numbers", "Numbers in dots"],
];

const canvas = document.getElementById("hud");
const ctx = canvas.getContext("2d", { alpha: true });
const settings = document.getElementById("settings");

await init({ module_or_path: new URL("./pkg/mxbo_web_preview_bg.wasm?v=dot-slate", import.meta.url) });
const preview = new Preview();

function syncButtons() {
  const active = preview.active_widget();
  for (const btn of document.querySelectorAll("[data-widget]")) {
    btn.classList.toggle("on", btn.dataset.widget === active);
  }
}

function el(html) {
  const t = document.createElement("template");
  t.innerHTML = html.trim();
  return t.content;
}

function selectHtml(key, options) {
  const cur = preview.get_field(key);
  return `<select data-field="${key}">${options
    .map(([v, l]) => `<option value="${v}"${v === cur ? " selected" : ""}>${l}</option>`)
    .join("")}</select>`;
}

function toggleRow(key, label) {
  return `<div class="row"><label>${label}</label><input class="toggle" type="checkbox" data-bool="${key}" ${preview.get_bool(key) ? "checked" : ""}></div>`;
}

function stepperRow(key, label, min, max) {
  return `<div class="row"><label>${label}</label><div class="stepper"><button type="button" data-step="${key}" data-d="-1" data-min="${min}" data-max="${max}">−</button><span data-int-label="${key}">${preview.get_int(key)}</span><button type="button" data-step="${key}" data-d="1" data-min="${min}" data-max="${max}">+</button></div></div>`;
}

function sliderRow(key, label, min, max, suffix) {
  const v = preview.get_int(key);
  return `<div class="row stack"><label>${label} <span class="range-val" data-int-label="${key}">${v}${suffix}</span></label><input type="range" min="${min}" max="${max}" value="${v}" data-int="${key}" data-suffix="${suffix}"></div>`;
}

function fieldRow(key, label, options) {
  return `<div class="row stack"><label>${label}</label>${selectHtml(key, options)}</div>`;
}

function look(prefix) {
  return `${sliderRow(`${prefix}_font`, "Font size", 70, 160, "%")}${toggleRow(`${prefix}_bold`, "Bold text")}`;
}

function snapGrid() {
  const cells = ["tl", "t", "tr", "l", "c", "r", "bl", "b", "br"];
  return `<div class="section">Position on screen</div><div class="snap">${cells
    .map((a) => `<button type="button" data-snap="${a}" aria-label="Snap ${a}"></button>`)
    .join("")}</div>`;
}

function renderSettings() {
  const w = preview.active_widget();
  let html = "";
  if (w === "standings") {
    html += `<div class="section">Header</div>`;
    html += fieldRow("st_head0", "Left", BOARD);
    html += fieldRow("st_head1", "Middle", BOARD);
    html += fieldRow("st_head2", "Right", BOARD);
    html += `<div class="section">Footer</div>`;
    html += fieldRow("st_foot0", "Left", BOARD);
    html += fieldRow("st_foot1", "Middle", BOARD);
    html += fieldRow("st_foot2", "Right", BOARD);
    html += stepperRow("standings_rows", "Rows", 3, 40);
    html += `<div class="section">Columns</div>`;
    html += ST_COLS.map(([k, l]) => toggleRow(k, l)).join("");
    html += sliderRow("st_bg", "Background", 0, 100, "%");
    html += look("st");
  } else if (w === "relative") {
    html += `<div class="section">Header</div>`;
    html += fieldRow("rel_head0", "Left", BOARD);
    html += fieldRow("rel_head1", "Middle", BOARD);
    html += fieldRow("rel_head2", "Right", BOARD);
    html += `<div class="section">Footer</div>`;
    html += fieldRow("rel_foot0", "Left", BOARD);
    html += fieldRow("rel_foot1", "Middle", BOARD);
    html += fieldRow("rel_foot2", "Right", BOARD);
    html += stepperRow("relative_count", "Nearby riders", 1, 8);
    html += `<div class="section">Columns</div>`;
    html += REL_COLS.map(([k, l]) => toggleRow(k, l)).join("");
    html += sliderRow("rel_bg", "Background", 0, 100, "%");
    html += look("rel");
  } else if (w === "map") {
    html += `<div class="section">On the map</div>`;
    html += MAP_TOGGLES.map(([k, l]) => toggleRow(k, l)).join("");
    html += fieldRow("map_dot", "Dot number", [["num", "Number"], ["pos", "Position"]]);
    html += sliderRow("map_bg", "Background", 0, 100, "%");
    html += look("map");
  } else if (w === "minimap") {
    html += `<div class="section">On the minimap</div>`;
    html += MINI_TOGGLES.map(([k, l]) => toggleRow(k, l)).join("");
    html += fieldRow("mini_dot", "Dot number", [["num", "Number"], ["pos", "Position"]]);
    html += sliderRow("mini_zoom", "Zoom", 0, 100, "%");
    html += sliderRow("mini_bg", "Background", 0, 100, "%");
    html += look("mini");
  } else if (w === "radar") {
    html += `<div class="section">Blind spots</div>`;
    html += toggleRow("radar_sides", "Riders beside you");
    html += toggleRow("radar_rear", "Riders behind you");
    html += sliderRow("radar_bg", "Panel opacity", 0, 100, "%");
    html += look("radar");
  } else if (w === "dash") {
    html += `<div class="section">Footer</div>`;
    html += fieldRow("dash_left", "Left", DASH);
    html += fieldRow("dash_mid", "Middle", DASH);
    html += fieldRow("dash_right", "Right", DASH);
    html += sliderRow("dash_bg", "Panel opacity", 0, 100, "%");
    html += look("dash");
  } else if (w === "ticker") {
    html += toggleRow("ticker_title", "Track name");
    html += toggleRow("ticker_autoscroll", "Autoscroll");
    html += `<div class="section">Side info</div>`;
    html += fieldRow("ticker_left", "Left", BOARD);
    html += fieldRow("ticker_right", "Right", BOARD);
    html += stepperRow("ticker_count", "Riders shown", 3, 15);
    html += sliderRow("ticker_bg", "Panel opacity", 0, 100, "%");
    html += look("ticker");
  }
  html += snapGrid();
  settings.replaceChildren(el(html));
}

const layoutEdit = document.getElementById("layout-edit");
layoutEdit.checked = preview.get_bool("layout_edit");
layoutEdit.addEventListener("change", () => {
  preview.set_bool("layout_edit", layoutEdit.checked);
});

for (const btn of document.querySelectorAll("[data-widget]")) {
  btn.addEventListener("click", () => {
    preview.select_widget(btn.dataset.widget);
    syncButtons();
    renderSettings();
  });
}

settings.addEventListener("change", (e) => {
  const t = e.target;
  if (t.dataset.bool) preview.set_bool(t.dataset.bool, t.checked);
  if (t.dataset.field) preview.set_field(t.dataset.field, t.value);
  if (t.dataset.int) {
    preview.set_int(t.dataset.int, Number(t.value));
    for (const label of settings.querySelectorAll(`[data-int-label="${t.dataset.int}"]`)) {
      label.textContent = `${t.value}${t.dataset.suffix || ""}`;
    }
  }
});

settings.addEventListener("click", (e) => {
  const step = e.target.closest("[data-step]");
  if (step) {
    const key = step.dataset.step;
    const next = Math.min(Number(step.dataset.max), Math.max(Number(step.dataset.min), preview.get_int(key) + Number(step.dataset.d)));
    preview.set_int(key, next);
    for (const label of settings.querySelectorAll(`[data-int-label="${key}"]`)) label.textContent = String(next);
    return;
  }
  const snap = e.target.closest("[data-snap]");
  if (snap) preview.snap_widget(snap.dataset.snap);
});

function norm(e) {
  const r = canvas.getBoundingClientRect();
  return {
    nx: (e.clientX - r.left) / r.width,
    ny: (e.clientY - r.top) / r.height,
  };
}

let dragging = false;
canvas.addEventListener("pointerdown", (e) => {
  const { nx, ny } = norm(e);
  preview.pointer_down(nx, ny, canvas.width, canvas.height);
  dragging = true;
  canvas.setPointerCapture(e.pointerId);
});
canvas.addEventListener("pointermove", (e) => {
  const { nx, ny } = norm(e);
  if (dragging) {
    preview.pointer_move(nx, ny, canvas.width, canvas.height);
  } else {
    canvas.style.cursor = preview.hover_cursor(nx, ny, canvas.width, canvas.height) || "default";
  }
});
canvas.addEventListener("pointerup", (e) => {
  preview.pointer_up();
  dragging = false;
  try { canvas.releasePointerCapture(e.pointerId); } catch {}
});
canvas.addEventListener("pointerleave", () => {
  if (!dragging) canvas.style.cursor = "default";
});

syncButtons();
renderSettings();

let last = performance.now();
function frame(now) {
  const dt = Math.min(0.05, (now - last) / 1000);
  last = now;
  preview.tick(dt);
  const w = canvas.width;
  const h = canvas.height;
  const bytes = preview.frame(w, h);
  ctx.putImageData(new ImageData(new Uint8ClampedArray(bytes), w, h), 0, 0);
  requestAnimationFrame(frame);
}
requestAnimationFrame(frame);
