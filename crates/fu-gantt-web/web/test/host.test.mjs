/**
 * The grid, on a host that is not fugantt.
 *
 * `examples/minimal-host` serves it at an address of its own. This drives a
 * real browser at it and checks the one thing no other test can: that the
 * script draws, and that every request it makes goes where the page said.
 *
 *   PORT=18620 cargo run -p minimal-host        (in another terminal)
 *   HOST_URL=http://127.0.0.1:18620 node test/host.test.mjs
 */

import puppeteer from "puppeteer-core";

const BASE = process.env["HOST_URL"] ?? "http://127.0.0.1:18620";
const CHROME =
  process.env["CHROME"] ?? "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome";

const passed = [];
const failed = [];
const check = (name, ok, detail = "") =>
  (ok ? passed : failed).push(detail ? `${name} — ${detail}` : name);

const browser = await puppeteer.launch({
  executablePath: CHROME,
  headless: "new",
  args: ["--no-sandbox"],
});
const page = await browser.newPage();
await page.setViewport({ width: 1400, height: 700 });

const pageErrors = [];
page.on("pageerror", (error) => pageErrors.push(String(error)));

const asked = [];
page.on("request", (request) => asked.push(new URL(request.url()).pathname));

await page.goto(BASE, { waitUntil: "domcontentloaded" });
await page.waitForSelector(".fg-grid", { timeout: 10000 }).catch(() => {});

const names = () =>
  page.evaluate(() =>
    [...document.querySelectorAll(".fg-pane-left .fg-row.fg-data .fg-name-text")].map(
      (cell) => cell.textContent,
    ),
  );

check("グリッドが描かれる", (await names()).join(",") === "設計,実装,テスト", (await names()).join(","));

// Rename the first row the way a person does: select, F2, type, Enter.
await page.click(".fg-pane-left .fg-row.fg-data .fg-name-text");
await page.keyboard.press("F2");
await page.keyboard.down("Meta");
await page.keyboard.press("a");
await page.keyboard.up("Meta");
await page.keyboard.type("基本設計");
await page.keyboard.press("Enter");
await new Promise((resolve) => setTimeout(resolve, 500));

check("名前を書き換えられる", (await names())[0] === "基本設計", (await names()).join(","));

// And it was the server that kept it, not only the screen.
const kept = await page.evaluate(async () => {
  const response = await fetch("/g/api/plans/demo/grid");
  return (await response.json()).tasks[0].name;
});
check("書いた値がホストに残っている", kept === "基本設計", kept);

const api = asked.filter((path) => path.includes("/api/"));
check("API を叩いている", api.length >= 2, String(api.length));
check(
  "すべてページが指した基点に向かう",
  api.every((path) => path.startsWith("/g/api/plans/demo/")),
  api.filter((path) => !path.startsWith("/g/api/plans/demo/")).join(" "),
);
check("JavaScript エラーが出ていない", pageErrors.length === 0, pageErrors.join(" / "));

await browser.close();

for (const name of passed) console.log("  ✓", name);
for (const name of failed) console.log("  ✗", name);
console.log(`\n${passed.length} passed, ${failed.length} failed`);

process.exit(failed.length ? 1 : 0);
