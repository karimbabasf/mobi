// Renders the Mobi mark (assets/icon.svg) to the app source PNG, plus two menu-bar
// tray icons: a monochrome template diamond (idle) and a colored diamond with an amber
// dot (pending). Run: `npm run gen:icon`, then `npm run tauri icon app-icon.png`.
import { Resvg } from "@resvg/resvg-js";
import { readFileSync, writeFileSync, mkdirSync } from "node:fs";

function toPng(svg, width) {
  const r = new Resvg(svg, { fitTo: { mode: "width", value: width } });
  return r.render().asPng();
}

const appSvg = readFileSync("assets/icon.svg", "utf8");

// Menu-bar template icon: a black diamond on transparent. macOS recolors it for the bar.
const trayIdle = `<svg xmlns="http://www.w3.org/2000/svg" width="44" height="44" viewBox="0 0 44 44">
  <path d="M22 5 L37 22 L22 39 L7 22 Z" fill="#000000"/>
</svg>`;

// Pending state: colored (non-template) diamond shifted left, with an amber alert dot.
const trayAlert = `<svg xmlns="http://www.w3.org/2000/svg" width="44" height="44" viewBox="0 0 44 44">
  <path d="M19 6 L33 22 L19 38 L5 22 Z" fill="#f5f5f7"/>
  <circle cx="37" cy="9" r="6.5" fill="#FFD60A"/>
</svg>`;

mkdirSync("src-tauri/icons", { recursive: true });
writeFileSync("app-icon.png", toPng(appSvg, 1024));
writeFileSync("src-tauri/icons/tray-idle.png", toPng(trayIdle, 44));
writeFileSync("src-tauri/icons/tray-alert.png", toPng(trayAlert, 44));
console.log("wrote app-icon.png, src-tauri/icons/tray-idle.png, src-tauri/icons/tray-alert.png");
