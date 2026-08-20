/**
 * どこから重いのかを、実ブラウザで測る。
 *
 * 仮想スクロールを入れるかどうかは「何行から重いか」を知らないと決められない。
 * 測らずに入れると、いちばん高い最適化を、効くかどうか分からないまま抱えることになる。
 *
 *   sh test/bulk.sh <db> <owner email> 500      # 使い捨ての大きい計画を作る
 *   FUGANTT_URL=http://127.0.0.1:3411 node test/measure.mjs
 *
 * 開発サーバーと同じポートで測らないこと。再起動の一瞬に別のプロセスが答えると、
 * 数字ではなく相手のページを測ることになる。DB をコピーして別ポートで立てるのが早い:
 *
 *   sqlite3 dev.db "VACUUM INTO 'perf.db'"
 *   FUGANTT_DB=perf.db PORT=3411 FUGANTT_OPEN=0 ./target/debug/fugantt
 *
 * 見るのは4つ。開くまで、選択を動かす打鍵（部分描画）、編集を開く打鍵（全体の描き直し）、
 * そして値を1つ確定する往復。体感に効くのは3つ目で、100ms を越えたあたりから「重い」。
 */
import puppeteer from "puppeteer-core";

const BASE = process.env["FUGANTT_URL"] ?? "http://127.0.0.1:1861";
const CHROME = "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome";

const browser = await puppeteer.launch({ executablePath: CHROME, headless: "new", args: ["--no-sandbox"] });
const page = await browser.newPage();
await page.setViewport({ width: 1600, height: 900 });

await page.goto(`${BASE}/login`, { waitUntil: "domcontentloaded" });
await page.evaluate(async () => {
  await fetch("/login", { method: "POST", body: new URLSearchParams({ email: "grid-test@example.com", password: "grid-test-password" }) });
});

/** キーを1つ送って、次のフレームが出るまで。描き直しの実測値。 */
const keyLatency = async (key, times) => {
  const samples = [];
  for (let i = 0; i < times; i++) {
    samples.push(
      await page.evaluate(async (key) => {
        const grid = document.querySelector(".fg-grid");
        grid.focus({ preventScroll: true });
        const at = performance.now();
        grid.dispatchEvent(new KeyboardEvent("keydown", { key, bubbles: true }));
        // 2フレーム待つ: 1つ目は同期処理の完了、2つ目はレイアウトと描画のあと。
        await new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done)));
        return performance.now() - at;
      }, key),
    );
  }
  samples.sort((a, b) => a - b);
  return {
    median: +samples[Math.floor(samples.length / 2)].toFixed(1),
    worst: +samples.at(-1).toFixed(1),
  };
};

const results = [];
const projects = process.env["FUGANTT_PROJECTS"]?.split(",") ?? [
  "test-project",
  "load-100",
  "load-500",
  "load-2000",
];

for (const project of projects) {
  const opened = Date.now();
  await page.goto(`${BASE}/projects/${project}`, { waitUntil: "domcontentloaded" });
  await page.waitForSelector(".fg-grid", { timeout: 60000 });
  const firstPaint = Date.now() - opened;

  await new Promise((r) => setTimeout(r, 600));

  const size = await page.evaluate(() => ({
    rows: document.querySelectorAll(".fg-pane-left .fg-row.fg-data").length,
    nodes: document.querySelectorAll(".fg-grid *").length,
    days: document.querySelectorAll(".fg-column").length,
  }));

  // 選択を動かすだけの打鍵。ここは部分描画なので、行数にあまり効かれない。
  const move = await keyLatency("ArrowDown", 12);

  // F2 は編集を開く＝島を丸ごと描き直す。行数がそのまま乗るのはこちら。
  const redraw = await (async () => {
    const samples = [];
    for (let i = 0; i < 9; i++) {
      samples.push(
        await page.evaluate(async () => {
          const grid = document.querySelector(".fg-grid");
          grid.focus({ preventScroll: true });
          const at = performance.now();
          grid.dispatchEvent(new KeyboardEvent("keydown", { key: "F2", bubbles: true }));
          await new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done)));
          const took = performance.now() - at;
          grid.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true }));
          await new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done)));
          return took;
        }),
      );
    }
    samples.sort((a, b) => a - b);
    return +samples[Math.floor(samples.length / 2)].toFixed(1);
  })();

  // 横スクロール1回ぶんの再描画。チャートは日付の数だけ要素がある。
  const scroll = await page.evaluate(async () => {
    const chart = document.querySelector(".fg-pane-chart");
    const at = performance.now();
    chart.scrollLeft += 800;
    await new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done)));
    return +(performance.now() - at).toFixed(1);
  });

  // 値を1つ確定する往復。島に打たせること。生の fetch で書くと、自分の書き込みが
  // 「他人の変更」として跳ね返り、計画をまるごと読み直す——実際の編集では起きない
  // 往復を測ることになる（それで1万行の1打が 206ms に見えていた。本当は76ms）。
  const commit = await (async () => {
    await page.evaluate(() => {
      window.__net = [];
      const original = window.fetch;
      window.fetch = async (...args) => {
        const at = performance.now();
        const response = await original(...args);
        const text = await response.clone().text();
        window.__net.push({ ms: performance.now() - at, kb: text.length / 1024 });
        return response;
      };
    });

    await page.click(".fg-pane-left .fg-row.fg-data .fg-cell");
    await page.keyboard.press("F2");
    await page.keyboard.type(`計測${Date.now() % 1000}`);

    const felt = Date.now();
    await page.keyboard.press("Enter");
    await page.waitForFunction(() => (window.__net?.length ?? 0) > 0, { timeout: 60000 });
    await page.evaluate(
      () => new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done))),
    );
    const whole = Date.now() - felt;

    const call = await page.evaluate(() => window.__net.at(-1));

    return { ms: +call.ms.toFixed(1), kb: Math.round(call.kb * 10) / 10, whole };
  })();

  results.push({
    project,
    ...size,
    "開く(ms)": firstPaint,
    "↓キー(ms)": move.median,
    "編集を開く(ms)": redraw,
    "確定の往復(ms)": commit.ms,
    "打鍵から描き直しまで(ms)": commit.whole,
    "返るJSON(KB)": commit.kb,
    "横スクロール(ms)": scroll,
  });
}

console.table(results);
await browser.close();
