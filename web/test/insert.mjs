/**
 * 行を足す打鍵（Ctrl+Enter）の連打を、人と同じ順序で測る。
 *
 *   sh test/bulk.sh <db> <owner email> 5
 *   FUGANTT_URL=http://127.0.0.1:3411 node test/insert.mjs load-5 200
 *
 * 連打すると重くなるのか——1打目と200打目を比べる——を見るための道具なので、
 * まとめではなく1打ずつその場で出す。ブラウザに実際のキーを送るので、
 * 「足す→編集が開く→次の打鍵は確定」という本当の順序をそのままなぞる。
 */
import puppeteer from "puppeteer-core";

const BASE = process.env["FUGANTT_URL"] ?? "http://127.0.0.1:3411";
const CHROME = "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome";
const project = process.argv[2] ?? "load-5";
const times = Number(process.argv[3] ?? 200);

const browser = await puppeteer.launch({ executablePath: CHROME, headless: "new", args: ["--no-sandbox"] });
const page = await browser.newPage();
await page.setViewport({ width: 1600, height: 900 });
page.on("console", (m) => {
  if (m.type() === "error") console.log("[console]", m.text());
});
page.on("pageerror", (error) => console.log("[例外]", error.message));

await page.goto(`${BASE}/login`, { waitUntil: "domcontentloaded" });
await page.evaluate(async () => {
  await fetch("/login", {
    method: "POST",
    body: new URLSearchParams({ email: "grid-test@example.com", password: "grid-test-password" }),
  });
});

const opened = Date.now();
await page.goto(`${BASE}/projects/${project}`, { waitUntil: "domcontentloaded" });
await page.waitForSelector(".fg-grid", { timeout: 60000 });
const firstPaint = Date.now() - opened;
await new Promise((r) => setTimeout(r, 800));

// 往復と返る JSON の大きさは、島の中の fetch を包んで拾う。
await page.evaluate(() => {
  window.__net = [];
  const original = window.fetch;
  window.fetch = async (...args) => {
    const at = performance.now();
    const response = await original(...args);
    const text = await response.clone().text();
    window.__net.push({
      url: String(args[0]),
      ms: +(performance.now() - at).toFixed(1),
      kb: +(text.length / 1024).toFixed(1),
      ok: response.ok,
      why: response.ok ? "" : text.slice(0, 120),
    });
    return response;
  };
});

await page.click(".fg-pane-left .fg-row.fg-data .fg-cell");

const samples = [];
for (let i = 0; i < times; i++) {
  const sent = await page.evaluate(() => window.__net.length);
  const start = Date.now();

  await page.keyboard.down("Control");
  await page.keyboard.press("Enter");
  await page.keyboard.up("Control");
  await page.evaluate(async () => {
    await new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done)));
  });

  const state = await page.evaluate(
    (sent) => {
      const pane = document.querySelector(".fg-pane-left");
      const spacers = [...document.querySelectorAll(".fg-pane-left .fg-spacer")].map((s) => s.offsetHeight);
      return {
        行: document.querySelectorAll(".fg-pane-left .fg-row.fg-data").length,
        DOM: document.querySelectorAll(".fg-grid *").length,
        往復: window.__net.length > sent ? window.__net.at(-1).ms : 0,
        KB: window.__net.length > sent ? window.__net.at(-1).kb : 0,
        上: pane?.scrollTop ?? -1,
        丈: pane?.scrollHeight ?? -1,
        枠: pane?.clientHeight ?? -1,
        余白: spacers.join("+"),
      };
    },
    sent,
  );

  samples.push({ 全体: Date.now() - start, ...state });
  const last = samples.at(-1);

  // 行が消えたら、そこで止めて画面に何が出ているかを見る。連打の途中で
  // 表が空になるのは、遅いのとは別の壊れ方で、続きを測っても意味がない。
  if (state.行 === 0) {
    console.log(`\n${i + 1} 打目で表が空になった。`);
    console.log(
      await page.evaluate(async () => {
        const pane = document.querySelector(".fg-pane-left");
        const look = () => ({
          行: document.querySelectorAll(".fg-pane-left .fg-row.fg-data").length,
          余白: [...document.querySelectorAll(".fg-pane-left .fg-spacer")].map((s) => s.offsetHeight),
          上: pane.scrollTop,
          丈: pane.scrollHeight,
        });
        const いま = look();

        pane.scrollTop = 0;
        pane.dispatchEvent(new Event("scroll", { bubbles: true }));
        await new Promise((done) => setTimeout(done, 200));
        const 上に戻すと = look();

        pane.scrollTop = pane.scrollHeight;
        pane.dispatchEvent(new Event("scroll", { bubbles: true }));
        await new Promise((done) => setTimeout(done, 200));
        const 下に戻すと = look();

        const server = await (await fetch(location.pathname.replace("/projects/", "/api/projects/") + "/grid")).json();

        return {
          いま,
          上に戻すと,
          下に戻すと,
          編集中: !!document.querySelector(".fg-editor:not(.is-typist)"),
          サーバーの行数: server.tasks.length,
          折りたたみ: document.querySelectorAll(".fg-fold.is-folded, .fg-fold[aria-expanded='false']").length,
          子: server.tasks.filter((task) => task.parent_id).length,
        };
      }),
    );
    break;
  }
  console.log(
    `${String(i + 1).padStart(3)} 打目  全体 ${String(last.全体).padStart(6)}ms` +
      `  往復 ${String(last.往復).padStart(6)}ms  ${last.KB}KB  行 ${last.行}` +
      `  上 ${last.上}/${last.丈} 枠 ${last.枠}  余白 ${last.余白}`,
  );
}

const refused = await page.evaluate(() => window.__net.filter((call) => !call.ok));
if (refused.length) console.table(refused.slice(0, 6));

const all = samples.map((s) => s.全体).sort((a, b) => a - b);
console.log(`\n${project} / 開く ${firstPaint}ms`);
console.log(`1打あたり: 中央 ${all[Math.floor(all.length / 2)]}ms / 最悪 ${all.at(-1)}ms`);

await browser.close();
