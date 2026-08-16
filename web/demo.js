import init, { Preview } from "./pkg/mxbo_web_preview.js";

const canvas = document.getElementById("hud");
const ctx = canvas.getContext("2d", { alpha: true });

await init();
const preview = new Preview();

for (const btn of document.querySelectorAll("[data-widget]")) {
  const name = btn.dataset.widget;
  btn.classList.toggle("on", preview.widget_on(name));
  btn.addEventListener("click", () => {
    const next = !preview.widget_on(name);
    preview.set_widget(name, next);
    btn.classList.toggle("on", next);
  });
}

let last = performance.now();
function frame(now) {
  const dt = Math.min(0.05, (now - last) / 1000);
  last = now;
  preview.tick(dt);
  const w = canvas.width;
  let h = canvas.height;
  const bytes = preview.frame(w, h);
  const img = new ImageData(new Uint8ClampedArray(bytes), w, h);
  ctx.putImageData(img, 0, 0);
  requestAnimationFrame(frame);
}
requestAnimationFrame(frame);
