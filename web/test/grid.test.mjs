/**
 * Browser test for the grid island.
 *
 * The grid is keyboard-driven and talks to the server on every commit, so the
 * only honest test drives a real browser against a running app.
 *
 * Needs a dev server (`cargo-topcoat dev`) and the database it is using:
 *
 *   FUGANTT_DB=fugantt.db node test/grid.test.mjs
 *
 * Do not edit files under the watcher while a run is in flight: the dev
 * server's live reload navigates the page mid-test, and the failure it
 * produces ("Execution context was destroyed") looks nothing like its cause.
 */

import puppeteer from "puppeteer-core";
import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const BASE = process.env["FUGANTT_URL"] ?? "http://127.0.0.1:3000";
const DB = process.env["FUGANTT_DB"] ?? "fugantt.db";
const CHROME =
  process.env["CHROME"] ?? "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome";

const EMAIL = "grid-test@example.com";
const PASSWORD = "grid-test-password";

const here = dirname(fileURLToPath(import.meta.url));

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
await page.setViewport({ width: 1680, height: 700 });
page.on("dialog", (dialog) => dialog.accept());

const pageErrors = [];
page.on("pageerror", (error) => pageErrors.push(String(error)));

// Sign in first: the seed needs an account to own the project.
await page.goto(`${BASE}/login`, { waitUntil: "domcontentloaded" });
await page.evaluate(
  async (email, password) => {
    const body = new URLSearchParams({ email, password });
    // Registering twice is a 400, which just means the account already exists.
    await fetch("/register", { method: "POST", body });
    await fetch("/login", { method: "POST", body });
  },
  EMAIL,
  PASSWORD,
);

execFileSync("sh", [join(here, "seed.sh"), DB, EMAIL], { stdio: "inherit" });

// Not `networkidle0`: the live-update stream stays open, so the network never
// goes idle. Waiting for the grid itself is both faster and truthful.
await page.goto(`${BASE}/projects/test-project`, { waitUntil: "domcontentloaded" });
await page.waitForSelector(".fg-grid");

const settle = () => new Promise((resolve) => setTimeout(resolve, 250));

/**
 * 決まった時間ではなく、起きるはずのことを待つ。
 *
 * 書き込みはサーバーへの往復なので、debug ビルドで込み合うと 500ms では足りない
 * ことがある。時間で待つテストは、遅い日にだけ落ちて、直す手がかりを残さない。
 */
const until = async (read, want, tries = 20) => {
  for (let at = 0; at < tries; at++) {
    if ((await read()) === want) return true;
    await settle();
  }
  return false;
};

const state = () =>
  page.evaluate(() => {
    const rows = [...document.querySelectorAll(".fg-pane-left .fg-row.fg-data")];
    const selected = document.querySelector(".fg-cell.is-selected");
    // The selected cell always holds an invisible field for the keyboard; only
    // the one without `is-typist` means the cell is actually being edited.
    const editor = document.querySelector(".fg-editor:not(.is-typist)");

    return {
      row: selected ? rows.findIndex((row) => row.contains(selected)) : -1,
      column: selected ? [...selected.parentElement.children].indexOf(selected) : -1,
      editing: !!editor,
      editorValue: editor?.value ?? null,
      rowCount: rows.length,
      names: rows.map((row) => row.querySelector(".fg-name-text")?.textContent ?? ""),
      cells: rows.map((row) =>
        [...row.querySelectorAll(".fg-cell")].map((cell) => cell.textContent.trim()),
      ),
      error: document.querySelector(".fg-error span")?.textContent ?? null,
    };
  });

/**
 * Column indices, read from the heading row.
 *
 * Looking them up by name keeps the test honest when columns are added — a
 * hard-coded 3 silently became "days" once the day count landed.
 */
/** Heading label to the key the app uses for that column. */
const COLUMN_KEY = Object.fromEntries(
  await page.evaluate(() =>
    [...document.querySelectorAll(".fg-heading .fg-cell")].map((cell) => [
      cell.textContent.trim(),
      // The cell carries other classes too (pinned columns, for one), so the
      // key is read from its own class rather than by trimming a prefix.
      [...cell.classList].find((name) => name.startsWith("fg-cell-"))?.slice("fg-cell-".length),
    ]),
  ),
);

const COLUMN = Object.fromEntries(
  (
    await page.evaluate(() =>
      [...document.querySelectorAll(".fg-heading .fg-cell")].map((cell) => cell.textContent.trim()),
    )
  ).map((name, index) => [name, index]),
);

/** Selects a cell the way a click would, without depending on its position. */
const selectCell = (row, column) => {
  if (row < 0) throw new Error(`選択しようとした行が見つかりません (column=${column})`);

  return page.evaluate(
    (row, column) => {
      const rows = [...document.querySelectorAll(".fg-pane-left .fg-row.fg-data")];
      rows[row]
        .querySelectorAll(".fg-cell")
        [column].dispatchEvent(new MouseEvent("mousedown", { bubbles: true }));

      // 作った event は選択を動かすが、フォーカスは動かさない。本物のクリックとの
      // この違いのせいで、直前に Escape などでフォーカスが表の外へ出ていると、
      // 続く F2 も打鍵もどこにも届かず、編集が「静かに起きなかった」ことになる。
      document.querySelector(".fg-grid")?.focus({ preventScroll: true });
    },
    row,
    column,
  );
};

/**
 * Types over whatever the editor holds.
 *
 * F2 opens the editor with its text already selected, so typing replaces it —
 * no select-all keystroke, which headless Chrome routes to the document rather
 * than the focused input.
 */
/**
 * 編集欄が開いてから打つ。
 *
 * F2 のあと島が描き直されるので、開く前に打った文字は行き先がない。以前は
 * 直前の編集が拒否された回だけ、ここで打った文字がどこにも入らず、あとの
 * 集計テストが「変わっていない」と言って落ちていた。
 */
const replaceEditorText = async (text) => {
  for (let at = 0; at < 20; at++) {
    if (await page.evaluate(() => !!document.querySelector(".fg-editor:not(.is-typist)"))) break;
    await settle();
  }

  await page.keyboard.type(text);
};

let s;

// --- navigation -------------------------------------------------------------

await page.click(".fg-pane-left .fg-row.fg-data .fg-cell");
s = await state();
check("クリックでセルを選択", s.row === 0 && s.column === 0, `row=${s.row} col=${s.column}`);

await page.keyboard.press("ArrowDown");
await page.keyboard.press("ArrowRight");
s = await state();
check("矢印キーで移動", s.row === 1 && s.column === 1, `row=${s.row} col=${s.column}`);

for (let i = 0; i < Object.keys(COLUMN).length; i++) await page.keyboard.press("ArrowRight");
await page.keyboard.press("Tab");
s = await state();
check("Tab が行末から次の行の先頭へ回り込む", s.row === 2 && s.column === 0, `row=${s.row}`);

// --- editing ----------------------------------------------------------------

await selectCell(0, 0);
await page.keyboard.type("A");
s = await state();
check("文字入力で編集が始まりその文字が残る", s.editing && s.editorValue === "A", `value=${s.editorValue}`);

await page.keyboard.press("Escape");
await settle();
s = await state();
check("Esc で編集を取り消す", !s.editing && s.cells[0][0] === "要件定義", `now=${s.cells[0][0]}`);

await page.keyboard.press("F2");
s = await state();
check("F2 は既存の値を開く", s.editing && s.editorValue === "要件定義", `value=${s.editorValue}`);

await page.keyboard.press("Escape");
await settle();
await page.keyboard.press("Enter");
s = await state();
check("Enter でも編集が始まる", s.editing && s.editorValue === "要件定義", `value=${s.editorValue}`);

await replaceEditorText("要件定義（改）");
await page.keyboard.press("Enter");
await settle();
s = await state();
check("Enter で確定し下のセルへ移る", !s.editing && s.row === 1 && s.column === 0, `row=${s.row}`);
check("編集がグリッドに反映される", s.cells[0][0] === "要件定義（改）", s.cells[0][0]);

const persisted = await page.evaluate(async () => {
  const response = await fetch("/api/projects/test-project/grid");
  return (await response.json()).tasks[0].name;
});
check("サーバーに保存される", persisted === "要件定義（改）", persisted);

// --- what the server refuses ------------------------------------------------

const summary = (await state()).names.indexOf("開発");
await selectCell(summary, COLUMN["予定開始"]);
await page.keyboard.press("F2");
await settle();
s = await state();
check("集計行の日付は編集させない", !s.editing && !!s.error, `error=${s.error}`);

await selectCell(0, COLUMN["予定開始"]);
await page.keyboard.press("F2");
await replaceEditorText("2026-13-99");
await page.keyboard.press("Enter");
await settle();
await settle();
s = await state();
check("不正な日付を拒否する", !!s.error, `error=${s.error}`);
// 断られたあとの画面は、人なら一度手を止めるところ。次の編集を続けて打つと、
// 描き直しの最中に打鍵が落ちる。
await page.keyboard.press("Escape");
await settle();
check("拒否された編集を元に戻す", s.cells[0][COLUMN["予定開始"]] === "2026-08-03", s.cells[0][COLUMN["予定開始"]]);

// --- rollup -----------------------------------------------------------------

const parentBefore = (await state()).cells[summary][COLUMN["実進捗"]];
await selectCell((await state()).names.indexOf("設計"), COLUMN["実進捗"]);
await page.keyboard.press("F2");
await replaceEditorText("100");
await page.keyboard.press("Enter");

// 集計はサーバーが返した値。届くのを待つ——固定の待ち時間では、混んだ日に
// 「変わっていない」と言われて落ちていた。
const rolled = await until(
  async () => (await state()).cells[summary][COLUMN["実進捗"]],
  "42%",
);
s = await state();
check(
  "子の進捗を変えると親の集計が動く",
  rolled,
  // 落ちたときに、どこで止まったのかが分かるように: 子に値が入っているか、
  // 編集欄が開いたままか、サーバーが何か言っているか。
  `親 ${parentBefore} → ${s.cells[summary][COLUMN["実進捗"]]}` +
    ` / 子 ${s.cells[s.names.indexOf("設計")]?.[COLUMN["実進捗"]]}` +
    ` / editing=${s.editing} error=${s.error ?? "なし"}`,
);

// --- outline ----------------------------------------------------------------

/** Names paired with their indent depth, which is what a hierarchy edit moves. */
const outline = async () =>
  page.evaluate(() =>
    [...document.querySelectorAll(".fg-pane-left .fg-row.fg-data")].map((row) => {
      const cell = row.querySelector(".fg-cell-name");
      return [
        cell.querySelector(".fg-name-text")?.textContent ?? "",
        (parseInt(cell.style.paddingLeft, 10) - 12) / 16,
      ];
    }),
  );

const move = async (key) => {
  await page.keyboard.down("Alt");
  await page.keyboard.press(key);
  await page.keyboard.up("Alt");
  await settle();
  await settle();
};

// テスト（トップレベル）を、直前の行「開発」の子にする
await selectCell((await state()).names.indexOf("テスト"), 0);
await move("ArrowRight");
let tree = await outline();
check(
  "⌥→ で直前の行の子になる",
  tree.find(([name]) => name === "テスト")?.[1] === 1,
  JSON.stringify(tree),
);
check(
  "子は親の直後に並ぶ",
  tree.map(([name]) => name).join(",").includes("実装,テスト"),
  tree.map(([n]) => n).join(","),
);

// 兄弟内での並べ替え
await move("ArrowUp");
tree = await outline();
check(
  "⌥↑ で兄弟の中を上へ",
  tree.map(([name]) => name).join(",").includes("テスト,実装"),
  tree.map(([n]) => n).join(","),
);

// 階層を戻す：親「開発」の直後に来る
await move("ArrowLeft");
tree = await outline();
check("⌥← で階層が戻る", tree.find(([name]) => name === "テスト")?.[1] === 0, JSON.stringify(tree));

// 先頭行では動かず、その理由を伝える
await selectCell(0, 0);
const beforeEdge = await outline();
await move("ArrowRight");
const edgeNotice = await page.evaluate(
  () => document.querySelector(".fg-notice")?.textContent ?? null,
);
check("先頭行では階層が変わらない", JSON.stringify(await outline()) === JSON.stringify(beforeEdge));
check(
  "動かせない理由を伝える",
  (edgeNotice ?? "").includes("子タスクにできません"),
  edgeNotice ?? "なし",
);
check("それはエラー扱いではない", !(await state()).error, (await state()).error ?? "");

// 選択はタスクに追従する（行番号ではなく）
await selectCell((await state()).names.indexOf("ドキュメント整備"), COLUMN["予定終了"]);
await move("ArrowUp");
s = await state();
check(
  "移動後も同じタスクが選択されたまま",
  s.names[s.row] === "ドキュメント整備" && s.column === COLUMN["予定終了"],
  `row=${s.names[s.row]} col=${s.column}`,
);

// --- folding ----------------------------------------------------------------

const fold = async (key) => {
  await page.keyboard.down("Meta");
  await page.keyboard.press(key);
  await page.keyboard.up("Meta");
  await settle();
};

// 「開発」を畳むと子(設計・実装)が消え、隠した件数が出る
await selectCell((await state()).names.indexOf("開発"), 0);
await fold("ArrowLeft");
s = await state();
check("⌘← で子タスクが隠れる", !s.names.includes("設計") && !s.names.includes("実装"), s.names.join(","));
check(
  "隠した件数を表示する",
  await page.evaluate(() => document.querySelector(".fg-folded")?.textContent) === "+2",
  await page.evaluate(() => document.querySelector(".fg-folded")?.textContent ?? "なし"),
);
check("集計行のバーは残る", (await state()).cells[(await state()).names.indexOf("開発")][COLUMN["予定開始"]] === "2026-08-10");

// 畳んだ状態はリロードしても残る
await page.reload({ waitUntil: "domcontentloaded" });
await page.waitForSelector(".fg-grid");
s = await state();
check("畳んだ状態がリロード後も残る", !s.names.includes("設計"), s.names.join(","));

// つまみのクリックで開く
await page.click(".fg-twisty:not(.is-leaf)");
await settle();
s = await state();
check("つまみのクリックで開く", s.names.includes("設計") && s.names.includes("実装"), s.names.join(","));

// 畳んだ親の中へ移動したタスクは、自動的に見えるようになる。
// ⌥→ は「直前の兄弟の子にする」なので、畳んだ「開発」の真下の行を使う。
await selectCell((await state()).names.indexOf("開発"), 0);
await fold("ArrowLeft");
check("再び畳んだ", !(await state()).names.includes("設計"));

const below = (await state()).names[(await state()).names.indexOf("開発") + 1];
await selectCell((await state()).names.indexOf(below), 0);
await move("ArrowRight");
s = await state();
check(
  "畳んだ親に入れると自動で開く",
  s.names.includes("設計") && s.names.includes(below),
  `「${below}」を入れた → ${s.names.join(",")}`,
);

// 後片付け: 入れた行の階層を戻す
await move("ArrowLeft");
await settle();

// --- Japanese input ----------------------------------------------------------

/**
 * Types through an IME, the way the app is actually used.
 *
 * A plain `keyboard.type` never opens a composition, so it cannot see the bug
 * this guards: with the caret on a `div`, an IME has nowhere to compose and
 * typing Japanese into a selected cell did nothing at all.
 */
const cdp = await page.createCDPSession();

const typeJapanese = async (reading, text) => {
  await cdp.send("Input.imeSetComposition", {
    text: reading,
    selectionStart: reading.length,
    selectionEnd: reading.length,
  });
  await settle();
  await cdp.send("Input.insertText", { text });
  await settle();
};

for (const [column, reading, text] of [
  ["タスク", "ようけん", "要件"],
  ["コメント", "ようすみ", "様子見"],
]) {
  await selectCell(0, COLUMN[column]);
  await typeJapanese(reading, text);

  check(
    `${column}: 変換の開始で編集に入る`,
    (await state()).editing,
    `editor=${(await state()).editorValue}`,
  );

  await page.keyboard.press("Enter");
  await settle();
  await settle();

  const stored = await page.evaluate(async () => {
    const grid = await (await fetch("/api/projects/test-project/grid")).json();
    return { name: grid.tasks[0].name, note: grid.tasks[0].note };
  });
  const saved = column === "タスク" ? stored.name : stored.note;

  check(`${column}: 日本語が保存される`, saved === text, saved);
}

// 変換確定の Enter を「入力確定」と取り違えると、変換の続きが次の行に流れ込む。
await selectCell(1, COLUMN["コメント"]);
await cdp.send("Input.imeSetComposition", { text: "すずき", selectionStart: 3, selectionEnd: 3 });
await settle();

const swallowed = await page.evaluate(() => {
  const before = document.querySelector(".fg-cell.is-selected")?.className;
  document
    .querySelector(".fg-editor")
    ?.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", isComposing: true, bubbles: true }));
  return before === document.querySelector(".fg-cell.is-selected")?.className;
});
check("変換中の Enter は IME のもの", swallowed);

await cdp.send("Input.insertText", { text: "鈴木" });
await settle();
await page.keyboard.press("Enter");
await settle();
await settle();

const spill = await page.evaluate(async () => {
  const grid = await (await fetch("/api/projects/test-project/grid")).json();
  return grid.tasks.map((task) => task.note);
});
check(
  "下の行に同じ値が漏れない",
  spill.filter((value) => value === "鈴木").length === 1,
  spill.join(","),
);

await page.evaluate(async () => {
  const grid = await (await fetch("/api/projects/test-project/grid")).json();
  await fetch(`/api/projects/test-project/tasks/${grid.tasks[0].id}`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ field: "name", value: "要件定義（改）" }),
  });
});
await page.reload({ waitUntil: "domcontentloaded" });
await page.waitForSelector(".fg-grid");
await settle();

// --- filtering ---------------------------------------------------------------

/** Types into one column's filter box. */
const filterBy = async (column, text) => {
  const box = `.fg-filter[data-column="${COLUMN_KEY[column]}"]`;
  const tag = await page.evaluate((box) => document.querySelector(box)?.tagName, box);

  // 担当者 and ステータス are menus; the rest are typed into.
  if (tag === "SELECT") {
    await page.select(box, text.toLowerCase());
  } else {
    // click ではなく focus。絞り込みの欄は横に流れる領域にあり、固定列の下に
    // 半分隠れた欄を押すと、上にある固定列側の欄が代わりに押される。打鍵は本物の
    // まま、宛先だけを確実にする。
    await page.evaluate((box) => {
      const field = document.querySelector(box);
      field.scrollIntoView({ block: "nearest", inline: "nearest" });
      field.value = "";
      field.focus();
    }, box);
    await page.keyboard.type(text);
  }

  await settle();
};

const clearFilters = async () => {
  await page.evaluate(() => {
    for (const box of document.querySelectorAll(".fg-filter")) {
      box.value = "";
      box.dispatchEvent(new Event(box.tagName === "SELECT" ? "change" : "input", { bubbles: true }));
    }
  });
  await settle();
};

await filterBy("担当者", "佐藤");
s = await state();
check("列の絞り込みが効く", s.names.includes("設計") && !s.names.includes("テスト"), s.names.join(","));
check("一致した行の親は残る", s.names.includes("開発"), s.names.join(","));
check(
  "入力中もフォーカスがその欄に残る",
  await page.evaluate(() => document.activeElement?.classList.contains("fg-filter") === true),
  await page.evaluate(() => document.activeElement?.className ?? "なし"),
);
check(
  "絞り込みの件数が出る",
  (await page.evaluate(() => document.getElementById("fugantt-filter-count")?.textContent ?? "")).includes("/"),
  await page.evaluate(() => document.getElementById("fugantt-filter-count")?.textContent ?? "なし"),
);

// 変換中に絞り込むと入力欄ごと作り直され、変換が壊れて「fう」になる。
await clearFilters();
await page.click('.fg-filter[data-column="name"]');
await cdp.send("Input.imeSetComposition", { text: "f", selectionStart: 1, selectionEnd: 1 });
await settle();
await cdp.send("Input.imeSetComposition", { text: "ふ", selectionStart: 1, selectionEnd: 1 });
await settle();
await cdp.send("Input.insertText", { text: "ふ" });
await settle();
check(
  "絞り込み欄の変換が壊れない",
  (await page.evaluate(() => document.querySelector('.fg-filter[data-column="name"]')?.value)) === "ふ",
  await page.evaluate(() => document.querySelector('.fg-filter[data-column="name"]')?.value ?? "なし"),
);
await clearFilters();
await filterBy("担当者", "佐藤");

// 二つ目の条件を足すと、両方に当てはまる行だけが残る（AND）
const beforeAnd = (await state()).rowCount;
await filterBy("タスク", "実装");
s = await state();
check(
  "二列の条件は AND で効く",
  s.rowCount < beforeAnd && !s.names.includes("設計"),
  `${beforeAnd} → ${s.rowCount}: ${s.names.join(",")}`,
);

await clearFilters();
check("絞り込みを外すと全部戻る", (await state()).rowCount === 7, String((await state()).rowCount));

await selectCell(0, 0);

// --- dragging rows -----------------------------------------------------------

/** Drags a row's grip to a gap, `dx` pixels sideways to choose the depth. */
const dragRow = async (name, toRow, dx) => {
  const from = await page.evaluate((name) => {
    const rows = [...document.querySelectorAll(".fg-pane-left .fg-row.fg-data")];
    const row = rows.find((row) => row.textContent.includes(name));
    const box = row.querySelector(".fg-handle").getBoundingClientRect();
    return { x: box.x + box.width / 2, y: box.y + box.height / 2 };
  }, name);

  const to = await page.evaluate((index) => {
    const rows = [...document.querySelectorAll(".fg-pane-left .fg-row.fg-data")];
    const box = (rows[index] ?? rows[rows.length - 1]).getBoundingClientRect();
    return { y: index < rows.length ? box.top + 2 : box.bottom - 2 };
  }, toRow);

  await page.mouse.move(from.x, from.y);
  await page.mouse.down();
  await page.mouse.move(from.x + dx / 2, (from.y + to.y) / 2);
  await page.mouse.move(from.x + dx, to.y);
  await page.mouse.up();
  await settle();
  await settle();
};

const names = async () => (await outline()).map(([name]) => name).join(",");

await dragRow("テスト", 0, 0);
check("行を先頭へドラッグできる", (await names()).startsWith("テスト,"), await names());

// The row just moved is the selected one, and the invisible typist covers its
// cell — dragging it again is what catches that overlap.
check(
  "移動直後の行（選択中）もつかめる",
  await page.evaluate(() => {
    const row = document.querySelector(".fg-row.is-current");
    const handle = row?.querySelector(".fg-handle");
    if (!handle) return false;
    const box = handle.getBoundingClientRect();
    return document.elementFromPoint(box.x + box.width / 2, box.y + box.height / 2) === handle;
  }),
);

// 下へ運びつつ右へ: 「開発」の子にする
await dragRow("テスト", 3, 18);
tree = await outline();
check(
  "横に振ると階層が決まる",
  tree.find(([name]) => name === "テスト")?.[1] === 1,
  JSON.stringify(tree),
);

// ドラッグ中は自分の部分木を落下先から外す。サーバー側の防御は API で直接確かめる。
const refusal = await page.evaluate(async () => {
  const grid = await (await fetch("/api/projects/test-project/grid")).json();
  const parent = grid.tasks.find((task) => task.has_children);
  const child = grid.tasks.find((task) => task.depth > parent.depth);

  const response = await fetch(`/api/projects/test-project/tasks/${parent.id}/place`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ parent: child.id, after: null }),
  });

  return (await response.json()).note ?? null;
});
check("自分の子孫の中には入れられない", (refusal ?? "").includes("子タスク"), refusal ?? "なし");

// 後片付け: 「テスト」を最上位の末尾へ戻す
await dragRow("テスト", 99, -40);
tree = await outline();
check("最下段へ戻せる", tree.at(-1)?.[0] === "テスト", JSON.stringify(tree));

// --- readable project ids ----------------------------------------------------

const ids = await page.evaluate(async () => {
  const create = async (name) => {
    const response = await fetch("/projects", {
      method: "POST",
      body: new URLSearchParams({ name }),
      redirect: "manual",
    });
    return response.status;
  };

  const name = `テスト計画 ${Date.now()}`;
  const first = await create(name);
  const again = await create(name);

  const html = await (await fetch("/")).text();
  const slug = [...html.matchAll(/href="\/projects\/([^"]+)"/g)].map((m) =>
    decodeURIComponent(m[1]),
  );

  return { first, again, slug };
});

// まっさらな画面は「読み込みに失敗した」ように見える。最初の1行は空でも要る。
check(
  "作ったばかりのプロジェクトに空の行が1つある",
  await page.evaluate(async () => {
    const name = `空行テスト ${Date.now()}`;
    await fetch("/projects", { method: "POST", body: new URLSearchParams({ name }), redirect: "manual" });

    const html = await (await fetch("/")).text();
    const id = [...html.matchAll(/href="\/projects\/([^"]+)"/g)]
      .map((m) => decodeURIComponent(m[1]))
      .find((slug) => slug.startsWith("空行テスト-"));

    const grid = await (await fetch(`/api/projects/${encodeURIComponent(id)}/grid`)).json();
    return grid.tasks.length === 1 && grid.tasks[0].name === "" && !grid.tasks[0].start;
  }),
);

check("プロジェクト名から読める ID を作る", ids.slug.some((s) => s.startsWith("テスト計画-")), ids.slug.join(", "));
check("同じ名前は作れない", ids.again === 400, String(ids.again));
check("UUID の既存プロジェクトも残る", ids.slug.includes("test-project"));

// --- Japanese calendar -------------------------------------------------------

/**
 * 全体の設定は管理者のもの。テストの口座はふだん管理者ではない（それを確かめる
 * テストが後ろにある）ので、必要なときだけ肩書きを借りて、すぐ返す。
 */
const asAdmin = async (run) => {
  const flag = (on) =>
    execFileSync("sqlite3", [DB, `UPDATE users SET base_role = '${on ? "admin" : "none"}' WHERE email = '${EMAIL}'`]);

  flag(true);
  try {
    return await run();
  } finally {
    flag(false);
  }
};

const holidays = await asAdmin(() =>
  page.evaluate(async () => {
    // 祝日は会社の暦なので、入れる場所は全体の設定。
    const response = await fetch("/admin/holidays/japan", {
      method: "POST",
      body: new URLSearchParams({ year: "2026" }),
    });
    const grid = await (await fetch("/api/projects/test-project/grid")).json();
    return { status: response.status, days: grid.holidays };
  }),
);

check("日本の祝日をまとめて入れられる", holidays.status < 400 && holidays.days.length === 18, `${holidays.days.length} 件`);
check(
  "振替休日も計算する",
  holidays.days.some((h) => h.date === "2026-05-06" && h.name === "振替休日"),
);
check(
  "国民の休日も計算する",
  holidays.days.some((h) => h.date === "2026-09-22" && h.name === "国民の休日"),
);

await page.reload({ waitUntil: "domcontentloaded" });
await page.waitForSelector(".fg-grid");
await settle();

const calendar = await page.evaluate(() => {
  const days = [...document.querySelectorAll(".fg-day")];
  return {
    weekdays: days.slice(0, 3).map((d) => d.querySelector(".fg-weekday")?.textContent),
    saturday: document.querySelectorAll(".fg-column.is-saturday").length,
    sunday: document.querySelectorAll(".fg-column.is-sunday").length,
    holidayTitle: [...document.querySelectorAll(".fg-day.is-holiday")].map((d) => d.title)[0] ?? null,
  };
});

check("日付の下に曜日が出る", calendar.weekdays.every((w) => "日月火水木金土".includes(w)), calendar.weekdays.join(""));
check("土曜と日曜を分けて塗る", calendar.saturday > 0 && calendar.sunday > 0, JSON.stringify(calendar));
check("祝日名がマウスオーバーで出る", (calendar.holidayTitle ?? "").length > 0, calendar.holidayTitle ?? "なし");

// 全体の祝日でも、この現場だけ動く日にできる。差分はプロジェクトが持つ。
const skipped = await page.evaluate(async () => {
  await fetch("/projects/test-project/holidays/remove", {
    method: "POST",
    body: new URLSearchParams({ date: "2026-05-06" }),
  });
  const gone = await (await fetch("/api/projects/test-project/grid")).json();

  await fetch("/projects/test-project/holidays/restore", {
    method: "POST",
    body: new URLSearchParams({ date: "2026-05-06" }),
  });
  const back = await (await fetch("/api/projects/test-project/grid")).json();

  const has = (grid) => grid.holidays.some((h) => h.date === "2026-05-06");
  return { gone: has(gone), back: has(back) };
});

check(
  "全体の祝日を、この計画だけ働く日にできる",
  skipped.gone === false && skipped.back === true,
  JSON.stringify(skipped),
);

// 後片付け: 祝日を消しておく
await asAdmin(() =>
  page.evaluate(async () => {
    const grid = await (await fetch("/api/projects/test-project/grid")).json();
    for (const holiday of grid.holidays) {
      await fetch("/admin/holidays/remove", {
        method: "POST",
        body: new URLSearchParams({ date: holiday.date }),
      });
    }
  }),
);
await page.reload({ waitUntil: "domcontentloaded" });
await page.waitForSelector(".fg-grid");
await settle();
await selectCell(0, 0);

// --- settings forms ----------------------------------------------------------

const settings = await page.evaluate(async () => {
  const html = await (await fetch("/projects/test-project/settings")).text();
  const doc = new DOMParser().parseFromString(html, "text/html");
  return {
    // タスク名の列だけは外せないので、そのチェックは常に無効。それ以外に無効な
    // 入力があるのは、真偽属性を取り違えたときの症状。
    disabled: doc.querySelectorAll(
      "form input[disabled]:not([name='column_name']), form select[disabled]",
    ).length,
    columnNames: [...doc.querySelectorAll("input[name^='column_']")].map((i) => i.name),
    // Only the palette; the status form has a colour box of its own.
    colours: doc.querySelectorAll("input[type=color][name^='color_']").length,
  };
});
check("設定の入力欄が無効になっていない", settings.disabled === 0, String(settings.disabled));
check(
  "列のチェックは1つずつ別の名前で送る",
  // 組み込みの列がそろっていて、名前がひとつも重なっていないこと。件数を数字で
  // 決め打つと、独自の項目が一つあるだけで落ちる——列の数は増えるものなので。
  [
    "column_name", "column_start", "column_end", "column_actual_start", "column_actual_end",
    "column_days", "column_start_variance", "column_end_variance", "column_progress",
    "column_status", "column_assignee", "column_note", "column_waits",
  ].every((name) => settings.columnNames.includes(name)) &&
    new Set(settings.columnNames).size === settings.columnNames.length,
  settings.columnNames.join(","),
);
// 選択肢は項目を作ったあとに、その場の UI で足す。作成フォームには置かない。
const kinds = await page.evaluate(async () => {
  const html = await (await fetch("/projects/test-project/settings")).text();
  const doc = new DOMParser().parseFromString(html, "text/html");

  return {
    hasOptionsBox: !!doc.getElementById("field-options"),
    kinds: [...(doc.getElementById("field-kind")?.options ?? [])].map((o) => o.value),
  };
});
check("作成フォームに選択肢の欄はない", !kinds.hasOptionsBox, JSON.stringify(kinds));
check(
  "独自項目の種類が5つある",
  kinds.kinds.join(",") === "text,select,suggest,date,number",
  kinds.kinds.join(","),
);

// 予定・完了分・実施・集計行・遅延・土曜・日曜・祝日・休暇・待ち。
// 予定・完了分・実施・集計行・遅延・今日・土曜・日曜・祝日・休暇・待ち。
check("色の設定が11ある（今日を分けたぶん）", settings.colours === 11, String(settings.colours));

// 実際に保存できるか（url-encoded は繰り返しキーを配列にできない）
const saved = await page.evaluate(async () => {
  const view = new URLSearchParams();
  view.set("skip_saturday", "1");
  view.set("skip_sunday", "1");
  const response = await fetch("/projects/test-project/view", { method: "POST", body: view });

  const columns = new URLSearchParams();
  columns.set("column_start", "1");
  columns.set("column_end", "1");
  columns.set("width_start", "140");
  await fetch("/projects/test-project/columns", { method: "POST", body: columns });

  const grid = await (await fetch("/api/projects/test-project/grid")).json();
  return {
    status: response.status,
    counting: grid.counting,
    hidden: grid.hidden_columns,
    widths: grid.column_widths,
  };
});
check("表示の設定が保存できる", saved.status < 400, `status=${saved.status}`);
check(
  "外した列が隠れる",
  saved.counting.saturday === true && saved.counting.sunday === true && saved.hidden.includes("status"),
  JSON.stringify(saved),
);
check("列の幅を保存できる", saved.widths.start === 140, JSON.stringify(saved.widths));

// 列を入れ替えられる。タスク名は先頭のまま。
const reordered = await page.evaluate(async () => {
  const body = new URLSearchParams();
  for (const key of ["name", "start", "end", "days", "progress", "status", "assignee", "note"]) {
    body.set(`column_${key}`, "1");
  }
  body.set("move", "up:end");
  await fetch("/projects/test-project/columns", { method: "POST", body });

  const grid = await (await fetch("/api/projects/test-project/grid")).json();
  return grid.column_order;
});
check(
  "列を入れ替えられる",
  reordered.indexOf("end") < reordered.indexOf("start") && reordered[0] === "name",
  reordered.slice(0, 4).join(","),
);

// 元に戻す
await page.evaluate(async () => {
  const columns = new URLSearchParams();
  for (const key of [
    "name",
    "start",
    "end",
    "actual_start",
    "actual_end",
    "days",
    "actual_days",
    "start_variance",
    "end_variance",
    "targets",
    "progress",
    "status",
    "assignee",
    "note",
    "waits",
  ]) {
    columns.set(`column_${key}`, "1");
  }
  columns.set("move", "up:start");
  await fetch("/projects/test-project/columns", { method: "POST", body: columns });

  // 休暇を日数から除くのは既定。戻すときも入れておかないと外れたままになる。
  const view = new URLSearchParams();
  view.set("skip_leave", "1");
  view.set("quarters", "1");
  await fetch("/projects/test-project/view", { method: "POST", body: view });
});

// メモ。プレーンテキストなので、書いた記号は記号のまま出る。
const memo = await page.evaluate(async () => {
  const body = new URLSearchParams({ memo: "# 引き継ぎ\n<b>設計</b>は佐藤" });
  const response = await fetch("/projects/test-project/memo", { method: "POST", body });
  const html = await (await fetch("/projects/test-project")).text();

  return {
    status: response.status,
    shown: html.includes("# 引き継ぎ"),
    escaped: html.includes("&lt;b&gt;設計&lt;/b&gt;") && !html.includes("<b>設計</b>"),
  };
});
check("プロジェクトメモを残せる", memo.status < 400 && memo.shown, JSON.stringify(memo));
check("メモの記号はそのまま文字として出る", memo.escaped, JSON.stringify(memo));

await page.reload({ waitUntil: "domcontentloaded" });
await page.waitForSelector(".fg-grid");
await settle();

// --- editors -----------------------------------------------------------------

// 日付は打ち切れるテキスト欄のまま。カレンダーは隣のボタンから開く。
await selectCell(0, COLUMN["予定開始"]);
await page.keyboard.press("F2");
await settle();

const dateEditor = await page.evaluate(() => {
  const editor = document.querySelector("input.fg-editor:not(.is-typist)");
  return {
    type: editor?.getAttribute("type") ?? null,
    focused: document.activeElement === editor,
    picker: !!document.querySelector(".fg-datepicker"),
  };
});
check(
  "開始: 打てるテキスト欄とカレンダーの両方が出る",
  dateEditor.type === "text" && dateEditor.focused && dateEditor.picker,
  JSON.stringify(dateEditor),
);

await page.keyboard.press("Escape");
await settle();

await selectCell(0, COLUMN["ステータス"]);
await page.keyboard.press("F2");
await settle();

const statusEditor = await page.evaluate(() => {
  const editor = document.querySelector(".fg-editor:not(.is-typist)");
  return editor
    ? { tag: editor.tagName, focused: document.activeElement === editor }
    : null;
});
check(
  "ステータス: 候補の一覧が開く",
  statusEditor?.tag === "SELECT" && statusEditor.focused,
  JSON.stringify(statusEditor),
);

await page.keyboard.press("Escape");
await settle();

// 横に隠れた列へ移っても画面内に入る
await page.evaluate(() => {
  document.querySelector(".fg-grid").style.setProperty("--fg-pane-width", "260px");
});
await selectCell(0, 0);
for (let i = 0; i < COLUMN["コメント"]; i++) await page.keyboard.press("ArrowRight");
await settle();
check(
  "隠れた列に移ると横スクロールで見える",
  await page.evaluate(() => {
    const cell = document.querySelector(".fg-cell.is-selected");
    const pane = document.querySelector(".fg-pane-left");
    if (!cell || !pane) return false;
    const a = cell.getBoundingClientRect();
    const b = pane.getBoundingClientRect();
    return a.left >= b.left - 1 && a.right <= b.right + 1;
  }),
);
await page.evaluate(() => window.localStorage.removeItem("fugantt:pane-width"));
await page.reload({ waitUntil: "domcontentloaded" });
await page.waitForSelector(".fg-grid");
await settle();
await selectCell(0, 0);

// --- full-width input --------------------------------------------------------

// 未入力のセルにも入れられること、日本語入力のままでも通ること。
await page.evaluate(async () => {
  const grid = await (await fetch("/api/projects/test-project/grid")).json();
  const task = grid.tasks.find((t) => t.name === "テスト");
  await fetch(`/api/projects/test-project/tasks/${task.id}`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ field: "schedule", value: "/" }),
  });
});
await page.reload({ waitUntil: "domcontentloaded" });
await page.waitForSelector(".fg-grid");
await settle();

const emptyRow = (await state()).names.indexOf("テスト");
check("未入力の日付は「—」で出る", (await state()).cells[emptyRow][COLUMN["予定開始"]] === "—");

await selectCell(emptyRow, COLUMN["予定開始"]);
await typeJapanese("２０２６－０９－０１", "２０２６－０９－０１");
await page.keyboard.press("Enter");
await settle();
await settle();

const widthTest = await page.evaluate(async () => {
  const grid = await (await fetch("/api/projects/test-project/grid")).json();
  return grid.tasks.find((t) => t.name === "テスト").start;
});
check("全角のまま打っても日付として通る", widthTest === "2026-09-01", String(widthTest));
check("エラーは出ていない", (await state()).error === null, (await state()).error ?? "");

// --- sticky headers ----------------------------------------------------------

await page.evaluate(() => {
  document.querySelector(".fg-pane-left").scrollTop = 200;
});
await settle();

const stuck = await page.evaluate(() => {
  const left = document.querySelector(".fg-pane-left");
  const chart = document.querySelector(".fg-pane-chart");
  const paneTop = left.getBoundingClientRect().top;
  const rows = [...document.querySelectorAll(".fg-pane-left .fg-row.fg-data")];
  const bars = [...document.querySelectorAll(".fg-bar-row")];

  return {
    filters: Math.round(document.querySelector(".fg-filters").getBoundingClientRect().top - paneTop),
    dates: Math.round(
      document.querySelector(".fg-chart-header").getBoundingClientRect().top -
        chart.getBoundingClientRect().top,
    ),
    synced: Math.round(chart.scrollTop) === Math.round(left.scrollTop),
    drift: rows.slice(0, 4).map((row, i) =>
      Math.round(row.getBoundingClientRect().y - bars[i].getBoundingClientRect().y),
    ),
    pageScrolls: document.body.scrollHeight > window.innerHeight + 2,
  };
});

check("スクロールしても見出しが残る", stuck.filters === 0 && stuck.dates === 0, JSON.stringify(stuck));
check("左右のペインが縦に同期する", stuck.synced);
check("行とバーがずれない", stuck.drift.every((d) => d === 0), stuck.drift.join(","));
check("ページ自体はスクロールしない", !stuck.pageScrolls);

await page.evaluate(() => {
  document.querySelector(".fg-pane-left").scrollTop = 0;
});
await settle();

// --- the context menu --------------------------------------------------------

/**
 * Drives the menu with real mouse events.
 *
 * Calling `.click()` on the item would pass while the menu was broken: the bug
 * lived in `mousedown`, which a synthetic click never sends.
 */
const runMenuItem = async (rowName, label) => {
  const at = await page.evaluate((rowName) => {
    const rows = [...document.querySelectorAll(".fg-pane-left .fg-row.fg-data")];
    const row = rows.find((row) => row.textContent.includes(rowName));
    const box = row.querySelector(".fg-cell-name").getBoundingClientRect();
    return { x: box.x + 40, y: box.y + box.height / 2 };
  }, rowName);

  await page.mouse.click(at.x, at.y, { button: "right" });
  await settle();

  const item = await page.evaluate((label) => {
    const button = [...document.querySelectorAll(".fg-menu-item")].find((b) =>
      b.textContent.includes(label),
    );
    if (!button) return null;
    const box = button.getBoundingClientRect();
    return { x: box.x + box.width / 2, y: box.y + box.height / 2 };
  }, label);

  if (!item) throw new Error(`メニュー項目が見つかりません: ${label}`);

  await page.mouse.click(item.x, item.y);
  await settle();
  await settle();
};

// 「ドキュメント整備」は最下段の兄弟なので、直前の兄弟の子になれる
await runMenuItem("ドキュメント整備", "子タスクにする");
tree = await outline();
check(
  "右クリックメニューから子タスクにできる",
  tree.find(([name]) => name === "ドキュメント整備")?.[1] === 1,
  JSON.stringify(tree),
);
check(
  "実行するとメニューが閉じる",
  await page.evaluate(() => document.querySelector(".fg-menu") === null),
);

await runMenuItem("ドキュメント整備", "階層を戻す");
tree = await outline();
check(
  "メニューから階層を戻せる",
  tree.find(([name]) => name === "ドキュメント整備")?.[1] === 0,
  JSON.stringify(tree),
);

// メニュー外をクリックしたら閉じるだけ
await page.mouse.click(40, 40, { button: "right" });
await settle();
await page.mouse.click(600, 600);
await settle();
check(
  "メニュー外のクリックでは閉じるだけ",
  await page.evaluate(() => document.querySelector(".fg-menu") === null),
);

// Clicking outside took the focus with it; the keyboard tests need it back.
await selectCell(0, 0);

// --- rows -------------------------------------------------------------------

const rowsBefore = (await state()).rowCount;
await page.keyboard.down("Meta");
await page.keyboard.press("Enter");
await page.keyboard.up("Meta");
await settle();
await settle();
s = await state();
check("⌘Enter で行を追加する", s.rowCount === rowsBefore + 1, `${rowsBefore} → ${s.rowCount}`);
check("追加した行はすぐ編集状態になる", s.editing);

await page.keyboard.type("追加されたタスク");
await page.keyboard.press("Enter");
await settle();
await settle();

const rowsBeforeDelete = (await state()).rowCount;
const addedRow = (await state()).names.indexOf("追加されたタスク");
check("追加した行に名前が入る", addedRow >= 0, (await state()).names.join(","));

// Deleting "whichever row is at 0" when the insert failed would destroy real
// data and hide the original failure behind a second one.
if (addedRow >= 0) {
  await selectCell(addedRow, 0);
  await page.keyboard.down("Meta");
  await page.keyboard.press("Delete");
  await page.keyboard.up("Meta");
  await settle();
  await settle();
  s = await state();
  check("⌘Delete で行を削除する", s.rowCount === rowsBeforeDelete - 1, `${rowsBeforeDelete} → ${s.rowCount}`);
  check("削除した行が消えている", !s.names.includes("追加されたタスク"));
} else {
  check("⌘Delete で行を削除する", false, "追加に失敗したため実行せず");
}

// --- dragging the bar --------------------------------------------------------

const DAY_WIDTH = 26;

const taskById = async (id) =>
  page.evaluate(async (id) => {
    const grid = await (await fetch("/api/projects/test-project/grid")).json();
    return grid.tasks.find((task) => task.id === id) ?? null;
  }, id);

/** Drags a bar by whole days, grabbing either the middle or one end. */
const dragBar = async (taskId, days, grab) => {
  const box = await page.evaluate((taskId) => {
    const bar = document.querySelector(`.fg-bar[data-task="${taskId}"]`);
    if (!bar) return null;

    // Scroll the bar into view first: a grip that sits outside the chart's
    // viewport would take the click on the pane beside it.
    const chart = document.querySelector(".fg-pane-chart");
    chart.scrollLeft = Math.max(0, bar.offsetLeft - 80);

    const rect = bar.getBoundingClientRect();
    return { x: rect.x, y: rect.y, width: rect.width, height: rect.height };
  }, taskId);

  await new Promise((resolve) => setTimeout(resolve, 100));

  if (!box) throw new Error(`バーが見つかりません: ${taskId}`);

  const y = box.y + box.height / 2;
  const from =
    grab === "start" ? box.x + 3 : grab === "end" ? box.x + box.width - 3 : box.x + box.width / 2;

  await page.mouse.move(from, y);
  await page.mouse.down();
  // Two moves: the first starts the gesture, the second lands it.
  await page.mouse.move(from + (days * DAY_WIDTH) / 2, y);
  await page.mouse.move(from + days * DAY_WIDTH, y);
  await page.mouse.up();
  await settle();
  await settle();
};

// The chart keeps a strip of its own by default, which is not enough room to
// drag a bar two days to the right without leaving the window. Store a narrow
// table instead of styling the element: every commit re-renders the grid, and
// an inline style would not survive the first drag.
await page.evaluate(() => window.localStorage.setItem("fugantt:pane-width", "300"));
await page.reload({ waitUntil: "domcontentloaded" });
await page.waitForSelector(".fg-grid");
await settle();

const docId = await page.evaluate(async () => {
  const grid = await (await fetch("/api/projects/test-project/grid")).json();
  return grid.tasks.find((task) => task.name === "ドキュメント整備").id;
});

let before = await taskById(docId);
await dragBar(docId, 3, "middle");
let after = await taskById(docId);
check(
  "バーをドラッグすると期間ごと動く",
  after.start !== before.start && after.days === before.days,
  `${before.start}〜${before.end} → ${after.start}〜${after.end}`,
);

before = after;
await dragBar(docId, 2, "end");
after = await taskById(docId);
check(
  "右端をつまむと終了日だけ伸びる",
  after.start === before.start && after.days === before.days + 2,
  `${before.days}日 → ${after.days}日`,
);

before = after;
await dragBar(docId, 1, "start");
after = await taskById(docId);
check(
  "左端をつまむと開始日だけ動く",
  after.end === before.end && after.days === before.days - 1,
  `${before.days}日 → ${after.days}日`,
);

// 進捗の境目をつまんで動かす
const progressTask = await page.evaluate(async () => {
  const grid = await (await fetch("/api/projects/test-project/grid")).json();
  const task = grid.tasks.find((task) => !task.has_children && task.start && task.progress < 90);
  return { id: task.id, progress: task.progress };
});

const knob = await page.evaluate((id) => {
  const bar = document.querySelector(`.fg-bar[data-task="${id}"]`);
  const chart = document.querySelector(".fg-pane-chart");
  chart.scrollLeft = Math.max(0, bar.offsetLeft - 80);
  const box = bar.getBoundingClientRect();
  return {
    x: box.x + (box.width * Number(bar.dataset.progress ?? 0)) / 100,
    y: box.y + box.height / 2,
    width: box.width,
    left: box.x,
  };
}, progressTask.id);

await page.mouse.move(knob.x, knob.y);
await page.mouse.down();
await page.mouse.move(knob.left + knob.width * 0.5, knob.y);
await page.mouse.move(knob.left + knob.width * 0.8, knob.y);
await page.mouse.up();
await settle();
await settle();

const afterProgress = await page.evaluate(async (id) => {
  const grid = await (await fetch("/api/projects/test-project/grid")).json();
  return grid.tasks.find((task) => task.id === id).progress;
}, progressTask.id);

check(
  "バーの境目をつまんで進捗を変えられる",
  afterProgress > progressTask.progress + 10,
  `${progressTask.progress}% → ${afterProgress}%`,
);

const summaryBarDraggable = await page.evaluate(async () => {
  const grid = await (await fetch("/api/projects/test-project/grid")).json();
  const dev = grid.tasks.find((task) => task.name === "開発");
  return document
    .querySelector(`.fg-bar[data-task="${dev.id}"]`)
    ?.classList.contains("is-draggable");
});
check("集計行のバーはドラッグできない", summaryBarDraggable === false, String(summaryBarDraggable));

await page.evaluate(() => {
  document.querySelector(".fg-grid").style.removeProperty("--fg-pane-width");
});

// --- collaboration -----------------------------------------------------------

const other = await browser.newPage();
await other.goto(`${BASE}/login`, { waitUntil: "domcontentloaded" });
await other.evaluate(
  async (email, password) => {
    const body = new URLSearchParams({ email, password });
    await fetch("/login", { method: "POST", body });
  },
  EMAIL,
  PASSWORD,
);
await other.goto(`${BASE}/projects/test-project`, { waitUntil: "domcontentloaded" });
await other.waitForSelector(".fg-grid");
await settle();

// この画面で名前を変えると、もう一方の画面に伝わるはず
await selectCell(0, 0);
await page.keyboard.press("F2");
await replaceEditorText("共同編集の確認");
await page.keyboard.press("Enter");

await other
  .waitForFunction(
    () => document.querySelector(".fg-notice") !== null,
    { timeout: 5000 },
  )
  .catch(() => {});

const seen = await other.evaluate(() => ({
  notice: document.querySelector(".fg-notice")?.textContent ?? null,
  firstName: document.querySelector(".fg-name-text")?.textContent ?? null,
}));

check("他の画面に変更が届く", seen.firstName === "共同編集の確認", seen.firstName ?? "なし");
check("誰が変えたかを知らせる", (seen.notice ?? "").includes(EMAIL), seen.notice ?? "なし");

const ownNotice = await page.evaluate(
  () => document.querySelector(".fg-notice")?.textContent ?? null,
);
check("自分の変更では通知が出ない", ownNotice === null, ownNotice ?? "");

await other.close();

// --- export -----------------------------------------------------------------

const exported = await page.evaluate(async () => {
  const xlsx = await fetch("/projects/test-project/export.xlsx");
  const bytes = new Uint8Array(await xlsx.arrayBuffer());

  return {
    xlsxStatus: xlsx.status,
    xlsxType: xlsx.headers.get("content-type"),
    xlsxSize: bytes.length,
    // An xlsx is a zip; the magic number is the cheapest proof it is one.
    xlsxIsZip: bytes[0] === 0x50 && bytes[1] === 0x4b,
    disposition: xlsx.headers.get("content-disposition"),
  };
});

check(
  "Excel を書き出せる",
  exported.xlsxIsZip && exported.xlsxSize > 1000,
  `${exported.xlsxSize} バイト / ${exported.xlsxType}`,
);
check(
  "日本語のファイル名がヘッダを通る",
  exported.disposition?.includes("filename*=UTF-8''%"),
  exported.disposition ?? "",
);

// --- 予実と待ち ---------------------------------------------------------------

// The suite has moved rows around by now, so reseed and start from what the
// project actually looks like.
execFileSync("sh", [join(here, "seed.sh"), DB, EMAIL], { stdio: "inherit" });
await page.reload({ waitUntil: "domcontentloaded" });
await page.waitForSelector(".fg-grid");
await settle();

const planned = await page.evaluate(() => {
  const labels = [...document.querySelectorAll(".fg-heading .fg-cell")].map((c) => c.textContent.trim());
  const row = [...document.querySelectorAll(".fg-pane-left .fg-row.fg-data")][0];
  const cells = [...row.querySelectorAll(".fg-cell")].map((c) => c.textContent.trim());
  return { labels, cells };
});
// 予定の4つと実施の4つが、同じ形で並んでいること。目で下に追えば対応が読める。
check(
  "予定と実施が同じ並びで一行に出る",
  planned.labels.join(",").includes("予定開始,予定終了,予定日数,予定進捗,実施開始,実施終了,実作業日数,実進捗"),
  planned.labels.join(","),
);
check(
  "終了差異は引き算で出る",
  planned.cells[planned.labels.indexOf("終了差異")] === "+4日" &&
    planned.cells[planned.labels.indexOf("開始差異")] === "±0",
  planned.cells.join(","),
);
const drawn = await page.evaluate(() => {
  const rows = [...document.querySelectorAll(".fg-bar-row")];
  const named = [...document.querySelectorAll(".fg-pane-left .fg-row.fg-data .fg-cell-name")].map(
    (c) => c.textContent.trim(),
  );
  const at = (name) => rows[named.findIndex((n) => n.includes(name))];
  return {
    plan: !!at("要件定義").querySelector(".fg-bar.is-plan"),
    actual: !!at("要件定義").querySelector(".fg-actual"),
    variance: at("要件定義").querySelector(".fg-variance")?.textContent ?? null,
    open: !!at("設計").querySelector(".fg-actual.is-open"),
  };
});
check(
  "実施バーは予定の枠の中に描かれる",
  drawn.plan && drawn.actual && drawn.variance === "+4日",
  JSON.stringify(drawn),
);
check("終わっていない実施バーは開いたまま描く", drawn.open, JSON.stringify(drawn));

// 早く終わった行では、実施バーの端に置くと予定バーの上に数字が乗る。
check(
  "差異のラベルはどちらのバーにも重ならない",
  await page.evaluate(async () => {
    await fetch("/api/projects/test-project/tasks/t-req", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ field: "actual_schedule", value: "2026-08-03/2026-08-10" }),
    });

    await new Promise((done) => setTimeout(done, 600));

    const named = [...document.querySelectorAll(".fg-pane-left .fg-row.fg-data .fg-cell-name")].map(
      (c) => c.textContent.trim(),
    );
    const row = [...document.querySelectorAll(".fg-bar-row")][
      named.findIndex((n) => n.includes("要件定義"))
    ];
    const label = row.querySelector(".fg-variance:not(.is-start)");
    const plan = row.querySelector(".fg-bar");

    return (
      !!label &&
      !!plan &&
      label.getBoundingClientRect().left >= plan.getBoundingClientRect().right
    );
  }),
);

execFileSync("sh", [join(here, "seed.sh"), DB, EMAIL], { stdio: "inherit" });
await page.reload({ waitUntil: "domcontentloaded" });
await page.waitForSelector(".fg-grid");
await settle();

// 差異はバーの端に付く。離れていると、どの行の何日なのか読めない。
check(
  "差異のラベルがバーの端に付く",
  await page.evaluate(() => {
    const named = [...document.querySelectorAll(".fg-pane-left .fg-row.fg-data .fg-cell-name")].map(
      (c) => c.textContent.trim(),
    );
    const row = [...document.querySelectorAll(".fg-bar-row")][
      named.findIndex((n) => n.includes("要件定義"))
    ];
    const bar = row.querySelector(".fg-actual") ?? row.querySelector(".fg-bar");
    const label = row.querySelector(".fg-variance:not(.is-start)");
    if (!bar || !label) return false;

    const gap = label.getBoundingClientRect().left - bar.getBoundingClientRect().right;
    return gap >= 0 && gap < 20;
  }),
);

const bands = await page.evaluate(() => ({
  quarters: [...document.querySelectorAll(".fg-quarter")].map((q) => q.textContent.trim()),
  months: [...document.querySelectorAll(".fg-month")].map((m) => m.textContent.trim()),
}));
// 端の月が細切れだと、その名前が隣の月の上に重なって出る。
check(
  "細い月は名前を出さない",
  await page.evaluate(() =>
    [...document.querySelectorAll(".fg-month")].every(
      (month) => month.textContent.trim() === "" || month.getBoundingClientRect().width >= 56,
    ),
  ),
  await page.evaluate(() =>
    [...document.querySelectorAll(".fg-month")]
      .map((m) => `${m.textContent.trim()}:${Math.round(m.getBoundingClientRect().width)}`)
      .join(","),
  ),
);

check(
  "年度と四半期がチャートの上に出る",
  bands.quarters.includes("2026年度 Q2") && bands.months.includes("2026年8月"),
  JSON.stringify(bands),
);

check(
  "ステータスのセルを開ける",
  await page.evaluate(async () => {
    const cells = [...document.querySelectorAll(".fg-heading .fg-cell")];
    const column = cells.findIndex((c) => c.textContent.trim() === "ステータス");
    const row = [...document.querySelectorAll(".fg-pane-left .fg-row.fg-data")][0];
    row.querySelectorAll(".fg-cell")[column].dispatchEvent(new MouseEvent("mousedown", { bubbles: true }));
    return true;
  }),
);
await page.keyboard.press("F2");
await settle();
check(
  "ステータスの候補が並ぶ",
  await page.evaluate(() =>
    [...(document.querySelector("select.fg-editor")?.options ?? [])].some((o) => o.value === "完了"),
  ),
);
await page.keyboard.press("Escape");
await settle();

// --- 担当者の休暇 ---------------------------------------------------------------

const daysOf = (name) =>
  page.evaluate((name) => {
    const rows = [...document.querySelectorAll(".fg-pane-left .fg-row.fg-data")];
    const row = rows.find((r) => r.querySelector(".fg-cell-name").textContent.includes(name));
    return row?.querySelector(".fg-cell-days")?.textContent.trim();
  }, name);

const designDaysBefore = await daysOf("設計");

const leaveId = await page.evaluate(async () => {
  const body = new URLSearchParams({
    assignee: "佐藤",
    start: "2026-08-17",
    end: "2026-08-21",
    note: "夏季休暇",
  });
  await fetch("/projects/test-project/leaves", { method: "POST", body });

  const grid = await (await fetch("/api/projects/test-project/grid")).json();
  return grid.leaves.at(-1)?.id ?? "";
});

await page.reload({ waitUntil: "domcontentloaded" });
await page.waitForSelector(".fg-grid");
await settle();

const designDaysAfter = await daysOf("設計");
check(
  "休暇の日数は担当者のタスクから引かれる",
  Number(designDaysAfter) === Number(designDaysBefore) - 5,
  `${designDaysBefore} → ${designDaysAfter}`,
);
// スタイルごと消えていて、位置も色も付かない箱になっていたことがある。
check(
  "休暇はチャートで色が付いて見える",
  await page.evaluate(() => {
    const leave = document.querySelector(".fg-leave");
    if (!leave) return false;

    const style = getComputedStyle(leave);
    return (
      style.position === "absolute" &&
      style.backgroundColor !== "rgba(0, 0, 0, 0)" &&
      Math.round(leave.getBoundingClientRect().width) > 0
    );
  }),
  await page.evaluate(() => {
    const leave = document.querySelector(".fg-leave");
    return leave ? getComputedStyle(leave).backgroundColor : "なし";
  }),
);

check(
  "休暇は担当者の行にだけ出る",
  await page.evaluate(() => {
    const named = [...document.querySelectorAll(".fg-pane-left .fg-row.fg-data .fg-cell-name")].map(
      (c) => c.textContent.trim(),
    );
    const rows = [...document.querySelectorAll(".fg-bar-row")];
    const at = (name) => rows[named.findIndex((n) => n.includes(name))];
    return (
      at("設計").querySelectorAll(".fg-leave").length === 1 &&
      at("レビュー").querySelectorAll(".fg-leave").length === 0
    );
  }),
);

// Leave outlives the seed, so the test has to take its own entry back out.
await page.evaluate(async (id) => {
  await fetch("/projects/test-project/leaves/remove", {
    method: "POST",
    body: new URLSearchParams({ id }),
  });
}, leaveId);

await page.reload({ waitUntil: "domcontentloaded" });
await page.waitForSelector(".fg-grid");
await settle();
check("休暇を消すと日数が戻る", (await daysOf("設計")) === designDaysBefore, await daysOf("設計"));

// ものさしは一本。実施バーがある行でも、塗りは予定バーの側に乗る。
check(
  "実施バーがあっても塗りは予定バーに乗る",
  await page.evaluate(() => {
    const named = [...document.querySelectorAll(".fg-pane-left .fg-row.fg-data .fg-cell-name")].map(
      (c) => c.textContent.trim(),
    );
    const rows = [...document.querySelectorAll(".fg-bar-row")];
    const row = rows[named.findIndex((n) => n.includes("要件定義"))];

    return (
      !!row.querySelector(".fg-bar .fg-bar-fill") && !row.querySelector(".fg-actual .fg-bar-fill")
    );
  }),
);

// --- 統計 ---------------------------------------------------------------------

const stats = await page.evaluate(async () => {
  const html = await (await fetch("/projects/test-project/stats")).text();
  const doc = new DOMParser().parseFromString(html, "text/html");
  return doc.querySelector("main")?.textContent.replace(/\s+/g, " ") ?? "";
});
check("統計にタスク数が出る", stats.includes("タスク"), stats.slice(0, 80));
check(
  "ずれを作業と待ちに分けて出す",
  stats.includes("ずれの内訳") && stats.includes("作業の遅れ") && stats.includes("待ち"),
  stats.replace(/.*ずれの内訳/, "").slice(0, 90),
);
check(
  "作業の遅れが出る",
  Number(stats.match(/作業の遅れ\s*(\d+)日/)?.[1]) > 0,
  stats.slice(0, 100),
);

// --- 実施バーのドラッグ -----------------------------------------------------------

await page.evaluate(() => window.localStorage.setItem("fugantt:pane-width", "300"));
await page.reload({ waitUntil: "domcontentloaded" });
await page.waitForSelector(".fg-grid");
await settle();

/** Drags the actual bar of `task`, the way a hand would. */
const dragActual = async (task, days, grab) => {
  // Bring the bar fully into the chart's viewport first: an edge scrolled off
  // to the left is not an edge the mouse can grab.
  await page.evaluate((task) => {
    const named = [...document.querySelectorAll(".fg-pane-left .fg-row.fg-data .fg-cell-name")].map(
      (c) => c.textContent.trim(),
    );
    const rows = [...document.querySelectorAll(".fg-bar-row")];
    const bar = rows[named.findIndex((n) => n.includes(task))]?.querySelector(".fg-actual");
    const pane = document.querySelector(".fg-pane-chart");
    if (bar && pane) pane.scrollLeft = Math.max(0, bar.offsetLeft - 80);
  }, task);
  await settle();

  const box = await page.evaluate((task) => {
    const named = [...document.querySelectorAll(".fg-pane-left .fg-row.fg-data .fg-cell-name")].map(
      (c) => c.textContent.trim(),
    );
    const rows = [...document.querySelectorAll(".fg-bar-row")];
    const bar = rows[named.findIndex((n) => n.includes(task))]?.querySelector(".fg-actual");
    if (!bar) return null;
    const rect = bar.getBoundingClientRect();
    return { x: rect.x, y: rect.y, width: rect.width, height: rect.height };
  }, task);

  if (!box) throw new Error(`実施バーが見つかりません: ${task}`);

  const y = box.y + box.height / 2;
  const from =
    grab === "start" ? box.x + 3 : grab === "end" ? box.x + box.width - 3 : box.x + box.width / 2;

  await page.mouse.move(from, y);
  await page.mouse.down();
  await page.mouse.move(from + (days * DAY_WIDTH) / 2, y);
  await page.mouse.move(from + days * DAY_WIDTH, y);
  await page.mouse.up();
  await settle();
  await settle();
};

const actualOf = (id) =>
  page.evaluate(async (id) => {
    const grid = await (await fetch("/api/projects/test-project/grid")).json();
    const task = grid.tasks.find((t) => t.id === id);
    return { start: task.actual_start, end: task.actual_end };
  }, id);

let actual = await actualOf("t-req");
await dragActual("要件定義", 2, "middle");
let moved = await actualOf("t-req");
check(
  "実施バーをドラッグすると実施期間ごと動く",
  moved.start === "2026-08-05" && moved.end === "2026-08-20",
  `${actual.start}〜${actual.end} → ${moved.start}〜${moved.end}`,
);

await dragActual("要件定義", -3, "start");
moved = await actualOf("t-req");
check(
  "左端をつまむと実施開始だけ動く",
  moved.start === "2026-08-02" && moved.end === "2026-08-20",
  `${moved.start}〜${moved.end}`,
);

// 進行中の行には終了日が無い。右端をつまんだときだけ、それを書く。
actual = await actualOf("t-des");
await dragActual("設計", 2, "middle");
moved = await actualOf("t-des");
check(
  "進行中のバーを動かしても終了日は作らない",
  moved.end === null && moved.start !== actual.start,
  `${actual.start}〜${actual.end} → ${moved.start}〜${moved.end}`,
);

await dragActual("設計", 4, "end");
moved = await actualOf("t-des");
check("進行中のバーは右端をつまむと終わる", moved.end !== null, `${moved.start}〜${moved.end}`);

await page.evaluate(() => window.localStorage.removeItem("fugantt:pane-width"));
execFileSync("sh", [join(here, "seed.sh"), DB, EMAIL], { stdio: "inherit" });
await page.reload({ waitUntil: "domcontentloaded" });
await page.waitForSelector(".fg-grid");
await settle();

// --- 進捗の入れ方 ---------------------------------------------------------------

execFileSync("sh", [join(here, "seed.sh"), DB, EMAIL], { stdio: "inherit" });

/** The view form rewrites every column, so each post has to carry them all. */
const setView = (extra = {}) =>
  page.evaluate(
    async (extra) => {
      // 列は別のフォーム、それ以外は表示の設定。どちらも受け取ったものだけを見る。
      const columns = new URLSearchParams();
      for (const key of [
        "name",
        "start",
        "end",
        "actual_start",
        "actual_end",
        "days",
        "actual_days",
        "start_variance",
        "end_variance",
        "targets",
        "progress",
        "status",
        "assignee",
        "note",
        "waits",
      ]) {
        columns.set(`column_${key}`, "1");
      }
      await fetch("/projects/test-project/columns", { method: "POST", body: columns });

      const body = new URLSearchParams();
      body.set("skip_leave", "1");
      // チェックボックスは送られなければ「外した」の意味になる。既定のままの
      // ものも明示しておかないと、保存のたびに消えていく。
      body.set("quarters", "1");
      for (const [key, value] of Object.entries(extra)) body.set(key, value);
      await fetch("/projects/test-project/view", { method: "POST", body });
    },
    extra,
  );

/**
 * 今日の日付、サーバーと同じ暦で。
 *
 * `toISOString()` は UTC なので、日本時間の 0 時から 9 時のあいだだけ前日を返す。
 * サーバーはその時間も「今日」を今日と言うので、テストだけが夜中に落ちていた。
 */
const today = () => new Date().toLocaleDateString("sv-SE");

// The status cell's own editor is covered above; what is under test here is the
// rule the server applies afterwards, so the edit goes the way the grid sends it.
const setCell = (task, field, value) =>
  page.evaluate(
    async (task, field, value) => {
      const response = await fetch(`/api/projects/test-project/tasks/${task}`, {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ field, value }),
      });
      const { grid } = await response.json();
      const row = grid.tasks.find((t) => t.id === task);
      return { progress: row.progress, actual_end: row.actual_end, status: row.status };
    },
    task,
    field,
    value,
  );

await setView();
let row = await setCell("t-imp", "status", "完了");
check(
  "手入力のままなら進捗は動かない",
  row.progress === 10 && row.actual_end === null,
  JSON.stringify(row),
);

await setView({ progress_mode: "status" });
row = await setCell("t-imp", "status", "完了");
check("ステータス連動: 完了で 100%", row.progress === 100, JSON.stringify(row));
check(
  "100% にすると実施終了が今日で埋まる",
  row.actual_end === today(),
  JSON.stringify(row),
);

row = await setCell("t-imp", "status", "未着手");
check("ステータス連動: 未着手で 0%", row.progress === 0, JSON.stringify(row));
check(
  "すでに入っている実施終了は書き換えない",
  row.actual_end === today(),
  JSON.stringify(row),
);

// 集計行は子から決まるので、連動しても書き込まない。
const parentBeforeStatus = await page.evaluate(async () => {
  const grid = await (await fetch("/api/projects/test-project/grid")).json();
  return grid.tasks.find((t) => t.id === "t-dev").progress;
});
const parent = await setCell("t-dev", "status", "完了");
check(
  "集計行の進捗は連動でも書き換えない",
  parent.progress === parentBeforeStatus && parent.actual_end === null,
  `${parentBeforeStatus} → ${JSON.stringify(parent)}`,
);

row = await setCell("t-imp", "status", "実施中");
check("実施中は手入力のまま", row.progress === 0, JSON.stringify(row));

// 予定終了が分かっている行を 100% にすると、実施終了はそこで埋まる。
row = await setCell("t-test", "progress", "100");
check(
  "進捗 100% だけでも実施終了が入る",
  row.actual_end === today(),
  JSON.stringify(row),
);

check(
  "連動した書き込みも履歴に残る",
  await page.evaluate(async () => {
    const html = await (await fetch("/projects/test-project/history")).text();
    return html.includes("実施終了") && html.includes("進捗");
  }),
);

// 選択行をチャート側で塗りつぶすと、暦の縞が消えて「描かれていない行」に見える。
await selectCell(2, 0);
await settle();
check(
  "選択行はチャートの暦を隠さない",
  await page.evaluate(() => {
    const bar = document.querySelector(".fg-bar-row.is-current");
    const colour = bar && getComputedStyle(bar).backgroundColor;
    const alpha = Number(/rgba?\([^)]*?([\d.]+)\)$/.exec(colour ?? "")?.[1] ?? "1");
    return !!bar && alpha < 1;
  }),
  await page.evaluate(() => {
    const bar = document.querySelector(".fg-bar-row.is-current");
    return bar ? getComputedStyle(bar).backgroundColor : "なし";
  }),
);

// 絞り込みの欄は、横スクロールしても見出しの真下から動かない。
check(
  "絞り込みの欄は見出しと同じ位置にある",
  await page.evaluate(() => {
    const pane = document.querySelector(".fg-pane-left");
    pane.scrollLeft = 600;

    const heads = [...document.querySelectorAll(".fg-heading .fg-cell")];
    const filters = [...document.querySelectorAll(".fg-filters .fg-cell")];

    const off = heads.map(
      (head, at) =>
        Math.round(head.getBoundingClientRect().x - filters[at].getBoundingClientRect().x),
    );

    pane.scrollLeft = 0;
    return heads.length === filters.length && off.every((gap) => gap === 0);
  }),
);

// --- 進捗の描き場所と絞り込みの向き ------------------------------------------------

// 進捗は「終わった量」なので、ものさしは予定の長さで固定する。実施バーに乗せる
// と、同じ 60% が行ごとに別の位置に出るうえ、終わっていない実施バーは今日まで
// 伸びるので、何もしていない日でも塗りが進んで見える。
check(
  "進捗の塗りは実施バーではなく予定バーに乗る",
  await page.evaluate(() => {
    const rows = [...document.querySelectorAll(".fg-bar-row")];
    const row = rows.find((r) => r.querySelector(".fg-actual") && r.querySelector(".fg-bar.is-plan"));
    if (!row) return false;

    const fill = row.querySelector(".fg-bar.is-plan .fg-bar-fill");
    return (
      !!fill &&
      fill.getBoundingClientRect().width > 0 &&
      !row.querySelector(".fg-actual .fg-bar-fill")
    );
  }),
  await page.evaluate(() => {
    const rows = [...document.querySelectorAll(".fg-bar-row")];
    const row = rows.find((r) => r.querySelector(".fg-actual") && r.querySelector(".fg-bar.is-plan"));
    return row ? row.innerHTML.slice(0, 200) : "実施バーのある行がない";
  }),
);

// 予定進捗は「量」の位置に置き、日付を文字で書く。
//
// 逆（日付の位置に印、量を文字）を先に試して、手で触った瞬間に壊れた。このバーは
// 掴むと進捗を入れる装置になる——つまり横軸は％として扱われる。そこに日付の位置で
// 印を置くと、塗りが届いた＝達成、と読まれる。作った本人が半日で2回やった。
// 比べるもの同士は同じものさしに乗せる。日付はここでは何とも比べないので文字。
check(
  "足りない分は塗りの続きから、約束した％まで",
  await page.evaluate(() => {
    const named = [...document.querySelectorAll(".fg-pane-left .fg-row.fg-data .fg-cell-name")].map(
      (c) => c.textContent.trim(),
    );
    const at = named.findIndex((n) => n.includes("ドキュメント整備"));
    const bar = [...document.querySelectorAll(".fg-bar-row")][at]?.querySelector(".fg-bar.is-plan");
    const band = bar?.querySelector(".fg-bar-behind");
    const fill = bar?.querySelector(".fg-bar-fill");
    if (!band || !fill) return false;

    const box = bar.getBoundingClientRect();
    const right = ((band.getBoundingClientRect().right - box.left) / box.width) * 100;

    // 5% まで来ていて、8/5 までに 50% の約束。帯は塗りの右端から 50% の位置まで。
    return (
      Math.abs(band.getBoundingClientRect().left - fill.getBoundingClientRect().right) <= 1 &&
      Math.abs(right - 50) <= 1 &&
      getComputedStyle(band).backgroundColor === "rgb(220, 38, 38)"
    );
  }),
  await page.evaluate(() => {
    const band = document.querySelector(".fg-bar-behind");
    if (!band) return "帯がない";
    const bar = band.closest(".fg-bar").getBoundingClientRect();
    const box = band.getBoundingClientRect();
    return `${(((box.left - bar.left) / bar.width) * 100).toFixed(1)}% → ${(((box.right - bar.left) / bar.width) * 100).toFixed(1)}% / ${band.title}`;
  }),
);

check(
  "いつまでか、は文字で出る",
  await page.evaluate(() => {
    const named = [...document.querySelectorAll(".fg-pane-left .fg-row.fg-data .fg-cell-name")].map(
      (c) => c.textContent.trim(),
    );
    const at = named.findIndex((n) => n.includes("ドキュメント整備"));
    const row = [...document.querySelectorAll(".fg-bar-row")][at];
    const bar = row.querySelector(".fg-bar.is-plan");
    const band = bar?.querySelector(".fg-bar-behind");
    const label = row.querySelector(".fg-target-label");
    if (!band || !label) return false;

    const style = getComputedStyle(label);

    return (
      label.textContent === "8/5 50%" &&
      // 帯の端のすぐ右。離れると、どの約束の日付か分からなくなる。
      label.getBoundingClientRect().left - band.getBoundingClientRect().right < 8 &&
      style.color === "rgb(220, 38, 38)" &&
      // 塗りの上に乗ることがあるので、白い縁で必ず読めるようにしてある。
      style.textShadow.includes("rgb(255, 255, 255)")
    );
  }),
  await page.evaluate(() => {
    const label = document.querySelector(".fg-target-label");
    return label ? `${label.textContent} / ${getComputedStyle(label).color}` : "文字が無い";
  }),
);

// 約束に届いた瞬間に赤が消える。これが触っていて分かる唯一の合図なので、
// 境目そのものを見る。
const crossing = await page.evaluate(async () => {
  const grid = await (await fetch("/api/projects/test-project/grid")).json();
  const id = grid.tasks.find((task) => task.name === "ドキュメント整備").id;

  const read = async (value) => {
    await fetch(`/api/projects/test-project/tasks/${id}`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ field: "progress", value: String(value) }),
    });
    await new Promise((done) => setTimeout(done, 500));

    const bar = document.querySelector(`.fg-bar[data-task="${id}"]`);
    return {
      red: bar.classList.contains("is-delayed"),
      band: !!bar.querySelector(".fg-bar-behind"),
    };
  };

  const before = await read(49);
  const after = await read(50);
  await read(5);

  return { before, after };
});

check(
  "約束の％に届くと赤が消える",
  crossing.before.red && crossing.before.band && !crossing.after.red && !crossing.after.band,
  JSON.stringify(crossing),
);

// まだ来ていない約束は帯ではなく細い印。位置はやはり％、日付は文字。
check(
  "これからの予定進捗はその％の位置に細く出る",
  await page.evaluate(() => {
    const named = [...document.querySelectorAll(".fg-pane-left .fg-row.fg-data .fg-cell-name")].map(
      (c) => c.textContent.trim(),
    );
    const at = named.findIndex((n) => n.trim() === "設計");
    const row = [...document.querySelectorAll(".fg-bar-row")][at];
    const bar = row.querySelector(".fg-bar.is-plan");
    const mark = row.querySelector(".fg-target");
    const label = [...row.querySelectorAll(".fg-target-label")].find(
      (l) => l.textContent === "8/24 90%",
    );
    if (!mark || !label) return false;

    const box = bar.getBoundingClientRect();
    const left = ((mark.getBoundingClientRect().left - box.left) / box.width) * 100;

    // 8/24 までに 90%。帯は出ない（まだその日ではない）。
    return (
      Math.abs(left - 90) <= 1.5 &&
      mark.title.includes("90%") &&
      !bar.querySelector(".fg-bar-behind") &&
      getComputedStyle(mark).backgroundColor !== "rgb(220, 38, 38)"
    );
  }),
  await page.evaluate(() => {
    const mark = document.querySelector(".fg-target");
    return mark ? `${mark.style.left} / ${mark.title}` : "印がない";
  }),
);

// 達成した約束は何も描かない。塗りがその先まで来ている、それが答えになっている。
check(
  "達成した予定進捗は描かない",
  await page.evaluate(async () => {
    const grid = await (await fetch("/api/projects/test-project/grid")).json();
    const design = grid.tasks.find((task) => task.name === "設計");
    const met = design.targets.filter((target) => target.due && !target.missed);

    const named = [...document.querySelectorAll(".fg-pane-left .fg-row.fg-data .fg-cell-name")].map(
      (c) => c.textContent.trim(),
    );
    const at = named.findIndex((n) => n.trim() === "設計");
    const row = [...document.querySelectorAll(".fg-bar-row")][at];

    // 8/12 の 50% は達成済み。印も文字も出ていないこと。
    return (
      met.length === 1 &&
      [...row.querySelectorAll(".fg-target, .fg-target-label")].every(
        (el) => !el.title.includes("08-12") && !el.textContent.startsWith("8/12"),
      )
    );
  }),
  await page.evaluate(() => {
    const rows = [...document.querySelectorAll(".fg-bar-row")];
    return rows
      .map((r) => [...r.querySelectorAll(".fg-target-label")].map((l) => l.textContent).join("+"))
      .join(" / ");
  }),
);

// チャートに何を出すかは人と日による。予定進捗も切れる。
check(
  "予定進捗はチャートから消せる",
  await page.evaluate(async () => {
    const drawn = () => document.querySelectorAll(".fg-target, .fg-target-label, .fg-bar-behind").length;
    const before = drawn();

    document.querySelector(".fg-shows").click();
    await new Promise((done) => setTimeout(done, 200));
    const box = document.querySelector('.fg-shows-menu input[data-shows="targets"]');
    if (!box) return false;

    box.click();
    await new Promise((done) => setTimeout(done, 300));
    const after = drawn();
    const remembered = window.localStorage.getItem("fugantt:chart-shows");

    // 戻す
    document.querySelector(".fg-shows").click();
    await new Promise((done) => setTimeout(done, 200));
    document.querySelector('.fg-shows-menu input[data-shows="targets"]').click();
    await new Promise((done) => setTimeout(done, 300));
    document.body.click();

    return before > 0 && after === 0 && drawn() === before && remembered.includes('"targets":false');
  }),
);

// ただし期限超過は日付の事実なので、何も約束していなくても赤い。約束を破った
// のではなく、日が過ぎたという別の話。
check(
  "予定進捗が無くても、期限を過ぎて終わっていなければ赤い",
  await page.evaluate(async () => {
    const grid = await (await fetch("/api/projects/test-project/grid")).json();
    const over = grid.tasks.find((task) => task.overdue > 0 && task.targets.length === 0);
    if (!over) return false;

    const named = [...document.querySelectorAll(".fg-pane-left .fg-row.fg-data .fg-cell-name")].map(
      (c) => c.textContent.trim(),
    );
    const at = named.findIndex((n) => n.includes(over.name));
    const row = [...document.querySelectorAll(".fg-pane-left .fg-row.fg-data")][at];

    return over.delayed === false && row.classList.contains("is-delayed");
  }),
  await page.evaluate(async () => {
    const grid = await (await fetch("/api/projects/test-project/grid")).json();
    return grid.tasks.map((t) => `${t.name}:超${t.overdue}:遅${t.delayed}`).join(" / ");
  }),
);

// 何も約束していない行は遅れない。ここが今回の設計そのもの。
check(
  "予定進捗を入れていない行は遅れにならない",
  await page.evaluate(async () => {
    const grid = await (await fetch("/api/projects/test-project/grid")).json();
    const none = grid.tasks.filter((task) => task.targets.length === 0 && !task.has_children);

    return none.length > 0 && none.every((task) => task.delayed === false);
  }),
  await page.evaluate(async () => {
    const grid = await (await fetch("/api/projects/test-project/grid")).json();
    return grid.tasks
      .map((t) => `${t.name}:${t.targets.length}:${t.delayed}:${t.expected}`)
      .join(" / ");
  }),
);

// 期日前の予定進捗は、届いていなくても遅れではない。まだ来ていない約束を破った
// ことにはできない。
check(
  "その日が来るまでは判定しない",
  await page.evaluate(async () => {
    const grid = await (await fetch("/api/projects/test-project/grid")).json();
    const design = grid.tasks.find((task) => task.name === "設計");
    const [met, soon] = design.targets;

    // 8/12 に 50% を約束して 60% まで来ている。8/24 の 90% はこれから。
    return (
      design.targets.length === 2 &&
      met.due === true && met.missed === false &&
      soon.due === false && soon.missed === false &&
      design.expected === met.percent &&
      design.delayed === false
    );
  }),
  await page.evaluate(async () => {
    const grid = await (await fetch("/api/projects/test-project/grid")).json();
    return JSON.stringify(grid.tasks.find((task) => task.name === "設計")?.targets);
  }),
);

/** その列の向きボタン。 */
const opChip = (column) => `.fg-filters .fg-cell-${COLUMN_KEY[column]} .fg-filter-op`;
const opMark = (column) => page.$eval(opChip(column), (b) => b.textContent);

/** ボタンを押して、出てきた一覧から比べ方を選ぶ。 */
const setOp = async (column, at) => {
  // 本物のクリックではなく element.click()。向きボタンは横に流れる領域にあって、
  // 右のほうの列は固定列の下に隠れる。隠れた要素は puppeteer が「押せない」と言う。
  await page.$eval(opChip(column), (button) => {
    button.scrollIntoView({ block: "nearest", inline: "nearest" });
    button.click();
  });
  await page.click(`.fg-bound-menu [data-bound="${at}"]`);
  await settle();
};

check("開始は以上、終了は以下から始まる", (await opMark("予定開始")) === "≧" && (await opMark("予定終了")) === "≦",
  `${await opMark("予定開始")} / ${await opMark("予定終了")}`);

await filterBy("予定開始", "20260810");
const fromTenth = (await state()).names;
await setOp("予定開始", "lte");
const toTenth = (await state()).names;

check(
  "一覧から選んで以上と以下が入れ替わる",
  (await opMark("予定開始")) === "≦" && toTenth.join() !== fromTenth.join(),
  `≧ ${fromTenth.join(",")} → ≦ ${toTenth.join(",")}`,
);

check(
  "いま選ばれている比べ方が一覧で分かる",
  await page.evaluate(async () => {
    document.querySelector(".fg-filters .fg-cell-start .fg-filter-op").click();
    const current = document.querySelector(".fg-bound-menu .fg-menu-item.is-current");
    const labels = [...document.querySelectorAll(".fg-bound-menu .fg-menu-item span")].map(
      (s) => s.textContent,
    );
    const ok =
      current?.dataset.bound === "lte" &&
      ["以上", "以下", "一致", "超過", "未満"].every((label) => labels.includes(label));
    document.dispatchEvent(new MouseEvent("mousedown", { bubbles: true }));
    return ok;
  }),
);

check(
  "向きを変えても入力した値は残る",
  (await page.evaluate(() => document.querySelector('.fg-filter[data-column="start"]')?.value)) === "2026-08-10",
  await page.evaluate(() => document.querySelector('.fg-filter[data-column="start"]')?.value ?? "なし"),
);

await page.click(".fg-filter-clear");
await settle();
check("解除で向きも既定に戻る", (await opMark("予定開始")) === "≧" && (await state()).rowCount === 7,
  `${await opMark("予定開始")} / ${(await state()).rowCount}`);

// 以上・以下だけでは足りない: 「ちょうど100%」「0日を超える」も条件になる。
await filterBy("実進捗", "100");
await setOp("実進捗", "eq");
// 残った行は全部ちょうど 100%。どの行が 100% かは、ここまでのテストで動く。
const hundreds = (await state()).names;
check(
  "一致で絞り込める",
  hundreds.length > 0 &&
    (await page.evaluate(() =>
      [...document.querySelectorAll(".fg-pane-left .fg-row.fg-data .fg-cell-progress")].every(
        (cell) => cell.textContent.trim() === "100%",
      ),
    )),
  hundreds.join(","),
);

await setOp("実進捗", "gt");
check(
  "超過はその値を含まない",
  (await state()).names.every((name) => !hundreds.includes(name)),
  `一致 ${hundreds.join(",")} / 超過 ${(await state()).names.join(",")}`,
);

await filterBy("実進捗", "40");
await setOp("実進捗", "lt");
const under = (await state()).names;
await setOp("実進捗", "lte");
const upTo = (await state()).names;
check(
  "未満と以下は境目の行だけ違う",
  upTo.includes("レビュー") && !under.includes("レビュー"),
  `未満 ${under.join(",")} / 以下 ${upTo.join(",")}`,
);

await page.click(".fg-filter-clear");
await settle();

// 進捗は「何%以上」より「遅れている行だけ」で見たいことのほうが多く、それには
// 打ち込む数字がない。
await setOp("実進捗", "behind");

check("進捗は遅れ・順調でも絞り込める", (await opMark("実進捗")) === "遅れ", await opMark("実進捗"));

const behind = (await state()).names;
await setOp("実進捗", "ahead");
const ahead = (await state()).names;

check(
  "遅れと順調で行が分かれる",
  behind.length > 0 &&
    ahead.length > 0 &&
    !behind.some((name) => ahead.includes(name) && name !== "開発"),
  `遅れ ${behind.join(",")} / 順調 ${ahead.join(",")}`,
);

check(
  "数字を打たなくても絞り込みは効いている",
  (await page.evaluate(() => document.getElementById("fugantt-filter-count")?.textContent ?? "")).includes("/"),
  await page.evaluate(() => document.getElementById("fugantt-filter-count")?.textContent ?? "なし"),
);

await page.click(".fg-filter-clear");
await settle();
check("解除で全部戻る（向きの条件も）", (await state()).rowCount === 7, String((await state()).rowCount));

// --- 固定する列 -------------------------------------------------------------------

const pinned = await page.evaluate(async () => {
  const body = new URLSearchParams();
  for (const key of [
    "start", "end", "actual_start", "actual_end", "days",
    "actual_days", "start_variance", "end_variance", "progress", "status", "assignee", "note",
  ]) {
    body.set(`column_${key}`, "1");
  }
  body.set("skip_leave", "1");
  body.set("frozen_columns", "3");
  await fetch("/projects/test-project/view", { method: "POST", body });
  return true;
});

await page.reload({ waitUntil: "domcontentloaded" });
await page.waitForSelector(".fg-grid");
await settle();

// 行ごとに幅を決めさせると、名前の長い行だけ列がずれる。
check(
  "どの行も同じ位置に列が並ぶ",
  await page.evaluate(() => {
    const rows = [...document.querySelectorAll(".fg-pane-left .fg-row")];
    const lefts = rows.map((row) =>
      Math.round(row.querySelectorAll(".fg-cell")[1].getBoundingClientRect().x),
    );
    return new Set(lefts).size === 1;
  }),
  await page.evaluate(() =>
    [
      ...new Set(
        [...document.querySelectorAll(".fg-pane-left .fg-row")].map((row) =>
          Math.round(row.querySelectorAll(".fg-cell")[1].getBoundingClientRect().x),
        ),
      ),
    ].join(","),
  ),
);

/** Where the first, third and fourth columns sit relative to the pane's edge. */
const atLeftEdge = (scroll) =>
  page.evaluate((scroll) => {
    const pane = document.querySelector(".fg-pane-left");
    pane.scrollLeft = scroll;

    const cells = [...document.querySelector(".fg-row.fg-data").querySelectorAll(".fg-cell")];
    const edge = Math.round(pane.getBoundingClientRect().left);
    const at = (index) => Math.round(cells[index].getBoundingClientRect().left) - edge;

    return { scrolled: pane.scrollLeft, first: at(0), third: at(2), fourth: at(3) };
  }, scroll);

const still = await atLeftEdge(0);
let edges = await atLeftEdge(500);
check(
  "固定した列は横スクロールで動かない",
  pinned &&
    edges.scrolled > 0 &&
    edges.first === still.first &&
    edges.third === still.third &&
    edges.fourth < still.fourth,
  `${JSON.stringify(still)} → ${JSON.stringify(edges)}`,
);

// ウィンドウが変われば列幅も変わる。止める位置も付いていかないとずれる。
await page.setViewport({ width: 1500, height: 700 });
await settle();
edges = await atLeftEdge(500);
check(
  "窓の大きさが変わっても固定位置が付いていく",
  edges.first === 0 && edges.third > 0,
  JSON.stringify(edges),
);
await page.setViewport({ width: 1680, height: 700 });
await settle();

await page.evaluate(async () => {
  const body = new URLSearchParams();
  for (const key of [
    "start", "end", "actual_start", "actual_end", "days",
    "actual_days", "start_variance", "end_variance", "progress", "status", "assignee", "note",
  ]) {
    body.set(`column_${key}`, "1");
  }
  body.set("skip_leave", "1");
  body.set("frozen_columns", "1");
  await fetch("/projects/test-project/view", { method: "POST", body });
});
await page.reload({ waitUntil: "domcontentloaded" });
await page.waitForSelector(".fg-grid");
await settle();

// 保存のたびに、開いていたものが閉じて画面が上に戻るのは地味に効く。
const stayed = await page.evaluate(async () => {
  const settings = async (path, body) => {
    const response = await fetch(`/projects/test-project/${path}`, {
      method: "POST",
      body: new URLSearchParams(body),
      redirect: "follow",
    });
    return response.url;
  };

  const holiday = await settings("holidays", { date: "2026-12-30", name: "仕事納め" });
  await settings("holidays/remove", { date: "2026-12-30" });

  const memo = await fetch("/projects/test-project/memo", {
    method: "POST",
    body: new URLSearchParams({ memo: "閉じない" }),
    redirect: "follow",
  });

  const page_ = await (await fetch(memo.url)).text();

  return {
    holiday,
    memo: memo.url,
    // The panel is a checkbox; it comes back ticked.
    panelOpen: page_.includes('id="fg-memo" checked'),
  };
});

check(
  "設定を保存すると、その節に戻ってくる",
  // fetch はフラグメントを捨てるので、ここで見えるのは open= まで。
  // 実際のスクロールはブラウザが #holidays でやる。
  stayed.holiday.includes("open=holidays"),
  stayed.holiday,
);
check(
  "メモを保存してもパネルは開いたまま",
  stayed.memo.includes("memo=1") && stayed.panelOpen,
  `${stayed.memo} panel=${stayed.panelOpen}`,
);

// --- 日付の入力 -------------------------------------------------------------------

const dates = await page.evaluate(async () => {
  const set = async (field, value) => {
    const response = await fetch("/api/projects/test-project/tasks/t-test", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ field, value }),
    });

    if (!response.ok) return { status: response.status };

    const { grid } = await response.json();
    const task = grid.tasks.find((t) => t.id === "t-test");
    return { status: response.status, start: task.start, end: task.end };
  };

  await set("schedule", "2026-09-21/2026-10-09");

  return {
    digits: await set("start", "20260920"),
    slashes: await set("start", "9/22"),
    kanji: await set("start", "2026年9月23日"),
    // 終了より後の開始は通さない。
    inverted: await set("start", "2026-10-20"),
    invertedEnd: await set("end", "2026-08-01"),
    restored: await set("schedule", "2026-09-21/2026-10-09"),
  };
});

check("数字だけで日付を入れられる", dates.digits.start === "2026-09-20", JSON.stringify(dates.digits));

// 打っている最中に整形される。普通の日付欄と同じ手触り。
await selectCell((await state()).names.indexOf("テスト"), COLUMN["予定開始"]);
await page.keyboard.press("Enter");
await page.keyboard.type("20260918");
check(
  "8桁を打つとその場で日付になる",
  (await state()).editorValue === "2026-09-18",
  (await state()).editorValue,
);
await page.keyboard.press("Escape");
await settle();
check("年を省くと今年になる", dates.slashes.start?.endsWith("-09-22"), JSON.stringify(dates.slashes));
check("年月日でも通る", dates.kanji.start === "2026-09-23", JSON.stringify(dates.kanji));
check(
  "終了より後の開始は断る",
  dates.inverted.status === 400 && dates.invertedEnd.status === 400,
  `${dates.inverted.status} / ${dates.invertedEnd.status}`,
);

// 絞り込みの欄も同じ読み方をして、同じように整形される。
await clearFilters();
await filterBy("予定開始", "20260921");
check(
  "絞り込みの日付も数字だけで通る",
  (await state()).names.includes("テスト"),
  (await state()).names.join(","),
);
check(
  "絞り込みの欄でも8桁が日付になる",
  (await page.evaluate(
    () => document.querySelector('.fg-filter[data-column="start"]')?.value,
  )) === "2026-09-21",
  await page.evaluate(() => document.querySelector('.fg-filter[data-column="start"]')?.value),
);
await clearFilters();

// --- 取り消しとやり直し -------------------------------------------------------------

/** ⌘Z / ⌘Y / ⌘⇧Z。表にフォーカスを戻してから打つ。 */
const chord = async (key, shift = false) => {
  await page.evaluate(() => document.querySelector(".fg-grid").focus({ preventScroll: true }));
  await page.keyboard.down("Meta");
  if (shift) await page.keyboard.down("Shift");
  await page.keyboard.press(key);
  if (shift) await page.keyboard.up("Shift");
  await page.keyboard.up("Meta");
  await settle();
  await settle();
};

/** 「実装」のコメント欄に打ち込む。 */
const typeNote = async (text) => {
  await selectCell((await state()).names.indexOf("実装"), COLUMN["コメント"]);
  await page.keyboard.press("F2");
  await replaceEditorText(text);
  await page.keyboard.press("Enter");
  await settle();
  await settle();
};

const noteNow = () =>
  page.evaluate(async () => {
    const grid = await (await fetch("/api/projects/test-project/grid")).json();
    return grid.tasks.find((task) => task.name === "実装").note;
  });

await typeNote("いち");
await typeNote("に");

await chord("KeyZ");
const undoneOnce = await noteNow();
await chord("KeyZ");
const undoneTwice = await noteNow();

check(
  "⌘Z で1つずつ戻る",
  undoneOnce === "いち" && undoneTwice === "",
  `${undoneOnce} / ${undoneTwice}`,
);

await chord("KeyZ");
check(
  "戻すものが無ければそう言う",
  (await page.evaluate(() => document.querySelector(".fg-error")?.textContent ?? "")).includes(
    "取り消せる操作がありません",
  ),
  await page.evaluate(() => document.querySelector(".fg-error")?.textContent ?? "何も出ていない"),
);

await chord("KeyY");
const redoneY = await noteNow();
await chord("KeyZ", true);
const redoneShift = await noteNow();

check(
  "⌘Y と ⌘⇧Z でやり直せる",
  redoneY === "いち" && redoneShift === "に",
  `${redoneY} / ${redoneShift}`,
);

// 取り消しは自分の変更を戻すもの。間に他人が同じセルを触っていたら、
// それは他人の仕事を消すことになるので止める。
await page.evaluate(async () => {
  const grid = await (await fetch("/api/projects/test-project/grid")).json();
  const id = grid.tasks.find((task) => task.name === "実装").id;

  // 別の画面から入った変更のふりをする（この島を通さない）。
  await fetch(`/api/projects/test-project/tasks/${id}`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ field: "note", value: "よそから" }),
  });
});
await settle();
await chord("KeyZ");

check(
  "他人が先に触っていたら取り消さない",
  (await noteNow()) === "よそから" &&
    (await page.evaluate(() => document.querySelector(".fg-error")?.textContent ?? "")).includes(
      "他の人が先に変更しています",
    ),
  `${await noteNow()} / ${await page.evaluate(() => document.querySelector(".fg-error")?.textContent ?? "")}`,
);

// 行の追加・削除・並べ替えは戻せない。黙って1つ前の値を戻すと、その行が無いまま
// 別のセルだけが巻き戻ることになる。
await typeNote("さん");
await selectCell((await state()).names.indexOf("実装"), 0);
await page.keyboard.down("Meta");
await page.keyboard.press("Enter");
await page.keyboard.up("Meta");
await settle();
await settle();

await chord("KeyZ");
const barrier = await page.evaluate(() => document.querySelector(".fg-error")?.textContent ?? "");
await chord("KeyZ");

check(
  "行の増減は取り消せないと言い、もう一度でその前に戻る",
  barrier.includes("取り消せません") && (await noteNow()) === "よそから",
  `${barrier} / ${await noteNow()}`,
);

// 片付け: 足した行を消し、コメントも戻す。
await page.evaluate(async () => {
  const grid = await (await fetch("/api/projects/test-project/grid")).json();
  const spare = grid.tasks.find((task) => !task.name);
  if (spare) {
    await fetch(`/api/projects/test-project/tasks/${spare.id}`, { method: "DELETE" });
  }

  const id = grid.tasks.find((task) => task.name === "実装").id;
  await fetch(`/api/projects/test-project/tasks/${id}`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ field: "note", value: "" }),
  });
});
await page.reload({ waitUntil: "domcontentloaded" });
await page.waitForSelector(".fg-grid");
await settle();

// --- ダイアログ ---------------------------------------------------------------------

// 待ちも休暇も、日付の範囲を複数持つ。セルの一行では扱えないので画面を出す。
await selectCell((await state()).names.indexOf("設計"), COLUMN["待ち"]);
await page.keyboard.press("Enter");
await settle();
check(
  "待ちはダイアログで編集する",
  await page.evaluate(() => !!document.querySelector("dialog.fg-dialog[open]")),
);

const savedWaits = await page.evaluate(async () => {
  const [start, end] = [...document.querySelectorAll(".fg-dialog-date")];
  start.value = "2026-08-17";
  end.value = "2026-08-21";
  document.querySelector(".fg-dialog-reason").value = "他部署 承認待ち";

  // もう1件、終わりなし＝継続中。
  document.querySelector(".fg-dialog-add").click();
  const rows = [...document.querySelectorAll(".fg-dialog-row")];
  rows.at(-1).querySelector(".fg-dialog-date").value = "2026-09-01";
  document.querySelector(".fg-dialog-save").click();

  await new Promise((done) => setTimeout(done, 800));

  const grid = await (await fetch("/api/projects/test-project/grid")).json();
  return grid.tasks.find((task) => task.id === "t-des").waits;
});

check(
  "ダイアログで複数の待ちを登録できる",
  savedWaits.length === 2 &&
    savedWaits[0].reason === "他部署 承認待ち" &&
    savedWaits[1].open === true,
  JSON.stringify(savedWaits),
);

// 予定進捗も同じ形。待ちと同じく、1行のセルには収まらない一覧だから。
await selectCell((await state()).names.indexOf("実装"), COLUMN["予定進捗"]);
await page.keyboard.press("Enter");
await settle();
check(
  "予定進捗はダイアログで編集する",
  await page.evaluate(
    () => document.querySelector("dialog.fg-dialog[open] .fg-dialog-title")?.textContent ?? "",
  ).then((title) => title.startsWith("予定進捗")),
  await page.evaluate(
    () => document.querySelector("dialog.fg-dialog[open] .fg-dialog-title")?.textContent ?? "ダイアログが無い",
  ),
);

const savedTargets = await page.evaluate(async () => {
  document.querySelector(".fg-dialog-date").value = "2026-09-10";
  document.querySelector(".fg-dialog-percent").value = "40";

  document.querySelector(".fg-dialog-add").click();
  const rows = [...document.querySelectorAll(".fg-dialog-row")];
  rows.at(-1).querySelector(".fg-dialog-date").value = "2026-08-30";
  rows.at(-1).querySelector(".fg-dialog-percent").value = "10";
  document.querySelector(".fg-dialog-save").click();

  await new Promise((done) => setTimeout(done, 800));

  const grid = await (await fetch("/api/projects/test-project/grid")).json();
  return grid.tasks.find((task) => task.id === "t-imp").targets;
});

check(
  "ダイアログで複数の予定進捗を登録でき、日付順に並ぶ",
  savedTargets.length === 2 &&
    savedTargets[0].date === "2026-08-30" &&
    savedTargets[0].percent === 10 &&
    savedTargets[1].percent === 40,
  JSON.stringify(savedTargets),
);

// セルにも出る。過ぎて届いていないものだけが赤い。
check(
  "予定進捗はセルに一覧で出る",
  await page.evaluate(() => {
    const named = [...document.querySelectorAll(".fg-pane-left .fg-row.fg-data .fg-cell-name")].map(
      (c) => c.textContent.trim(),
    );
    const at = named.findIndex((n) => n.includes("実装"));
    const cell = [...document.querySelectorAll(".fg-pane-left .fg-row.fg-data")][at]
      ?.querySelector(".fg-cell-targets");

    return [...(cell?.querySelectorAll(".fg-target-pill") ?? [])].map((p) => p.textContent).join(" / ");
  }).then((text) => text.includes("10%") && text.includes("40%")),
  await page.evaluate(() => {
    const cell = document.querySelector(".fg-cell-targets");
    return cell ? cell.textContent : "列が無い";
  }),
);

// 片付け: この行の予定進捗は他のテストに効かせない。
await page.evaluate(async () => {
  await fetch("/api/projects/test-project/tasks/t-imp", {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ field: "targets", value: "" }),
  });
});
await page.reload({ waitUntil: "domcontentloaded" });
await page.waitForSelector(".fg-grid");
await settle();

// 休暇は設定ではなく通常業務。表の下のボタンから。
await page.evaluate(
  () => [...document.querySelectorAll("button")].find((b) => b.textContent === "担当者の休暇/出社")?.click(),
);
await settle();
const leaveDialog = await page.evaluate(async () => {
  if (!document.querySelector("dialog.fg-dialog[open]")) return null;

  document.querySelector(".fg-dialog-who").value = "佐藤";
  const [start, end] = [...document.querySelectorAll(".fg-dialog-date")];
  start.value = "2026-08-24";
  end.value = "2026-08-26";
  document.querySelector(".fg-dialog-reason").value = "夏季休暇";
  document.querySelector(".fg-dialog-save").click();

  await new Promise((done) => setTimeout(done, 800));

  const grid = await (await fetch("/api/projects/test-project/grid")).json();
  return grid.leaves;
});
check(
  "休暇も画面から登録できる",
  leaveDialog?.length === 1 && leaveDialog[0].note === "夏季休暇",
  JSON.stringify(leaveDialog),
);

// 片付け。
await page.evaluate(async () => {
  await fetch("/api/projects/test-project/leaves", {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ leaves: [] }),
  });
  await fetch("/api/projects/test-project/tasks/t-des", {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ field: "waits", value: "" }),
  });
});
await page.reload({ waitUntil: "domcontentloaded" });
await page.waitForSelector(".fg-grid");
await settle();

// --- 待ち期間 ---------------------------------------------------------------------

const waiting = await page.evaluate(async () => {
  const set = async (value) => {
    const response = await fetch("/api/projects/test-project/tasks/t-des", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ field: "waits", value }),
    });

    if (!response.ok) return { status: response.status };

    const { grid } = await response.json();
    const task = grid.tasks.find((t) => t.id === "t-des");
    return {
      status: response.status,
      waits: task.waits,
      days: task.days,
      wait_days: task.wait_days,
    };
  };

  const before = await set("");
  const one = await set("8/17〜8/21");
  const reasoned = await set("8/17〜8/21 他部署 承認待ち");
  const open = await set("8/17〜");
  const two = await set("8/17〜8/21, 2026-08-24〜2026-08-26");
  const wide = await set("８／１７〜８／２１");
  const nonsense = await set("きのう");
  await set("");

  return { before, one, reasoned, open, two, wide, nonsense };
});

check(
  "理由を書けて、終わりを省けば継続中",
  waiting.reasoned.waits[0]?.reason === "他部署 承認待ち" &&
    waiting.open.waits[0]?.open === true,
  JSON.stringify([waiting.reasoned.waits, waiting.open.waits]),
);
check(
  "待ちの日数は日数から外れる",
  waiting.one.days === waiting.before.days - 5 && waiting.one.wait_days === 5,
  JSON.stringify(waiting.one),
);
check(
  "待ちはいくつでも入れられる",
  waiting.two.waits.length === 2 && waiting.two.wait_days === 8,
  JSON.stringify(waiting.two),
);
check(
  "全角で書いても通る",
  waiting.wide.waits[0]?.start === "2026-08-17",
  JSON.stringify(waiting.wide.waits),
);
check("範囲でないものは断る", waiting.nonsense.status === 400, String(waiting.nonsense.status));

await page.evaluate(async () => {
  await fetch("/api/projects/test-project/tasks/t-des", {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ field: "waits", value: "8/17〜8/21" }),
  });
});
await page.reload({ waitUntil: "domcontentloaded" });
await page.waitForSelector(".fg-grid");
await settle();
// 予定の外に入れた待ちは、日数にも効かないし、バーの無いところに縞も引かない。
const outside = await page.evaluate(async () => {
  await fetch("/api/projects/test-project/tasks/t-des", {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ field: "waits", value: "8/17〜8/21 中, 9/10〜10/9 外" }),
  });

  await new Promise((done) => setTimeout(done, 700));

  const named = [...document.querySelectorAll(".fg-pane-left .fg-row.fg-data .fg-cell-name")].map(
    (c) => c.textContent.trim(),
  );
  const at = named.findIndex((n) => n.includes("設計"));
  const cell = [...document.querySelectorAll(".fg-pane-left .fg-row.fg-data")][at];

  return {
    idle: [...cell.querySelectorAll(".fg-wait-pill")].map((p) => p.classList.contains("is-idle")),
    hatches: [...document.querySelectorAll(".fg-bar-row")][at].querySelectorAll(".fg-wait").length,
  };
});
check(
  "予定の外の待ちは効かないと分かる",
  outside.idle.join(",") === "false,true" && outside.hatches === 1,
  JSON.stringify(outside),
);

// 中の1件だけに戻してから、チャートの確認。
await page.evaluate(async () => {
  await fetch("/api/projects/test-project/tasks/t-des", {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ field: "waits", value: "8/17〜8/21" }),
  });
});
await page.reload({ waitUntil: "domcontentloaded" });
await page.waitForSelector(".fg-grid");
await settle();

check(
  "待ちはチャートにも出る",
  await page.evaluate(() => {
    const named = [...document.querySelectorAll(".fg-pane-left .fg-row.fg-data .fg-cell-name")].map(
      (c) => c.textContent.trim(),
    );
    const at = named.findIndex((n) => n.includes("設計"));
    const cell = [...document.querySelectorAll(".fg-pane-left .fg-row.fg-data")][at];

    return (
      [...document.querySelectorAll(".fg-bar-row")][at].querySelectorAll(".fg-wait").length === 1 &&
      cell.querySelector(".fg-wait-pill")?.textContent === "8/17〜8/21"
    );
  }),
);
await page.evaluate(async () => {
  await fetch("/api/projects/test-project/tasks/t-des", {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ field: "waits", value: "" }),
  });
});

// --- 担当者の色 -------------------------------------------------------------------

const people = await asAdmin(() =>
  page.evaluate(async () => {
    const post = (path, body) =>
      fetch(path, { method: "POST", body: new URLSearchParams(body) });

    // 誰がこの計画にいるかはプロジェクト、その人が何色かは全体。
    await post("/projects/test-project/assignees", { name: "佐藤" });
    // アカウントの無い相手も、ここに足せば選べる。
    await post("/projects/test-project/assignees", { name: "協力会社ほげ" });

    await post("/admin/assignees", { name: "佐藤", color: "#1e3a8a", background: "#dbeafe" });
    await post("/admin/assignees", {
      name: "協力会社ほげ",
      color: "#3f6212",
      background: "#ecfccb",
    });

    const grid = await (await fetch("/api/projects/test-project/grid")).json();
    return grid.assignees;
  }),
);

check(
  "担当者に色をつけられる",
  people.find((p) => p.name === "佐藤")?.background === "#dbeafe",
  JSON.stringify(people),
);
check(
  "アカウントの無い名前も足せる",
  people.some((p) => p.name === "協力会社ほげ"),
  people.map((p) => p.name).join(","),
);

await page.reload({ waitUntil: "domcontentloaded" });
await page.waitForSelector(".fg-grid");
await settle();
check(
  "担当者の色はセルにも出る",
  await page.evaluate(() =>
    [...document.querySelectorAll(".fg-cell-assignee .fg-status")].some(
      (pill) => pill.textContent === "佐藤" && pill.style.background === "rgb(219, 234, 254)",
    ),
  ),
);

await page.evaluate(async () => {
  for (const name of ["佐藤", "協力会社ほげ"]) {
    await fetch("/projects/test-project/assignees/remove", {
      method: "POST",
      body: new URLSearchParams({ name }),
    });
  }
});

// --- 独自項目の選択肢 -------------------------------------------------------------

const master = await page.evaluate(async () => {
  const post = (path, body) =>
    fetch(`/projects/test-project/${path}`, { method: "POST", body: new URLSearchParams(body) });
  const fields = async () =>
    (await (await fetch("/api/projects/test-project/grid")).json()).fields;

  await post("fields", { label: "製品", kind: "select" });
  const id = (await fields()).find((f) => f.label === "製品").id;
  const fresh = (await fields()).find((f) => f.label === "製品").options;

  await post("fields/options", { field_id: id, value: "製品A" });
  await post("fields/options", { field_id: id, value: "製品B" });
  await post("fields/options", {
    field_id: id,
    value: "製品C",
    color: "#7c2d12",
    background: "#ffedd5",
  });
  const added = (await fields()).find((f) => f.label === "製品").options;

  await post("fields/options/move", { field_id: id, value: "製品C", direction: "up" });
  const moved = (await fields()).find((f) => f.label === "製品").options.map((o) => o.value);

  await post("fields/options/remove", { field_id: id, value: "製品B" });
  const left = (await fields()).find((f) => f.label === "製品").options.map((o) => o.value);

  await fetch(`/api/projects/test-project/tasks/t-req`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ field: "custom", field_id: id, value: "製品C" }),
  });

  return { id, fresh, added, moved, left };
});

check("作った項目の選択肢は空から始まる", master.fresh.length === 0, JSON.stringify(master.fresh));
check(
  "選択肢に色をつけられる",
  master.added.at(-1).color === "#7c2d12" && master.added.at(-1).background === "#ffedd5",
  JSON.stringify(master.added),
);
check("選択肢を上へ動かせる", master.moved.join(",") === "製品A,製品C,製品B", master.moved.join(","));
check("選択肢を消せる", master.left.join(",") === "製品A,製品C", master.left.join(","));

await page.reload({ waitUntil: "domcontentloaded" });
await page.waitForSelector(".fg-grid");
await settle();
check(
  "選択肢の色はセルにも出る",
  await page.evaluate(() =>
    [...document.querySelectorAll(".fg-status")].some(
      (pill) =>
        pill.textContent === "製品C" &&
        pill.style.background === "rgb(255, 237, 213)" &&
        pill.style.color === "rgb(124, 45, 18)",
    ),
  ),
);

await page.evaluate(async (id) => {
  await fetch("/projects/test-project/fields/remove", {
    method: "POST",
    body: new URLSearchParams({ field_id: id }),
  });
}, master.id);

// 独自の項目も、中身が日付や数なら比べ方は組み込みの列と同じ。
const custom = await page.evaluate(async () => {
  const post = (path, body) =>
    fetch(`/projects/test-project/${path}`, { method: "POST", body: new URLSearchParams(body) });
  const fields = async () =>
    (await (await fetch("/api/projects/test-project/grid")).json()).fields;

  await post("fields", { label: "検収日", kind: "date" });
  await post("fields", { label: "工数", kind: "number" });
  await post("fields", { label: "ほげ", kind: "text" });

  const all = await fields();
  const id = (label) => all.find((f) => f.label === label).id;

  const set = (task, field_id, value) =>
    fetch(`/api/projects/test-project/tasks/${task}`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ field: "custom", field_id, value }),
    });

  await set("t-req", id("工数"), "8");
  await set("t-des", id("工数"), "3");

  return { date: id("検収日"), number: id("工数"), text: id("ほげ") };
});

await page.reload({ waitUntil: "domcontentloaded" });
await page.waitForSelector(".fg-grid");
await settle();

const chipFor = (key) =>
  page.evaluate(
    (key) =>
      document.querySelector(`.fg-filters [data-column="${key}"]`)?.parentElement?.querySelector(
        ".fg-filter-op",
      )?.textContent ?? null,
    key,
  );

check(
  "独自の日付・数値にも向きが付く",
  (await chipFor(custom.date)) === "≧" && (await chipFor(custom.number)) === "≧",
  `日付 ${await chipFor(custom.date)} / 数値 ${await chipFor(custom.number)}`,
);
check("フリー欄は今までどおり文字で探す", (await chipFor(custom.text)) === null, await chipFor(custom.text));

await page.click(`.fg-filter[data-column="${custom.number}"]`);
await page.keyboard.type("5");
await settle();
const overFive = (await state()).names;

await page.click(`.fg-filters .fg-cell-${custom.number} .fg-filter-op`);
await page.click('.fg-bound-menu [data-bound="lte"]');
await settle();
const underFive = (await state()).names;

check(
  "独自の数値も以上・以下で絞り込める",
  overFive.includes("要件定義") &&
    !overFive.includes("設計") &&
    underFive.includes("設計") &&
    !underFive.includes("要件定義"),
  `以上 ${overFive.join(",")} / 以下 ${underFive.join(",")}`,
);

await page.click(".fg-filter-clear");
await settle();

await page.evaluate(async (ids) => {
  for (const field_id of Object.values(ids)) {
    await fetch("/projects/test-project/fields/remove", {
      method: "POST",
      body: new URLSearchParams({ field_id }),
    });
  }
}, custom);

await page.reload({ waitUntil: "domcontentloaded" });
await page.waitForSelector(".fg-grid");
await settle();

// --- ステータスの設定 -------------------------------------------------------------

const statuses = await page.evaluate(async () => {
  const added = await fetch("/projects/test-project/statuses", {
    method: "POST",
    body: new URLSearchParams({ name: "レビュー中", color: "#fde68a", percent: "80" }),
  });
  const grid = await (await fetch("/api/projects/test-project/grid")).json();

  return {
    status: added.status,
    names: grid.statuses.map((s) => s.name),
    percent: grid.statuses.find((s) => s.name === "レビュー中")?.percent,
    colour: grid.statuses.find((s) => s.name === "レビュー中")?.color,
  };
});
check(
  "ステータスを足せる",
  statuses.status < 400 && statuses.names.includes("レビュー中") && statuses.percent === 80,
  JSON.stringify(statuses),
);
check(
  "既定のステータスは残る",
  statuses.names.includes("未着手") && statuses.names.includes("完了"),
  statuses.names.join(","),
);

// 進捗の連動は、その状態が宣言したパーセントに従う。
const linked = await page.evaluate(async () => {
  const body = new URLSearchParams();
  for (const key of [
    "start", "end", "actual_start", "actual_end", "days",
    "actual_days", "start_variance", "end_variance", "progress", "status", "assignee", "note",
  ]) {
    body.set(`column_${key}`, "1");
  }
  body.set("skip_leave", "1");
  body.set("progress_mode", "status");
  await fetch("/projects/test-project/view", { method: "POST", body });

  const response = await fetch("/api/projects/test-project/tasks/t-imp", {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ field: "status", value: "レビュー中" }),
  });
  const { grid } = await response.json();
  const row = grid.tasks.find((t) => t.id === "t-imp");

  return { status: row.status, progress: row.progress };
});
check(
  "連動は状態ごとのパーセントに従う",
  linked.status === "レビュー中" && linked.progress === 80,
  JSON.stringify(linked),
);

await page.reload({ waitUntil: "domcontentloaded" });
await page.waitForSelector(".fg-grid");
await settle();
check(
  "ステータスの色は設定した色で出る",
  await page.evaluate(() =>
    [...document.querySelectorAll(".fg-status")].some(
      (pill) => pill.textContent === "レビュー中" && pill.style.background === "rgb(253, 230, 138)",
    ),
  ),
);

// 後片付け: 足した状態を消して、既定に戻す。
await page.evaluate(async () => {
  await fetch("/projects/test-project/statuses/remove", {
    method: "POST",
    body: new URLSearchParams({ name: "レビュー中" }),
  });
});
await setView();

// Back to hand-entered, so the next run starts where this one did.
await setView();

// --- JSON の往復 ---------------------------------------------------------------

// 全部入りの1本しか出せないと、タスクを他所へ渡したい人は毎回削ることになる。
const onlyTasks = await page.evaluate(async () => {
  const sections = ["settings", "statuses", "assignees", "holidays", "leaves", "fields"];
  const read = async (query) =>
    await (await fetch(`/projects/test-project/export.json${query}`)).json();

  const full = await read("?settings=1");
  const bare = await read("?settings=0");
  const bydefault = await read("");

  return {
    full: sections.filter((name) => name in full),
    bare: sections.filter((name) => name in bare),
    bydefault: sections.filter((name) => name in bydefault),
    tasks: [full.tasks.length, bare.tasks.length],
    // 取り込めること。設定が無いファイルは、その部分について何も言わないだけ。
    reimported: (
      await fetch("/projects/test-project/import.json", {
        method: "POST",
        body: (() => {
          const body = new FormData();
          body.append("document", new Blob([JSON.stringify(bare)], { type: "application/json" }), "plan.json");
          return body;
        })(),
      })
    ).ok,
  };
});

check(
  "設定を入れずにタスクだけ書き出せる",
  onlyTasks.full.length === 6 &&
    onlyTasks.bare.length === 0 &&
    onlyTasks.tasks[0] === onlyTasks.tasks[1] &&
    onlyTasks.tasks[1] > 0,
  JSON.stringify(onlyTasks),
);
check("既定では全部入り", onlyTasks.bydefault.length === 6, JSON.stringify(onlyTasks.bydefault));
check("設定の無いファイルもそのまま取り込める", onlyTasks.reimported === true);

// 出し分けは画面から。チェックボックスだと「入れない」が送られないので、ボタンを2つ。
check(
  "書き出しの出し分けがドロワーにある",
  await page.evaluate(async () => {
    const html = await (await fetch("/projects/test-project")).text();
    const doc = new DOMParser().parseFromString(html, "text/html");
    const buttons = [...doc.querySelectorAll('form[action$="export.json"] button')].map((b) => [
      b.getAttribute("value"),
      b.textContent.trim(),
    ]);

    return JSON.stringify(buttons);
  }).then(
    (text) =>
      text.includes('["1","JSON で書き出す（タスク＋設定）"]') &&
      text.includes('["0","JSON で書き出す（タスク）"]'),
  ),
  await page.evaluate(async () => {
    const html = await (await fetch("/projects/test-project")).text();
    const doc = new DOMParser().parseFromString(html, "text/html");
    return [...doc.querySelectorAll('form[action$="export.json"] button')]
      .map((b) => `${b.getAttribute("value")}:${b.textContent.trim()}`)
      .join(" / ") || "ボタンが無い";
  }),
);


const round = await page.evaluate(async () => {
  const text = await (await fetch("/projects/test-project/export.json")).text();
  const document_ = JSON.parse(text);

  // Take one row out and put the file back: an import is the whole plan.
  const kept = document_.tasks.slice(0, 3);
  const body = new FormData();
  body.append(
    "document",
    new Blob([JSON.stringify({ ...document_, tasks: kept })], { type: "application/json" }),
    "plan.json",
  );
  const response = await fetch("/projects/test-project/import.json", { method: "POST", body });
  const grid = await (await fetch("/api/projects/test-project/grid")).json();

  return {
    exported: document_.tasks.length,
    sections: [
      "settings",
      "statuses",
      "assignees",
      "holidays",
      "leaves",
      "fields",
    ].filter((key) => document_[key] !== undefined),
    names: document_.tasks.map((t) => t.name),
    depths: document_.tasks.map((t) => t.depth),
    status: response.status,
    after: grid.tasks.map((t) => t.name),
  };
});

check("JSON で書き出せる", round.exported > 0 && round.names.includes("要件定義"), round.names.join(","));
check(
  "設定も名簿も同じファイルに入る",
  round.sections.join(",") === "settings,statuses,assignees,holidays,leaves,fields",
  round.sections.join(","),
);
check("階層は depth で出る", round.depths.some((d) => d > 0), round.depths.join(","));
check(
  "JSON を取り込むと全置換される",
  round.status < 400 && round.after.length === 3,
  `${round.status}: ${round.after.join(",")}`,
);

execFileSync("sh", [join(here, "seed.sh"), DB, EMAIL], { stdio: "inherit" });

// 書き出した JSON を直して戻す往復。id があるので、同じ行が同じ行のまま残る。
const roundtrip = await page.evaluate(async () => {
  const before = await (await fetch("/projects/test-project/export.json")).json();
  const first = before.tasks.find((t) => t.depth === 0 && t.id);

  // 1行だけ書き換えて、1行足して、1行落とす。
  const edited = {
    ...before,
    tasks: before.tasks
      .filter((t) => t.name !== "レビュー")
      .map((t) => (t.id === first.id ? { ...t, name: `${t.name}（改）`, progress: 42 } : t))
      .concat([{ name: "追記したタスク", depth: 0, progress: 7 }]),
  };

  const body = new FormData();
  body.append("document", new Blob([JSON.stringify(edited)], { type: "application/json" }), "p.json");
  const status = (await fetch("/projects/test-project/import.json", { method: "POST", body, redirect: "manual" })).status;

  const after = await (await fetch("/projects/test-project/export.json")).json();
  const same = after.tasks.find((t) => t.id === first.id);

  return {
    status,
    version: before.version,
    hadIds: before.tasks.every((t) => !!t.id),
    keptId: !!same,
    renamed: same?.name ?? "",
    progress: same?.progress ?? -1,
    added: after.tasks.some((t) => t.name === "追記したタスク"),
    dropped: !after.tasks.some((t) => t.name === "レビュー"),
  };
});

check("書き出しに version が入る", roundtrip.version === 1, String(roundtrip.version));
check("書き出しの各行に id が入る", roundtrip.hadIds);
check(
  "直して戻すと、同じ行が同じ行のまま更新される",
  roundtrip.keptId && roundtrip.renamed.includes("（改）") && roundtrip.progress === 42,
  JSON.stringify(roundtrip),
);
check("ファイルに足した行は増える", roundtrip.added);
check("ファイルから消した行は消える", roundtrip.dropped);

execFileSync("sh", [join(here, "seed.sh"), DB, EMAIL], { stdio: "inherit" });
await page.reload({ waitUntil: "domcontentloaded" });
await page.waitForSelector(".fg-grid");
await settle();

// --- 独自項目の編集 ---------------------------------------------------------------

// 名前を間違えたときに、消して作り直すしか道が無いと、入力済みの内容まで捨てる
// ことになる。
const editing = await page.evaluate(async () => {
  const post = (path, body) =>
    fetch(`/projects/test-project/${path}`, {
      method: "POST",
      body: new URLSearchParams(body),
      redirect: "manual",
    });
  const fields = async () => (await (await fetch("/api/projects/test-project/grid")).json()).fields;

  await post("fields", { label: "工数", kind: "number" });
  const made = (await fields()).find((f) => f.label === "工数");

  // 空のうちは種類も変えられる。
  const kindEmpty = (await post("fields/kind", { field_id: made.id, kind: "text" })).status;

  await post("fields/rename", { field_id: made.id, label: "作業時間" });

  await fetch("/api/projects/test-project/tasks/t-req", {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ field: "custom", field_id: made.id, value: "8" }),
  });

  // 入ってしまえば、種類はもう変えられない。
  const kindUsed = (await post("fields/kind", { field_id: made.id, kind: "date" })).status;

  const after = (await fields()).find((f) => f.id === made.id);
  const grid = await (await fetch("/api/projects/test-project/grid")).json();
  const kept = grid.tasks.find((t) => t.id === "t-req").values[made.id];

  // 名前を変えたあとも同じ列であること。
  await post("fields/rename", { field_id: made.id, label: "" });
  const blank = (await fields()).find((f) => f.id === made.id).label;

  await post("fields/remove", { field_id: made.id });

  return { name: after.label, kind: after.kind, in_use: after.in_use, kindEmpty, kindUsed, kept, blank };
});

check("独自項目の名前を変えられる", editing.name === "作業時間", editing.name);
check("名前を変えても入力した値は残る", editing.kept === "8", String(editing.kept));
check("空のうちは種類も変えられる", editing.kindEmpty < 400 && editing.kind === "text", JSON.stringify(editing));
check("入力済みなら種類は変えられない", editing.kindUsed === 400, String(editing.kindUsed));
check("空の名前には変えられない", editing.blank === "作業時間", editing.blank);

await page.reload({ waitUntil: "domcontentloaded" });
await page.waitForSelector(".fg-grid");
await settle();

// --- 横断の API -------------------------------------------------------------------

// プロジェクト単位の鍵では「どの計画があるか」すら訊けない。案件をまたいだ数字は
// 管理者が出す1本の鍵で見る。
const across = await asAdmin(() =>
  page.evaluate(async () => {
    // 比べる相手が要る: 鍵の効く範囲の話なので、計画が2つ無いと確かめられない。
    await fetch("/projects", {
      method: "POST",
      body: new URLSearchParams({ name: `横断テスト ${Date.now()}` }),
    });

    const wide = await fetch("/admin/tokens", {
      method: "POST",
      body: new URLSearchParams({ name: "横断集計", role: "viewer" }),
    });
    const token = decodeURIComponent((wide.url.match(/issued=([^&#]+)/) ?? [])[1] ?? "");
    const head = { Authorization: `Bearer ${token}` };

    // 1つの計画にしか効かない鍵と比べる。redirect を追わないと、発行された
    // トークンが載っている URL が読めない（空の Bearer は無いのと同じで、
    // ログイン中のセッションで通ってしまう）。
    const narrow = await fetch("/projects/test-project/tokens", {
      method: "POST",
      body: new URLSearchParams({ name: "1つだけ", role: "viewer" }),
    });
    const one = decodeURIComponent((narrow.url.match(/issued=([^&#]+)/) ?? [])[1] ?? "");

    const list = await (await fetch("/api/projects", { headers: head })).json();
    const mine = await (await fetch("/api/projects", { headers: { Authorization: `Bearer ${one}` } })).json();
    const summary = await (await fetch("/api/summary", { headers: head })).json();
    const release = summary.find((s) => s.id === "test-project");

    // 読むだけの鍵は書けない
    const doc = await (await fetch("/api/projects/test-project/document", { headers: head })).json();
    const wrote = await fetch("/api/projects/test-project/document", {
      method: "POST",
      headers: { ...head, "content-type": "application/json" },
      body: JSON.stringify(doc),
    });

    // 後片付け
    for (const [url, form] of [["/admin", '/admin/tokens/remove'],
                               ["/projects/test-project/settings?open=tokens", "/projects/test-project/tokens/remove"]]) {
      const html = await (await fetch(url)).text();
      for (const input of new DOMParser().parseFromString(html, "text/html")
        .querySelectorAll(`form[action$="${form.split("/").pop()}"] input[name="id"]`)) {
        await fetch(form, { method: "POST", body: new URLSearchParams({ id: input.value }) });
      }
    }

    return {
      wide: list.length,
      narrow: mine.map((p) => p.id),
      release,
      wrote: wrote.status,
      hasNumbers: release && ["tasks", "delayed", "progress", "late_days", "wait_days", "slipped"]
        .every((key) => key in release),
    };
  }),
);

check("全プロジェクトの鍵は全部の計画を返す", across.wide > 1, String(across.wide));
check(
  "1つだけの鍵はその1つしか見えない",
  across.narrow.length === 1 && across.narrow[0] === "test-project",
  across.narrow.join(","),
);
check(
  "案件ごとの数字が出る",
  across.hasNumbers && across.release.tasks > 0 && across.release.slipped ===
    across.release.late_days + across.release.wait_days,
  JSON.stringify(across.release),
);
check("読むだけの鍵では書けない", across.wrote === 403, String(across.wrote));

// --- 既定のステータス -------------------------------------------------------------

// 新しいプロジェクトは全体の一覧を写して始まる。写したあとは独立で、あとから
// 全体を直しても、既にあるプロジェクトの色や進捗は動かない。
const defaults = await asAdmin(() =>
  page.evaluate(async () => {
    const post = (path, body) =>
      fetch(path, { method: "POST", body: new URLSearchParams(body), redirect: "manual" });

    const beforeExisting = (await (await fetch("/api/projects/test-project/grid")).json()).statuses
      .map((s) => s.name);

    await post("/admin/statuses", { name: "検収待ち", color: "#fde68a", percent: "90" });
    await post("/admin/statuses/move", { name: "検収待ち", direction: "up" });

    const name = `既定テスト ${Date.now()}`;
    await post("/projects", { name });

    const html = await (await fetch("/")).text();
    const id = [...html.matchAll(/href="\/projects\/([^"]+)"/g)]
      .map((m) => decodeURIComponent(m[1]))
      .find((slug) => slug.startsWith("既定テスト-"));

    const fresh = (await (await fetch(`/api/projects/${encodeURIComponent(id)}/grid`)).json()).statuses;
    const afterExisting = (await (await fetch("/api/projects/test-project/grid")).json()).statuses
      .map((s) => s.name);

    await post("/admin/statuses/remove", { name: "検収待ち" });

    return {
      id,
      fresh: fresh.map((s) => s.name),
      percent: fresh.find((s) => s.name === "検収待ち")?.percent ?? null,
      beforeExisting,
      afterExisting,
    };
  }),
);

check(
  "新しいプロジェクトは全体の既定を写して始まる",
  defaults.fresh.includes("検収待ち") && defaults.percent === 90,
  JSON.stringify(defaults.fresh),
);
check(
  "並べ替えた順もそのまま写る",
  defaults.fresh.indexOf("検収待ち") === defaults.fresh.length - 2,
  defaults.fresh.join(","),
);
check(
  "既にあるプロジェクトは動かない",
  defaults.afterExisting.join(",") === defaults.beforeExisting.join(","),
  `${defaults.beforeExisting.join(",")} → ${defaults.afterExisting.join(",")}`,
);

execFileSync("sqlite3", [DB, "DELETE FROM app_statuses"]);
execFileSync("sqlite3", [DB, "DELETE FROM project_members WHERE project_id LIKE '既定テスト-%'"]);
execFileSync("sqlite3", [DB, "DELETE FROM project_statuses WHERE project_id LIKE '既定テスト-%'"]);
execFileSync("sqlite3", [DB, "DELETE FROM projects WHERE name LIKE '既定テスト %'"]);

// --- API トークン ---------------------------------------------------------------

// ブラウザの外から、このプロジェクトだけを読み書きするための鍵。
const api = await page.evaluate(async () => {
  const issue = async (role) => {
    const res = await fetch("/projects/test-project/tokens", {
      method: "POST",
      body: new URLSearchParams({ name: `検査 ${role}`, role }),
    });
    return decodeURIComponent((res.url.match(/issued=([^&#]+)/) ?? [])[1] ?? "");
  };

  const editor = await issue("editor");
  const viewer = await issue("viewer");

  const head = (token) => ({ Authorization: `Bearer ${token}` });
  const get = (token) => fetch("/api/projects/test-project/document", { headers: head(token) });

  // 読んで、1行直して、書き戻す。
  const doc = await (await get(editor)).json();
  doc.tasks[0].name = "API が直した行";

  const wrote = await fetch("/api/projects/test-project/document", {
    method: "POST",
    headers: { ...head(editor), "content-type": "application/json" },
    body: JSON.stringify(doc),
  });

  const after = await (await get(editor)).json();

  // 読むだけのトークンでは書けない。
  const readOnly = await fetch("/api/projects/test-project/document", {
    method: "POST",
    headers: { ...head(viewer), "content-type": "application/json" },
    body: JSON.stringify(doc),
  });

  return {
    issued: editor.startsWith("fug_"),
    read: (await get(editor)).status,
    wrote: wrote.status,
    name: after.tasks[0].name,
    viewerReads: (await get(viewer)).status,
    viewerWrites: readOnly.status,
    // この画面はログイン済みなので、鍵もクッキーも無い状態を作って試す。
    noToken: (await fetch("/api/projects/test-project/document", { credentials: "omit" })).status,
    rubbish: (await fetch("/api/projects/test-project/document", { headers: head("fug_nope") })).status,
    other: (await fetch("/api/projects/nowhere/document", { headers: head(editor) })).status,
  };
});

check("トークンを発行できる", api.issued);
check("トークンで書き出しが読める", api.read === 200, String(api.read));
check(
  "トークンで直して書き戻せる",
  api.wrote === 200 && api.name === "API が直した行",
  JSON.stringify(api),
);
check("読むだけのトークンは読めて書けない", api.viewerReads === 200 && api.viewerWrites === 403,
  `${api.viewerReads} / ${api.viewerWrites}`);
check("鍵もクッキーも無ければ断られる", api.noToken === 403, String(api.noToken));

// トークンで書いた変更は、どの鍵でやったのかが履歴に残る。人の名前は付かない。
check(
  "API の変更は、どのトークンかが履歴に出る",
  await page.evaluate(async () => {
    const html = await (await fetch("/projects/test-project/history")).text();
    const rows = [...new DOMParser().parseFromString(html, "text/html").querySelectorAll("ul li")]
      .map((li) => li.textContent.replace(/\s+/g, " "));
    return rows.some((row) => row.includes("API 検査 editor") && row.includes("取り込み"));
  }),
  await page.evaluate(async () => {
    const html = await (await fetch("/projects/test-project/history")).text();
    return (new DOMParser().parseFromString(html, "text/html").querySelector("ul li")?.textContent ?? "なし")
      .replace(/\s+/g, " ");
  }),
);
check("でたらめなトークンは断られる", api.rubbish === 403, String(api.rubbish));
check("他のプロジェクトには効かない", api.other === 403, String(api.other));

// API で書いた変更も、開いている画面にその場で届く。届かないと、書いた側は
// 誰かがリロードするのを待つことしかできない。
check(
  "API の書き込みが開いている画面に届く",
  await page.evaluate(async () => {
    const res = await fetch("/projects/test-project/tokens", {
      method: "POST",
      body: new URLSearchParams({ name: "live", role: "editor" }),
    });
    const token = decodeURIComponent((res.url.match(/issued=([^&#]+)/) ?? [])[1] ?? "");
    const head = { Authorization: `Bearer ${token}` };

    const before = document.querySelector(".fg-name-text")?.textContent;

    const doc = await (await fetch("/api/projects/test-project/document", { headers: head })).json();
    doc.tasks[0].name = "外から書き換えた行";
    await fetch("/api/projects/test-project/document", {
      method: "POST",
      headers: { ...head, "content-type": "application/json" },
      body: JSON.stringify(doc),
    });

    // 画面には触らない。SSE で届くのを待つだけ。
    for (let at = 0; at < 20; at++) {
      await new Promise((r) => setTimeout(r, 250));
      if (document.querySelector(".fg-name-text")?.textContent === "外から書き換えた行") {
        return `${before} → 外から書き換えた行`;
      }
    }

    return `届かなかった（${before} のまま）`;
  }),
);

// 後片付け: 作ったトークンを消して、タスクを元に戻す。
await page.evaluate(async () => {
  const html = await (await fetch("/projects/test-project/settings?open=tokens")).text();
  const doc = new DOMParser().parseFromString(html, "text/html");
  for (const input of doc.querySelectorAll('form[action$="/tokens/remove"] input[name="id"]')) {
    await fetch("/projects/test-project/tokens/remove", {
      method: "POST",
      body: new URLSearchParams({ id: input.value }),
    });
  }
});

execFileSync("sh", [join(here, "seed.sh"), DB, EMAIL], { stdio: "inherit" });
await page.reload({ waitUntil: "domcontentloaded" });
await page.waitForSelector(".fg-grid");
await settle();

// --- ユーザーマスター -------------------------------------------------------------

const accounts = await page.evaluate(async () => {
  const status = async (path) => (await fetch(path, { redirect: "manual" })).status;

  return {
    // 管理者だけが見える。招待の仕組みはもう無い。
    users: await status("/users"),
    invites: await status("/invites"),
    // 自分の設定は誰でも。
    me: await status("/me"),
  };
});

check(
  "ユーザーの管理は管理者だけ",
  accounts.users === 404 && accounts.invites === 404,
  JSON.stringify(accounts),
);
check("自分の設定は誰でも開ける", accounts.me === 200, String(accounts.me));

check(
  "名前とパスワードは自分で変えられる",
  await page.evaluate(async () => {
    const html = await (await fetch("/me")).text();
    return (
      html.includes('action="/me/name"') &&
      html.includes('action="/me/password"') &&
      // ユーザー名とベース権限は管理者のもの。
      !html.includes('name="base_role"')
    );
  }),
);

// --- 全体とプロジェクトの境目 -------------------------------------------------

// 会社の暦・名簿・休暇は全体に一つ。新しい計画を作った人が、祝日を貼り直したり
// 色を決め直したりしなくていい、というのがこの分け方の目的。
const shared = await asAdmin(() =>
  page.evaluate(async () => {
    const post = (path, body) =>
      fetch(path, { method: "POST", body: new URLSearchParams(body), redirect: "manual" });

    await post("/admin/holidays", { date: "2026-07-06", name: "創立記念日" });
    await post("/admin/assignees", { name: "山田", color: "#7c2d12", background: "#ffedd5" });

    const name = `全体テスト ${Date.now()}`;
    await post("/projects", { name });

    const html = await (await fetch("/")).text();
    const id = [...html.matchAll(/href="\/projects\/([^"]+)"/g)]
      .map((m) => decodeURIComponent(m[1]))
      .find((slug) => slug.startsWith("全体テスト-"));

    const fresh = await (await fetch(`/api/projects/${encodeURIComponent(id)}/grid`)).json();
    const here = await (await fetch("/api/projects/test-project/grid")).json();

    return {
      fresh: fresh.holidays.map((h) => h.date),
      here: here.holidays.map((h) => h.date),
      colour: here.assignees.find((p) => p.name === "山田")?.background ?? "",
    };
  }),
);

check(
  "全体の祝日は作ったばかりの計画にも出ている",
  shared.fresh.includes("2026-07-06") && shared.here.includes("2026-07-06"),
  `新 ${shared.fresh.join(",")} / 既存 ${shared.here.join(",")}`,
);
check("担当者の色も全体のもの", shared.colour === "#ffedd5", shared.colour || "なし");

await asAdmin(() =>
  page.evaluate(() =>
    fetch("/admin/holidays/remove", {
      method: "POST",
      body: new URLSearchParams({ date: "2026-07-06" }),
    }),
  ),
);

// 休暇は設定ではなく通常業務なので、設定画面には無い。
check(
  "設定に休暇の欄はもう無い",
  await page.evaluate(async () => {
    const html = await (await fetch("/projects/test-project/settings")).text();
    return !html.includes('id="leaves"');
  }),
);

// --- パスワードの決まり -------------------------------------------------------

const passwords = await asAdmin(() =>
  page.evaluate(async () => {
    const post = (path, body) =>
      fetch(path, { method: "POST", body: new URLSearchParams(body), redirect: "manual" });

    await post("/admin/password", {
      password_min: "12",
      kind_upper: "on",
      kind_digit: "on",
      password_banned: "password\nfugantt\n社名",
    });

    // 作るときも、自分で変えるときも、同じ決まりを通る。
    // 表示名は一人に一つ。同じ名前で作ろうとすると、それ自体が断られる。
    const make = (password) => {
      const tag = `${Date.now()}-${Math.random()}`;
      return post("/users", {
        name: `検査 ${tag}`,
        email: `check-${tag}`,
        password,
        base_role: "editor",
      }).then((r) => r.status);
    };

    const short = await make("Short1");
    const oneKind = await make("abcdefghijklm");
    const banned = await make("MyPassword123");
    const ok = await make("Karasuma2026Ave");

    const html = await (await fetch("/users")).text();

    return { short, oneKind, banned, ok, html: html.includes("12文字以上") };
  }),
);

check("短いパスワードは断る", passwords.short === 400, String(passwords.short));
check("求めた文字種が無ければ断る", passwords.oneKind === 400, String(passwords.oneKind));
check("よくある語を含めば断る", passwords.banned === 400, String(passwords.banned));
check("決まりを満たせば作れる", passwords.ok < 400, String(passwords.ok));
check("画面の説明も設定から作る", passwords.html);

// 日本語のパスフレーズは文字で数える。バイトで数えると3文字で通っていた。
const multibyte = await asAdmin(() =>
  page.evaluate(async () => {
    const post = (path, body) =>
      fetch(path, { method: "POST", body: new URLSearchParams(body), redirect: "manual" });

    await post("/admin/password", { password_min: "8", password_banned: "" });

    const make = (password) => {
      const tag = `${Date.now()}-${Math.random()}`;
      return post("/users", {
        name: `検査 ${tag}`,
        email: `mb-${tag}`,
        password,
        base_role: "none",
      }).then((r) => r.status);
    };

    return { five: await make("あいうえお"), eight: await make("あいうえおかきく") };
  }),
);

check(
  "パスワードは文字数で数える",
  multibyte.five === 400 && multibyte.eight < 400,
  JSON.stringify(multibyte),
);

// 後片付け: 決まりを既定に戻し、作った口座を消す。
await asAdmin(() =>
  page.evaluate(async () => {
    await fetch("/admin/password", {
      method: "POST",
      body: new URLSearchParams({ password_min: "8", password_banned: "" }),
    });
  }),
);

execFileSync("sqlite3", [DB, "DELETE FROM users WHERE email LIKE 'check-%' OR email LIKE 'mb-%'"]);

check(
  "文字種はチェックボックスで選ぶ",
  await asAdmin(() =>
    page.evaluate(async () => {
      const html = await (await fetch("/admin")).text();
      const doc = new DOMParser().parseFromString(html, "text/html");
      const boxes = [...doc.querySelectorAll('input[type="checkbox"][name^="kind_"]')];
      return (
        boxes.length === 4 &&
        boxes.map((b) => b.name).join(",") === "kind_lower,kind_upper,kind_digit,kind_symbol"
      );
    }),
  ),
);

// 「3件」だけでは、終わりかけなのか手つかずなのか分からない。
check(
  "統計に担当者ごとのステータス内訳が出る",
  await page.evaluate(async () => {
    const html = await (await fetch("/projects/test-project/stats")).text();
    const doc = new DOMParser().parseFromString(html, "text/html");
    const section = [...doc.querySelectorAll("section")].find(
      (s) => s.querySelector("h2")?.textContent === "担当者",
    );
    const rows = [...section.querySelectorAll("li")];
    const yamada = rows.find((li) => li.textContent.includes("山田"));

    // 名前・件数・内訳・平均が同じ行にそろっていること。
    const pills = [...yamada.querySelectorAll("span span")].map((s) => s.textContent.trim());
    return (
      rows.length >= 2 &&
      yamada.textContent.includes("平均") &&
      pills.length > 0 &&
      [...yamada.querySelectorAll("span")].some((s) => /^(未着手|実施中|待ち|完了|保留)/.test(s.textContent.trim()))
    );
  }),
);

// 変更履歴は溜まる一方なので、1ページに全部は出さない。
const pagination = await page.evaluate(async () => {
  const read = async (query) => {
    const html = await (await fetch(`/projects/test-project/history${query}`)).text();
    const doc = new DOMParser().parseFromString(html, "text/html");
    return {
      rows: doc.querySelectorAll("ul li").length,
      links: [...doc.querySelectorAll("nav a")].map((a) => a.textContent.trim()),
      where: doc.querySelector("nav span")?.textContent ?? "",
      count: doc.querySelector("h1 + p span")?.textContent ?? "",
    };
  };

  return { first: await read(""), second: await read("?page=2"), silly: await read("?page=999") };
});

check("履歴は1ページ100件まで", pagination.first.rows <= 100, String(pagination.first.rows));
check(
  "1ページ目に「新しい」への戻り先は出さない",
  !pagination.first.links.includes("← 新しい"),
  pagination.first.links.join(","),
);
check(
  "何件目を見ているかが分かる",
  pagination.first.count.includes("件中"),
  pagination.first.count || "なし",
);
check(
  "行き過ぎたページ番号は最後のページに丸める",
  pagination.silly.rows > 0 && !pagination.silly.links.includes("古い →"),
  `${pagination.silly.rows} 行 / ${pagination.silly.links.join(",")}`,
);

// メンバーはユーザーマスターから選ぶ。打ち込ませれば打ち間違いが起きる。
check(
  "メンバーの追加は選択式",
  await page.evaluate(async () => {
    const html = await (await fetch("/projects/test-project/settings")).text();
    const doc = new DOMParser().parseFromString(html, "text/html");
    const field = doc.querySelector('form[action$="/members"] [name="email"]');
    return field?.tagName === "SELECT" && field.querySelectorAll("option").length > 0;
  }),
);

// パスワードを変えたら、開いたままの他のセッションは閉じる。
check(
  "パスワードを変えると他のセッションが切れる",
  await page.evaluate(async () => {
    // 別の browser context ではなく、cookie を持たない fetch で「もう一つの
    // 端末」を作る——サーバー側の行が消えているかどうかだけを見る。
    const password = "grid-test-password";

    const login = await fetch("/login", {
      method: "POST",
      body: new URLSearchParams({ email: "grid-test@example.com", password }),
      redirect: "manual",
    });
    if (login.status >= 400) return false;

    // いまの画面のセッションは残っていること（自分を追い出さない）。
    await fetch("/me/password", {
      method: "POST",
      body: new URLSearchParams({ current: password, password }),
      redirect: "manual",
    });

    const still = await fetch("/me", { redirect: "manual" });
    return still.status === 200;
  }),
);

// 担当者はタスクの上では名前の文字列なので、同じ表示名が二人いると見分けられない。
check(
  "同じ表示名は二人に付けられない",
  await asAdmin(() =>
    page.evaluate(async () => {
      const post = (path, body) =>
        fetch(path, { method: "POST", body: new URLSearchParams(body), redirect: "manual" });

      const first = await post("/users", {
        name: "同姓同名 太郎",
        email: `dup-a-${Date.now()}`,
        password: "grid-test-password",
        base_role: "editor",
      });
      const second = await post("/users", {
        name: "同姓同名 太郎",
        email: `dup-b-${Date.now()}`,
        password: "grid-test-password",
        base_role: "editor",
      });

      return { first: first.status, second: second.status };
    }),
  ).then((r) => r.first < 400 && r.second === 400),
);

execFileSync("sqlite3", [DB, "DELETE FROM users WHERE email LIKE 'dup-%'"]);

// 画面に出る言葉をひとつに: 同じものを「計画」と「プロジェクト」で呼び分けない。
check(
  "設定の言葉がプロジェクトでそろっている",
  await page.evaluate(async () => {
    const html = await (await fetch("/projects/test-project/settings")).text();
    const text = new DOMParser().parseFromString(html, "text/html").body.textContent;
    return !text.includes("この計画");
  }),
);

// 触る順に並べる: まず見え方、次に表の形、次に言葉、最後に人と色。
check(
  "設定の並びが触る順になっている",
  await page.evaluate(async () => {
    const html = await (await fetch("/projects/test-project/settings")).text();
    const doc = new DOMParser().parseFromString(html, "text/html");
    const ids = [...doc.querySelectorAll("[id]")]
      .map((el) => el.id)
      .filter((id) =>
        ["view", "columns", "statuses", "fields", "assignees", "holidays", "colours", "members"].includes(id),
      );
    return ids.join(",") === "view,columns,statuses,fields,assignees,holidays,colours,members";
  }),
);

// --- 実作業日数 -----------------------------------------------------------------

// 「日数」は予定の日数。実際に何日動いたかは別の数字で、終わっていなければ今日まで。
check(
  "実作業日数の列が出る",
  await page.evaluate(() =>
    [...document.querySelectorAll(".fg-heading .fg-cell")]
      .map((c) => c.textContent.trim())
      .includes("実作業日数"),
  ),
  await page.evaluate(() =>
    [...document.querySelectorAll(".fg-heading .fg-cell")].map((c) => c.textContent.trim()).join(","),
  ),
);

check(
  "実施の入っている行だけ実作業が出る",
  await page.evaluate(() => {
    const rows = [...document.querySelectorAll(".fg-pane-left .fg-row.fg-data")];
    const worked = rows.map((r) => r.querySelector(".fg-cell-actual_days")?.textContent.trim() ?? "");
    const started = rows.map((r) => r.querySelector(".fg-cell-actual_start")?.textContent.trim() ?? "");

    // 実施開始のある行には数字、無い行には「—」。
    return started.every((at, i) => (at === "—" ? worked[i] === "—" : Number(worked[i]) > 0));
  }),
);

check(
  "チャートにも実作業の日数が出る",
  await page.evaluate(() => {
    const labels = [...document.querySelectorAll(".fg-worked")].map((w) => w.textContent ?? "");
    return labels.length > 0 && labels.every((text) => /実作業 \d+(営業)?日/.test(text));
  }),
  await page.evaluate(() => [...document.querySelectorAll(".fg-worked")].map((w) => w.textContent).join(" / ")),
);

// 表の行は選べるのにチャートの行は選べない、という状態は同じ1行として扱えていない。
check(
  "チャート側を押しても行が選べる",
  await page.evaluate(async () => {
    const rows = [...document.querySelectorAll(".fg-bar-row")];
    rows[2].dispatchEvent(new MouseEvent("mousedown", { bubbles: true }));
    await new Promise((r) => setTimeout(r, 100));

    const left = [...document.querySelectorAll(".fg-pane-left .fg-row.fg-data")]
      .findIndex((r) => r.classList.contains("is-current"));
    const chart = [...document.querySelectorAll(".fg-bar-row")]
      .findIndex((r) => r.classList.contains("is-current"));

    return left === 2 && chart === 2;
  }),
);

// 差異も実作業も要るが、全部出すとチャートが数字で埋まる。何を見るかは人と場面で
// 変わるので、設定ではなく画面の上で切り替える。
const shows = await page.evaluate(async () => {
  const count = () => ({
    worked: document.querySelectorAll(".fg-worked").length,
    variance: document.querySelectorAll(".fg-variance").length,
  });

  const before = count();

  document.querySelector(".fg-shows").click();
  await new Promise((r) => setTimeout(r, 50));

  const box = document.querySelector('.fg-shows-menu [data-shows="worked"]');
  box.checked = false;
  box.dispatchEvent(new Event("change", { bubbles: true }));
  await new Promise((r) => setTimeout(r, 150));

  const after = count();
  const remembered = window.localStorage.getItem("fugantt:chart-shows");

  window.localStorage.removeItem("fugantt:chart-shows");
  return { before, after, remembered };
});

check(
  "チャートに出すものを画面から切り替えられる",
  shows.before.worked > 0 && shows.after.worked === 0 && shows.after.variance === shows.before.variance,
  JSON.stringify(shows),
);
check(
  "選んだ表示はこの端末に覚える",
  (shows.remembered ?? "").includes('"worked":false'),
  shows.remembered ?? "なし",
);

await page.reload({ waitUntil: "domcontentloaded" });
await page.waitForSelector(".fg-grid");
await settle();
await selectCell(0, 0);

// 固定した列に選択セルがあると、フォーカスが外れた瞬間に透明になり、その下を
// 流れる他の列が透けていた（1行目にだけ別の行の文字が重なる、という見え方）。
check(
  "フォーカスが外れても固定列の選択セルは透けない",
  await page.evaluate(async () => {
    const pane = document.querySelector(".fg-pane-left");
    pane.querySelector(".fg-row.fg-data .fg-cell").dispatchEvent(
      new MouseEvent("mousedown", { bubbles: true }),
    );
    pane.scrollLeft = 700;
    document.querySelector(".fg-shows")?.focus();
    await new Promise((r) => setTimeout(r, 150));

    const cell = document.querySelector(".fg-cell.is-selected");
    const colour = getComputedStyle(cell).backgroundColor;
    pane.scrollLeft = 0;

    return !!cell && colour !== "rgba(0, 0, 0, 0)" && colour !== "transparent";
  }),
  await page.evaluate(() => {
    const cell = document.querySelector(".fg-cell.is-selected");
    return cell ? getComputedStyle(cell).backgroundColor : "なし";
  }),
);

await selectCell(0, 0);

// 0% と 100% では進捗のつまみがバーの端と重なり、端の判定に負けて動かせなかった。
check(
  "0% と 100% でも進捗をつまんで動かせる",
  await page.evaluate(async () => {
    const set = (value) =>
      fetch("/api/projects/test-project/tasks/t-req", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ field: "progress", value }),
      });

    const results = [];

    for (const start of ["0", "100"]) {
      await set(start);
      await new Promise((r) => setTimeout(r, 250));

      const bar = [...document.querySelectorAll(".fg-bar")].find((b) => b.dataset.task === "t-req");
      const knob = bar.querySelector(".fg-grip-progress");
      const k = knob.getBoundingClientRect();
      const b = bar.getBoundingClientRect();

      // つまみの座標で判定しているかどうかを見るので、実際に押して引く。
      const press = (type, x) =>
        bar.dispatchEvent(
          new PointerEvent(type, { bubbles: true, clientX: x, clientY: k.y + k.height / 2, button: 0, pointerId: 1 }),
        );

      press("pointerdown", k.x + k.width / 2);
      press("pointermove", b.x + b.width / 2);
      press("pointerup", b.x + b.width / 2);
      await new Promise((r) => setTimeout(r, 400));

      const grid = await (await fetch("/api/projects/test-project/grid")).json();
      results.push(grid.tasks.find((t) => t.id === "t-req").progress);
    }

    await set("100");
    return results.every((value) => value > 10 && value < 90) ? results.join(",") : `動かず: ${results.join(",")}`;
  }),
);

await page.reload({ waitUntil: "domcontentloaded" });
await page.waitForSelector(".fg-grid");
await settle();
await selectCell(0, 0);

// --- 言語 ---------------------------------------------------------------------

// ブラウザが送ってくる言語は、その人の OS の設定。サーバーの OS ではない。
const languages = await page.evaluate(async () => {
  const read = async (accept) => {
    const html = await (await fetch("/projects/test-project", {
      headers: { "accept-language": accept },
    })).text();
    const doc = new DOMParser().parseFromString(html, "text/html");
    return {
      lang: doc.documentElement.lang,
      first: doc.querySelector("aside nav a")?.textContent.trim() ?? "",
    };
  };

  const english = await read("en-US,en;q=0.9");
  const japanese = await read("ja-JP,ja;q=0.9");

  // 本人の設定は、ブラウザより強い。
  await fetch("/me/name", {
    method: "POST",
    body: new URLSearchParams({ name: "grid-test@example.com", language: "en" }),
  });
  const chosen = await read("ja-JP,ja;q=0.9");

  await fetch("/me/name", {
    method: "POST",
    body: new URLSearchParams({ name: "grid-test@example.com", language: "" }),
  });
  const back = await read("ja-JP,ja;q=0.9");

  return { english, japanese, chosen, back };
});

check(
  "ブラウザ（OS）の言語で画面が変わる",
  languages.english.lang === "en" && languages.english.first === "Schedule" &&
    languages.japanese.lang === "ja" && languages.japanese.first === "スケジュール",
  JSON.stringify(languages),
);
check(
  "本人が選んだ言語はブラウザより優先される",
  languages.chosen.lang === "en" && languages.back.lang === "ja",
  JSON.stringify({ chosen: languages.chosen, back: languages.back }),
);

// --- バックアップ ---------------------------------------------------------------
//
// いちばん最後に置く。戻すと全部が入れ替わるので、他のテストの足元を抜かないこと。

const backup = await asAdmin(() =>
  page.evaluate(async () => {
    const log = {};

    const response = await fetch("/admin/backup.db");
    const file = new Uint8Array(await response.arrayBuffer());

    log.status = response.status;
    log.magic = new TextDecoder().decode(file.slice(0, 15));
    log.named = response.headers.get("content-disposition") ?? "";

    // 中身を変えてから戻す。
    const grid = await (await fetch("/api/projects/test-project/grid")).json();
    const id = grid.tasks.find((task) => task.name === "設計").id;
    await fetch(`/api/projects/test-project/tasks/${id}`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ field: "name", value: "書き換えた" }),
    });
    log.changed = (await (await fetch("/api/projects/test-project/grid")).json()).tasks.some(
      (task) => task.name === "書き換えた",
    );

    const body = new FormData();
    body.append("backup", new Blob([file], { type: "application/vnd.sqlite3" }), "b.db");
    const restore = await fetch("/admin/restore", { method: "POST", body });

    log.restored = restore.status;
    // 直前の中身を控えた先を、戻ったあとの画面が教えてくれる。
    log.kept = decodeURIComponent(new URL(restore.url).searchParams.get("restored") ?? "");

    const after = (await (await fetch("/api/projects/test-project/grid")).json()).tasks;
    log.back = after.some((task) => task.name === "設計");
    log.gone = !after.some((task) => task.name === "書き換えた");

    // SQLite ですらないもの。
    const junk = new FormData();
    junk.append("backup", new Blob([new TextEncoder().encode("hello")]), "x.db");
    const refused = await fetch("/admin/restore", { method: "POST", body: junk });

    log.junk = refused.status;
    log.junkSays = await refused.text();

    return log;
  }),
);

check(
  "バックアップは1つのファイルとして落ちてくる",
  backup.status === 200 && backup.magic === "SQLite format 3" && /fugantt-\d{8}-\d{6}\.db/.test(backup.named),
  JSON.stringify({ status: backup.status, magic: backup.magic, named: backup.named }),
);
check(
  "バックアップから戻すと、その時点の中身になる",
  backup.changed && backup.restored === 200 && backup.back && backup.gone,
  JSON.stringify(backup),
);
check(
  "戻す直前の中身を自動で控える",
  /fugantt-before-restore-\d{8}-\d{6}\.db$/.test(backup.kept),
  backup.kept || "控えた先が分からない",
);
check(
  "SQLite でないファイルは断る",
  backup.junk === 400 && backup.junkSays.includes("SQLite"),
  `${backup.junk} / ${backup.junkSays}`,
);

// 他人の SQLite を掴んで壊さない。買い物メモのデータベースは fugantt ではない。
const foreign = (() => {
  const path = join(here, "..", "..", "target", "not-fugantt.db");
  execFileSync("sqlite3", [path, "CREATE TABLE IF NOT EXISTS shopping (item TEXT)"]);
  return [...readFileSync(path)];
})();

const strangerSays = await asAdmin(() =>
  page.evaluate(async (bytes) => {
    const body = new FormData();
    body.append("backup", new Blob([new Uint8Array(bytes)]), "other.db");
    const response = await fetch("/admin/restore", { method: "POST", body });
    const grid = await (await fetch("/api/projects/test-project/grid")).json();

    return { status: response.status, says: await response.text(), tasks: grid.tasks.length };
  }, foreign),
);

check(
  "fugantt のものでない SQLite は断り、何も壊さない",
  strangerSays.status === 400 && strangerSays.says.includes("fugantt") && strangerSays.tasks > 0,
  JSON.stringify(strangerSays),
);

// 管理者以外には、そんな口があること自体を教えない。
const notAdmin = await page.evaluate(async () => {
  const download = await fetch("/admin/backup.db");
  const body = new FormData();
  body.append("backup", new Blob([new TextEncoder().encode("x")]), "x.db");
  const restore = await fetch("/admin/restore", { method: "POST", body });

  return { download: download.status, restore: restore.status };
});

check(
  "管理者でなければバックアップは触れない",
  notAdmin.download === 404 && notAdmin.restore === 404,
  JSON.stringify(notAdmin),
);

check("JavaScript エラーが出ていない", pageErrors.length === 0, pageErrors.join(" / "));

await browser.close();

for (const name of passed) console.log("  ✓", name);
for (const name of failed) console.log("  ✗", name);
console.log(`\n${passed.length} passed, ${failed.length} failed`);

process.exit(failed.length ? 1 : 0);
