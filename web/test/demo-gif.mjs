/**
 * README の先頭に置く動画（GIF）を撮る台本。
 *
 * 新しい DB で立てた fugantt に、今日を中心にした小さな計画を作り、
 * 手が止まらずに動く3つの場面を、人の速さでなぞって録画する。
 * 台本にしてあるのは、画面が変わったときに同じ手順で撮り直せるように。
 *
 *   FUGANTT_DB=/tmp/demo.db PORT=18670 FUGANTT_OPEN=0 fugantt     # 新しい DB で
 *   FUGANTT_URL=http://127.0.0.1:18670 node test/demo-gif.mjs /tmp/demo.webm
 *
 * GIF にするのは ffmpeg。色を先に128色へ絞ると、くっきりしたまま 1〜2MB に収まる:
 *
 *   ffmpeg -i /tmp/demo.webm -c:v libx264 -pix_fmt yuv420p /tmp/demo.mp4
 *   ffmpeg -i /tmp/demo.mp4 -vf "fps=12,scale=1280:-1:flags=lanczos,palettegen=max_colors=128:stats_mode=diff" /tmp/pal.png
 *   ffmpeg -i /tmp/demo.mp4 -i /tmp/pal.png -lavfi "fps=12,scale=1280:-1:flags=lanczos[x];[x][1:v]paletteuse=dither=bayer:bayer_scale=4:diff_mode=rectangle" ../docs/images/demo.gif
 *
 * 1. Ctrl+Enter で行を足して、そのまま名前・担当者・ステータス・日付を打つ
 * 2. チャートのバーを引くと、表の日付が一緒に動く
 * 3. 予定遅れで絞り込むと、遅れている行だけが残る
 */

import puppeteer from "puppeteer-core";

const BASE = process.env["FUGANTT_URL"] ?? "http://127.0.0.1:18670";
const OUT = process.argv[2] ?? "demo.webm";
const CHROME =
  process.env["CHROME"] ?? "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome";

const wait = (ms) => new Promise((done) => setTimeout(done, ms));

/** 今日から `days` 日ずらした日付。計画はいつ撮っても今日のまわりにある。 */
const day = (days) => {
  const at = new Date();
  at.setDate(at.getDate() + days);
  return at.toLocaleDateString("sv-SE");
};

const plan = {
  version: 1,
  name: "ECサイト リニューアル",
  tasks: [
    { name: "要件定義", depth: 0 },
    { name: "現行サイトの調査", depth: 1, assignee: "山田", status: "完了", start: day(-31), end: day(-21), actual_start: day(-31), actual_end: day(-17), progress: 100 },
    { name: "要件の取りまとめ", depth: 1, assignee: "佐藤", status: "完了", start: day(-18), end: day(-7), actual_start: day(-17), actual_end: day(-7), progress: 100 },
    { name: "設計", depth: 0 },
    { name: "デザインカンプ", depth: 1, assignee: "鈴木", status: "進行中", start: day(-18), end: day(-2), actual_start: day(-18), progress: 80 },
    { name: "画面設計", depth: 1, assignee: "鈴木", status: "進行中", start: day(-4), end: day(14), actual_start: day(-4), progress: 35 },
    { name: "DB設計", depth: 1, assignee: "田中", status: "待ち", start: day(-4), end: day(7), actual_start: day(-3), progress: 40, waits: [`${day(-1)}/${day(4)}`] },
    { name: "実装", depth: 0 },
    { name: "会員機能", depth: 1, assignee: "山田", status: "未着手", start: day(10), end: day(28) },
    { name: "商品一覧・検索", depth: 1, assignee: "鈴木", status: "未着手", start: day(17), end: day(35) },
    { name: "テスト", depth: 0 },
    { name: "結合テスト", depth: 1, assignee: "佐藤", status: "未着手", start: day(45), end: day(56) },
  ],
};

const browser = await puppeteer.launch({ executablePath: CHROME, headless: "new" });
const page = await browser.newPage();
await page.setViewport({ width: 1280, height: 640 });
await page.emulateMediaFeatures([{ name: "prefers-color-scheme", value: "light" }]);

// 新しい DB なら最初の人が管理者になる。撮るためだけのアカウント。
await page.goto(`${BASE}/login`);
const id = await page.evaluate(async (plan) => {
  const body = new URLSearchParams({ email: "demo", password: "Demo-recording-2026", name: "デモ" });
  await fetch("/register", { method: "POST", body });
  await fetch("/login", { method: "POST", body });
  await fetch("/projects", { method: "POST", body: new URLSearchParams({ name: plan.name }), redirect: "manual" });

  const html = await (await fetch("/")).text();
  const id = [...html.matchAll(/href="\/projects\/([^"/]+)"/g)].map((m) => m[1])[0];

  await fetch(`/api/projects/${id}/document`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(plan),
  });

  // この幅で見せる列だけ。動画の主役はチャートとの行き来なので、表は必要なぶんに絞る。
  const columns = new URLSearchParams();
  for (const key of ["late", "assignee", "status", "start", "end"]) columns.set(`column_${key}`, "on");
  columns.set("width_name", "200");
  columns.set("width_late", "72");
  columns.set("width_assignee", "72");
  columns.set("width_status", "84");
  await fetch(`/projects/${id}/columns`, { method: "POST", body: columns });

  return id;
}, plan);

await page.goto(`${BASE}/projects/${id}`, { waitUntil: "domcontentloaded" });
await page.waitForSelector(".fg-grid");

// 表は予定終了まで見える幅に。
const paneWidth = await page.evaluate(() => {
  const heads = [...document.querySelectorAll(".fg-pane-left .fg-heading .fg-cell")];
  const left = document.querySelector(".fg-pane-left").getBoundingClientRect().left;
  return Math.ceil(heads[heads.length - 1].getBoundingClientRect().right - left + 4);
});
await page.evaluate((width) => localStorage.setItem("fugantt:pane-width", String(width)), paneWidth);
await page.reload({ waitUntil: "domcontentloaded" });
await page.waitForSelector(".fg-grid");
await wait(800);

// 録画にはマウスの矢印が映らない。引いているのが見えるように、点を1つ描く。
await page.evaluate(() => {
  const dot = document.createElement("div");
  dot.id = "demo-pointer";
  Object.assign(dot.style, {
    position: "fixed", width: "18px", height: "18px", borderRadius: "50%",
    background: "rgb(37 99 235 / 0.35)", border: "2px solid rgb(37 99 235)",
    pointerEvents: "none", zIndex: 99999, transform: "translate(-50%, -50%)",
    left: "-40px", top: "-40px", transition: "background 0.1s",
  });
  document.body.append(dot);
  document.addEventListener("mousemove", (event) => {
    dot.style.left = `${event.clientX}px`;
    dot.style.top = `${event.clientY}px`;
  }, true);
  document.addEventListener("mousedown", () => (dot.style.background = "rgb(37 99 235 / 0.7)"), true);
  document.addEventListener("mouseup", () => (dot.style.background = "rgb(37 99 235 / 0.35)"), true);
});

/** 表の行の中で、名前が `name` の行のセルの中心。 */
const cellOf = (name, column) =>
  page.evaluate(
    (name, column) => {
      const rows = [...document.querySelectorAll(".fg-pane-left .fg-row.fg-data")];
      const row = rows.find((r) => r.querySelector(".fg-name-text")?.textContent.trim() === name);
      const cell = row?.querySelector(`.fg-cell-${column}`);
      const box = cell?.getBoundingClientRect();
      return box ? { x: box.x + Math.min(box.width / 2, 60), y: box.y + box.height / 2 } : null;
    },
    name,
    column,
  );

/** マウスを人の速さで動かす。 */
const glide = async (from, to, steps = 18) => {
  for (let step = 1; step <= steps; step++) {
    await page.mouse.move(from.x + ((to.x - from.x) * step) / steps, from.y + ((to.y - from.y) * step) / steps);
    await wait(16);
  }
};

const recorder = await page.screencast({ path: OUT });
await wait(900);

// --- 1. 行を足して、そのまま打つ ------------------------------------------------

const start = await cellOf("DB設計", "name");
await glide({ x: start.x + 300, y: start.y + 120 }, start);
await page.mouse.click(start.x, start.y);
await wait(250);
// 押したら手を離す。点が字の上に残ると、打っている名前が読めない。
await glide(start, { x: start.x + 40, y: start.y + 230 }, 10);
await wait(250);

await page.keyboard.down("Control");
await page.keyboard.press("Enter");
await page.keyboard.up("Control");
await wait(450);
await page.keyboard.type("決済APIの検証", { delay: 110 });
await wait(300);
// Tab で右へ。計算で決まる予定遅れは飛ばして担当者へ。
await page.keyboard.press("Tab");
await wait(450);

// 担当者とステータスは一覧から選ぶ列。録画には一覧の窓が映らないので、
// 選んだ結果を、人が選んだのと同じ経路（change）で入れる。確定すると右へ進む。
for (const value of ["田中", "進行中"]) {
  await page.keyboard.press("Enter");
  await wait(500);
  await page.evaluate((value) => {
    const select = document.querySelector("select.fg-editor");
    if (!select) return;
    select.value = value;
    select.dispatchEvent(new Event("change", { bubbles: true }));
  }, value);
  await wait(650);
}

// 日付は桁数だけで決まる。4桁は当年の月日。
for (const offset of [2, 6]) {
  const at = new Date();
  at.setDate(at.getDate() + offset);
  const typed = `${String(at.getMonth() + 1).padStart(2, "0")}${String(at.getDate()).padStart(2, "0")}`;
  await page.keyboard.type(typed, { delay: 140 });
  await wait(250);
  await page.keyboard.press("Enter");
  await wait(650);
}
await wait(900);

// --- 2. バーを引くと、表の日付も動く ----------------------------------------------

const bar = await page.evaluate(() => {
  const rows = [...document.querySelectorAll(".fg-pane-left .fg-row.fg-data")];
  const index = rows.findIndex((r) => r.querySelector(".fg-name-text")?.textContent.trim() === "会員機能");
  const target = document.querySelectorAll(".fg-bar-row")[index]?.querySelector(".fg-bar.is-plan");
  const pane = document.querySelector(".fg-pane-chart");
  if (target && pane) pane.scrollLeft = Math.max(0, target.offsetLeft - pane.clientWidth * 0.35);
  return true;
});
await wait(400);
const grip = await page.evaluate(() => {
  const rows = [...document.querySelectorAll(".fg-pane-left .fg-row.fg-data")];
  const index = rows.findIndex((r) => r.querySelector(".fg-name-text")?.textContent.trim() === "会員機能");
  const box = document.querySelectorAll(".fg-bar-row")[index]?.querySelector(".fg-bar.is-plan")?.getBoundingClientRect();
  return box ? { x: box.x + box.width / 2, y: box.y + box.height / 2 } : null;
});
if (bar && grip) {
  await glide({ x: grip.x - 160, y: grip.y - 90 }, grip);
  await wait(250);
  await page.mouse.down();
  await wait(200);
  const dayWidth = 26;
  await glide(grip, { x: grip.x + dayWidth * 5, y: grip.y }, 30);
  await wait(300);
  await page.mouse.up();
  await wait(1100);
}

// --- 3. 遅れている行だけ ------------------------------------------------------

const filter = await page.evaluate(() => {
  const box = document.querySelector('.fg-filters .fg-cell-late .fg-filter-pick')?.getBoundingClientRect();
  return box ? { x: box.x + box.width / 2, y: box.y + box.height / 2 } : null;
});
if (filter) {
  await glide({ x: filter.x + 200, y: filter.y + 200 }, filter);
  await page.mouse.click(filter.x, filter.y);
  await wait(500);
  const item = await page.evaluate(() => {
    const entry = [...document.querySelectorAll(".fg-pick-menu .fg-pick-item")].find((e) => e.textContent.trim() === "遅れ");
    const box = entry?.getBoundingClientRect();
    return box ? { x: box.x + 16, y: box.y + box.height / 2 } : null;
  });
  if (item) {
    await glide(filter, item, 12);
    await page.mouse.click(item.x, item.y);
    await wait(500);
    await page.keyboard.press("Escape");
  }
  await wait(1800);
}

await recorder.stop();
await browser.close();
