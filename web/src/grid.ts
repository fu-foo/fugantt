import "./grid.css";

/** One row as the server resolved it. Nothing here is recomputed in the browser. */
interface Task {
  id: string;
  depth: number;
  name: string;
  start: string | null;
  end: string | null;
  actual_start: string | null;
  actual_end: string | null;
  progress: number;
  days: number | null;
  actual_days: number | null;
  start_variance: number | null;
  end_variance: number | null;
  status: string;
  assignee: string;
  note: string;
  waits: { start: string; end: string; reason: string; open: boolean; days: number }[];
  wait_days: number;
  /** Days past the planned end, on work that has not finished. */
  overdue: number;
  /** 予定進捗: the checkpoints this plan named, in date order. */
  targets: { date: string; percent: number; due: boolean; missed: boolean }[];
  /** The last checkpoint that has come round. `null` when the plan named none. */
  expected: number | null;
  /** Behind a checkpoint the plan itself named. Never a guess from the dates. */
  delayed: boolean;
  /** Colours somebody gave this row, `#rrggbb`, or empty. */
  color: string;
  background: string;
  has_children: boolean;
  tags: string[];
  values: Record<string, string>;
}

interface GridData {
  project_id: string;
  revision: number;
  /// The language, decided by the server from the person's own setting, the
  /// installation's, and the browser's.
  language: string;
  today: string;
  range_start: string;
  range_end: string;
  holidays: { date: string; name: string }[];
  leaves: {
    id: string;
    assignee: string;
    start: string;
    end: string;
    note: string;
    kind: string;
  }[];
  assignees: { name: string; color: string; background: string }[];
  statuses: { name: string; color: string; percent: number | null }[];
  theme: {
    bar: string;
    done: string;
  actual: string;
    summary: string;
    late: string;
    saturday: string;
    sunday: string;
    holiday: string;
    leave: string;
    wait: string;
  today: string;
  };
  fields: {
    id: string;
    label: string;
    kind: string;
    options: { value: string; color: string; background: string }[];
  }[];
  hidden_columns: string[];
  column_order: string[];
  /// Columns the chart repeats in a bar's tooltip, by key.
  tooltip_columns: string[];
  /** Saved filter conditions: everybody's first, then this person's. */
  filter_sets: { id: string; name: string; conditions: string; shared: boolean }[];
  column_widths: Record<string, number>;
  frozen_columns: number;
  counting: {
    monday: boolean;
    tuesday: boolean;
    wednesday: boolean;
    thursday: boolean;
    friday: boolean;
    saturday: boolean;
    sunday: boolean;
    holidays: boolean;
    leave: boolean;
  };
  fiscal_year_start: number;
  japanese_era: boolean;
  quarters: boolean;
  eras: { from: string; name: string }[];
  day_width: number;
  can_edit: boolean;
  tasks: Task[];
}

/**
 * One change this tab made, in both directions.
 *
 * `send` is a value the server will accept; `stored` is the value it keeps.
 * They are the same for almost every field — 待ち is the exception, whose
 * written form (`8/17〜8/21 他部署`) is not the form it is kept in.
 *
 * A step with no field is a barrier: adding, deleting and reordering rows are
 * not undoable, and stepping silently past one would undo something older
 * while the row it was about is still missing.
 */
/**
 * Where a row sits among its siblings: the summary row it hangs from, and the
 * one it comes after inside it.
 *
 * Both together are what `place` takes, so a spot noted before a move is the
 * instruction that puts the row back.
 */
interface Spot {
  parent: string | null;
  after: string | null;
}

/**
 * One change, and how to take it back.
 *
 * A cell remembers the value on each side. A row that moved remembers where it
 * stood; a row that was added remembers where it landed, and taking that back
 * means removing it. A delete is still a barrier: putting a row back means
 * putting its subtree back with the ids it had, which is a different piece of
 * work from any of these.
 */
type Step =
  | {
      kind: "cell";
      taskId: string;
      field: string;
      fieldId?: string;
      before: { send: string; stored: string };
      after: { send: string; stored: string };
    }
  | { kind: "move"; taskId: string; from: Spot; to: Spot }
  | { kind: "add"; taskId: string; at: Spot }
  | { kind: "barrier" };

interface Mutation {
  /** The whole plan, from the writes that can move anything anywhere. */
  grid?: GridData;
  /** What one ordinary write changed. See [`Patch`]. */
  patch?: Patch;
  task_id: string | null;
  /** Why a request that succeeded still changed nothing. */
  note?: string;
}

/**
 * The rows one write changed, instead of the plan it changed them in.
 *
 * A value carries as far as the summary rows above it and stops, so that is
 * what comes back: three or four rows on a plan of ten thousand. `total` is the
 * check — if the plan is not the length the server says it is, this browser has
 * missed something and asks for the whole thing rather than drawing numbers it
 * cannot vouch for.
 */
interface Patch {
  revision: number;
  rows: Task[];
  /** Where a new row goes: the id it follows, or absent for the top. */
  after?: string;
  /** A row that changed places, with its subtree following it. */
  moved?: { id: string; after?: string; depth: number };
  /** Rows that are gone, subtree and all. */
  removed?: string[];
  range_start: string;
  range_end: string;
  total: number;
}

/** A column in the left pane: a built-in one, or one the project defined. */
interface ColumnDef {
  key: string;
  label: string;
  kind:
    | "name"
    | "date"
    | "days"
    | "variance"
    | "progress"
    | "status"
    | "text"
    | "number"
    | "select"
    /** Free text with the project's master list offered as candidates. */
    | "suggest";
  /** Set for project-defined columns; the built-ins live on the task itself. */
  fieldId?: string;
  options?: { value: string; color: string; background: string }[];
}

/**
 * A built-in column by name.
 *
 * The bars ask "may this row's dates be edited" by handing over a column, and
 * they used to reach into this list by position. Reordering the columns then
 * quietly made summary bars draggable, because position 1 stopped being a date.
 */
function column(key: string): ColumnDef {
  return BASE_COLUMNS.find((entry) => entry.key === key) ?? BASE_COLUMNS[0]!;
}

const BASE_COLUMNS: ColumnDef[] = [
  { key: "name", label: "タスク", kind: "name" },
  // Late, as a column rather than as red text. A colour cannot be filtered,
  // sorted or exported, and the text colour now belongs to whoever painted the
  // row. A column can be asked a question: show me only these.
  {
    key: "late",
    label: "遅延",
    kind: "select",
    options: [
      { value: "遅延", color: "", background: "" },
      { value: "順調", color: "", background: "" },
    ],
  },
  // Who and what state, before any dates: the two things read at a glance.
  { key: "assignee", label: "担当者", kind: "text" },
  { key: "status", label: "ステータス", kind: "status" },
  // The plan, then what happened, in the same four columns each: when it
  // starts, when it ends, how many days, how far along. Read down one and then
  // the other and the pairs line up.
  { key: "start", label: "予定開始", kind: "date" },
  { key: "end", label: "予定終了", kind: "date" },
  { key: "days", label: "予定日数", kind: "days" },
  // 予定進捗: entered, never derived. A list of "by this date, this much",
  // which is the only thing that can say whether the work is behind.
  { key: "targets", label: "予定進捗", kind: "text" },
  { key: "actual_start", label: "実施開始", kind: "date" },
  { key: "actual_end", label: "実施終了", kind: "date" },
  // Days actually worked. Counted up to today while it is still running.
  { key: "actual_days", label: "実作業日数", kind: "days" },
  { key: "progress", label: "実進捗", kind: "progress" },
  // The subtraction, after both sides it subtracts.
  { key: "start_variance", label: "開始差異", kind: "variance" },
  { key: "end_variance", label: "終了差異", kind: "variance" },
  { key: "waits", label: "待ち", kind: "text" },
  // Last on purpose: free text is the widest column and the least often read,
  // so it is the one that should run off the edge rather than push anything.
  { key: "note", label: "コメント", kind: "text" },
];

/** Columns a summary row takes from its children rather than its own row. */
const ROLLED_UP: readonly string[] = [
  "actual_days",
  "start",
  "end",
  "actual_start",
  "actual_end",
  "days",
  "start_variance",
  "end_variance",
  "progress",
];


/**
 * Columns whose filter is a bound, not a substring, and which way a bare value
 * points.
 *
 * A date column filtered by text is nearly useless — nobody wants the rows
 * whose start date happens to contain "08". What they want is everything from
 * a day onwards, or up to one. A start reads as "at least" and an end as "at
 * most", which is also how the two are asked for out loud. Writing the other
 * word (or `<=`) after the value turns any of them around.
 */
/**
 * A filter's direction, and on progress a question that needs no number.
 *
 * `behind` / `ahead` compare the progress against where today says it should
 * be. "Only the rows that are behind" is what people actually want to see, and
 * there is no number to type for it.
 */
type Bound = "gte" | "lte" | "eq" | "gt" | "lt" | "behind" | "ahead";

const BOUND_LABEL: Record<Bound, string> = {
  gte: "以上",
  lte: "以下",
  eq: "一致",
  gt: "超過",
  lt: "未満",
  behind: "遅れ",
  ahead: "順調",
};

/**
 * What the button itself shows.
 *
 * Spelled out, the words are two characters wide, and in a narrow column that
 * leaves no room for the number they apply to. The signs say the same thing in
 * one character; the words are on the button's tooltip.
 */
const BOUND_MARK: Record<Bound, string> = {
  gte: "≧",
  lte: "≦",
  eq: "＝",
  gt: "＞",
  lt: "＜",
  behind: "遅れ",
  ahead: "順調",
};

/** What one column's button offers, in the order it offers it. */
const BOUND_CHOICES: Record<string, Bound[]> = {
  // Progress is more often asked as "only what is behind" than as a percentage,
  // and there is no number to type for that.
  progress: ["gte", "lte", "eq", "gt", "lt", "behind", "ahead"],
};

const BOUND_DEFAULT: Bound[] = ["gte", "lte", "eq", "gt", "lt"];

const FILTER_BOUND: Record<string, Bound> = {
  progress: "gte",
  start: "gte",
  actual_start: "gte",
  end: "lte",
  actual_end: "lte",
  days: "gte",
  // Days actually worked is a number like any other: the question asked of it
  // is "more than five", not "contains a five".
  actual_days: "gte",
  start_variance: "gte",
  end_variance: "gte",
};

/**
 * The island's wording. The key is the original Japanese, and anything without a
 * translation comes out in Japanese.
 *
 * The same idea as `src/i18n/en.rs` on the server. What is deliberately absent
 * is anything the users chose — status names, their own fields, assignees —
 * because that is data, not wording.
 */
const EN: Record<string, string> = {
  // columns
  "タスク": "Task",
  "予定開始": "Planned start",
  "予定終了": "Planned end",
  "実施開始": "Actual start",
  "実施終了": "Actual end",
  "予定日数": "Planned days",
  "実作業日数": "Actual days",
  "表示": "Show",
  "チャートに出すものを選びます": "Choose what to draw on the chart",
  "実際に動いた日数。終わっていなければ今日まで数えます":
    "Days actually worked; counted up to today while it is still running",
  "開始差異": "Start variance",
  "終了差異": "End variance",
  "予定進捗": "Planned",
  "遅延": "Late",
  "予定進捗に届いていません": "Not up to the checkpoint it promised",
  "予定終了を過ぎて、実施終了が入っていません": "Past its planned end, with no actual end",
  "色を消す": "Clear the colour",
  "背景": "Background",
  "文字": "Text",
  "実進捗": "Progress",
  "進捗": "Progress",
  "ステータス": "Status",
  "担当者": "Assignee",
  "コメント": "Note",
  "待ち": "Waiting",

  // filtering
  "以上": "at least",
  "以下": "at most",
  "一致": "equals",
  "超過": "more than",
  "未満": "less than",
  "遅れ": "behind",
  "順調": "on track",
  "解除": "Clear",
  "検索条件": "Saved filters",
  "絞り込みの条件を名前をつけて置いておく": "Keep a set of filters under a name",
  "いまの条件に名前をつけて保存": "Name these filters to keep them",
  "みんなで使う": "Share with everybody",
  "みんなの": "Everybody's",
  "自分の": "Mine",
  "この条件を消す": "Forget this one",
  "絞り込み": "Filter",
  "20260805・8/5・2026-08-05 のどれでも。左のボタンで向きを変えられます":
    "20260805, 8/5 or 2026-08-05 all work. The button on the left changes the comparison.",
  "左のボタンで「以上」「以下」を切り替えられます":
    "The button on the left switches between at least and at most.",
  "カレンダーから選ぶ": "Pick from a calendar",

  // calendar and units
  "休業日": "Closed",
  "日": "d",
  "営業日": "working days",
  "元": "was",

  // dialogs
  "休み": "Away",
  "出社": "Working",
  "メモ（任意）": "Note (optional)",
  "削除": "Delete",
  "＋ 休暇を追加": "+ Add leave",
  "保存": "Save",
  "キャンセル": "Cancel",
  "担当者の休暇 / 出社": "Leave and working days",
  "担当者の休暇/出社": "Leave and working days",
  "休みの日はその人のタスクの日数にも遅れの判定にも入りません。逆に「出社」は、土日祝でもその日を数えます。":
    "Days away count towards neither the day count nor the delay of that person's tasks. Working days do the opposite: they count even on a weekend or a holiday.",
  "予定は人につくので、ここでの登録はその人が出ている全部のプロジェクトに効きます。":
    "Leave belongs to the person, so what you record here applies to every project they are on.",
  "継続中": "still open",
  "理由（任意）": "Reason (optional)",
  "＋ 期間を追加": "+ Add a period",
  "待ちの期間を登録する": "Record the waiting periods",
  "予定進捗を登録する": "Record what should be done by when",
  "＋ 予定を追加": "+ Add a checkpoint",
  "までに": "by then",
  "その日を過ぎても実進捗が届いていなければ遅れになります。間の日は判定しません。入れなければ、この行は進捗では遅れになりません。":
    "Once that date has passed, the row is behind if the work has not reached that percentage. Nothing is judged in between, and a row with no checkpoints is never behind on progress.",
  "この日までに届いていません": "Not there by this date",
  "達成": "Met",
  "これから": "Still to come",
  "終わりを空にすると「まだ待っている」になり、今日まで数え続けます。待ちの日数は日数からも遅れの判定からも外れます。":
    "Leave the end empty for work that is still waiting; it counts up to today. Waiting days are excluded from the day count and from the delay.",
  "（継続中）": "(still waiting)",
  "予定の期間の外なので日数には効きません": "Outside the planned dates, so it changes nothing",
  "8/17〜8/21 他部署（終わり省略で継続中）":
    "8/17-8/21 another team (omit the end while it is still waiting)",

  // the grid
  "（無題）": "(untitled)",
  "無題のタスク": "Untitled task",
  "が更新しました": "made a change",
  "保存できませんでした。接続を確認してください。": "Could not save. Check the connection.",
  "取り消せる操作がありません。": "Nothing to undo.",
  "やり直せる操作がありません。": "Nothing to redo.",
  "その行はもうありません。": "That row is gone.",
  "その行は誰かが動かしました。": "Somebody else moved that row.",
  "その行には子タスクがあるので、取り消しでは消しません。": "That row has child tasks, so undo will not remove it.",
  "書き込みのある行は、取り消しでは消しません。": "That row has something in it, so undo will not remove it.",
  "行の削除は取り消せません。もう一度押すと、その前の変更を取り消します。": "Deleting a row cannot be undone. Press again to undo the change before it.",
  "閉じる": "Close",
  "タスクがありません。": "No tasks yet.",
  "最初のタスクを追加": "Add the first task",
  "閲覧のみ": "Read only",
  "行を追加": "Add a row",
  "行を削除": "Delete the row",
  "誰がいつ休み、いつ出るか。日数の数え方に効きます":
    "Who is away and who is in. It changes how days are counted.",
  "土日・祝日を除いた営業日で数えています": "Counted in working days, weekends and holidays excluded",
  "条件に合う行がありません。": "Nothing matches.",
  "集計行の日付と進捗は子タスクから決まります。":
    "A summary row's dates and progress come from its children.",
  "子タスクのずれを足したものです（この行の日付の差ではありません）":
    "The sum of the children's slippage, not the difference between this row's own dates",
  "ドラッグで移動": "Drag to move",
  "展開する": "Expand",
  "折りたたむ": "Collapse",
  "セルの入力": "Cell editor",
  "ドラッグで進捗を変える": "Drag to change the progress",
  "ドラッグで幅を変える": "Drag to resize",
  "子タスクにする": "Make it a child",
  "階層を戻す": "Move it back out",
  "上へ移動": "Move up",
  "下へ移動": "Move down",
  "下に行を追加": "Add a row below",
  "スケジュールを読み込めませんでした。再読み込みしてください。":
    "Could not load the schedule. Please reload.",
};

/** Which language to draw in. The server says so on every load. */
let LANG: "ja" | "en" = "ja";

function t(ja: string): string {
  return LANG === "en" ? (EN[ja] ?? ja) : ja;
}

const WEEKDAYS_EN = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
const WEEKDAYS = ["日", "月", "火", "水", "木", "金", "土"];

/** Weekdays are one character in Japanese and three in English, so they get
    their own table rather than a dictionary entry. */
function weekday(at: number): string {
  return (LANG === "en" ? WEEKDAYS_EN[at] : WEEKDAYS[at]) ?? "";
}

const DAY_MS = 86_400_000;
const PAGE_ROWS = 10;
/** How close to an end counts as grabbing it rather than the whole bar. */
const GRIP_WIDTH = 7;

/**
 * Parses `YYYY-MM-DD` at UTC midnight.
 *
 * The chart only ever measures whole days, and local parsing would shift a
 * date across a boundary for anyone east or west of the server.
 */
function parseDate(text: string): number {
  const [year, month, day] = text.split("-").map(Number);
  return Date.UTC(year ?? 1970, (month ?? 1) - 1, day ?? 1);
}

function dayIndex(date: string, origin: number): number {
  return Math.round((parseDate(date) - origin) / DAY_MS);
}

/** The `YYYY-MM-DD` that sits `offset` days after the chart's first day. */
function shiftDate(origin: number, offset: number): string {
  return new Date(origin + offset * DAY_MS).toISOString().slice(0, 10);
}

function element<K extends keyof HTMLElementTagNameMap>(
  tag: K,
  className?: string,
  text?: string,
): HTMLElementTagNameMap[K] {
  const node = document.createElement(tag);
  if (className) node.className = className;
  if (text !== undefined) node.textContent = text;
  return node;
}

/**
 * Folds full-width digits and separators onto ASCII.
 *
 * A Japanese keyboard left in kana mode types "２０２６－０９－０１", which is
 * the same date entered the same way. Doing this here as well as on the server
 * keeps the optimistic redraw honest.
 */
/**
 * Reads `100以上`, `50以下`, `>=3`, `<=2026-08-31` — or a bare value on a column
 * that has a direction of its own.
 *
 * Returns null when the box holds plain text, which is the signal to fall back
 * to matching by substring.
 */
function parseBound(
  text: string,
  fallback?: Bound,
): { at: Bound; limit: string } | null {

  // Full-width digits and signs arrive whenever the IME is on, which is most
  // of the time here; a filter that ignored them would look broken.
  const value = normalizeWidth(text).trim();
  if (!value) return null;

  const SIGNS: Record<string, Bound> = {
    ">=": "gte", "=>": "gte", "≧": "gte", "≥": "gte",
    "<=": "lte", "=<": "lte", "≦": "lte", "≤": "lte",
    ">": "gt", "＞": "gt",
    "<": "lt", "＜": "lt",
    "=": "eq", "＝": "eq",
  };

  const written = /^(>=|<=|=>|=<|≧|≥|≦|≤|＞|＜|＝|>|<|=)\s*(.*)$/.exec(value);
  if (written) {
    return { at: SIGNS[written[1]!] ?? "eq", limit: written[2]!.trim() };
  }

  // The spoken forms. 「超」 is tested after 「以上」, which contains no such
  // character, so the longer word wins where both could match.
  const WORDS: [RegExp, Bound][] = [
    [/(以上|以降|いじょう|いこう)$/, "gte"],
    [/(以下|以前|まで|いか|いぜん)$/, "lte"],
    [/(超過|超|より後|より大きい)$/, "gt"],
    [/(未満|より前|より小さい)$/, "lt"],
    [/(と同じ|一致|ちょうど)$/, "eq"],
  ];

  for (const [pattern, at] of WORDS) {
    if (pattern.test(value)) return { at, limit: value.replace(pattern, "").trim() };
  }

  return fallback ? { at: fallback, limit: value } : null;
}

/** One comparison, whichever way the column is asking. */
function compare<T extends number | string>(at: Bound, left: T, right: T): boolean {
  switch (at) {
    case "gte":
      return left >= right;
    case "lte":
      return left <= right;
    case "gt":
      return left > right;
    case "lt":
      return left < right;
    default:
      // Behind and on track never get here: they are answered before a value
      // is read at all.
      return left === right;
  }
}

/**
 * A date however somebody typed it, or null when it is not one.
 *
 * The same readings the server takes: `20260805`, `0805`, `8/5`, `2026年8月5日`.
 * A half-written date like `2026-08` comes back null on purpose — the filter
 * falls back to comparing it as a prefix, which is what half a date means.
 */
function flexibleDate(text: string): string | null {
  const value = normalizeWidth(text)
    .trim()
    .replace(/[/.年月]/g, "-")
    .replace(/日/g, "")
    .replace(/-+$/, "");

  const year = new Date().getUTCFullYear();
  let iso: string | null = null;

  if (/^\d+$/.test(value)) {
    if (value.length === 8) iso = `${value.slice(0, 4)}-${value.slice(4, 6)}-${value.slice(6)}`;
    else if (value.length === 4) iso = `${year}-${value.slice(0, 2)}-${value.slice(2)}`;
  } else {
    const parts = value.split("-").filter(Boolean);
    const pad = (part: string, width: number) => part.padStart(width, "0");

    if (parts.length === 2) iso = `${year}-${pad(parts[0]!, 2)}-${pad(parts[1]!, 2)}`;
    else if (parts.length === 3) iso = `${pad(parts[0]!, 4)}-${pad(parts[1]!, 2)}-${pad(parts[2]!, 2)}`;
  }

  if (!iso || !/^\d{4}-\d{2}-\d{2}$/.test(iso)) return null;

  // `2026-13-99` parses into next year somewhere; only a date that survives the
  // round trip is a date.
  const parsed = new Date(`${iso}T00:00:00Z`);
  return Number.isNaN(parsed.getTime()) || parsed.toISOString().slice(0, 10) !== iso ? null : iso;
}

const MONTHS_EN = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
// Written out where there is room for it. The band over the chart is as wide as
// the month is long, and 2026年8月 is not abbreviated either.
const MONTH_NAMES_EN = [
  "January", "February", "March", "April", "May", "June",
  "July", "August", "September", "October", "November", "December",
];

/**
 * `2026-08-17` as `8/17`, or `Aug 17`: the year is nearly always the one on
 * screen.
 *
 * English never gets `8/17`. Written that way it is the seventeenth of August
 * in Boston and nothing at all in Berlin, where the same shape means the eighth
 * of a seventeenth month. A date nobody can be sure of is worse than a longer
 * one.
 */
function short(iso: string): string {
  const [, month, day] = iso.split("-");
  if (!month || !day) return iso;

  const name = MONTHS_EN[Number(month) - 1];
  return LANG === "en" && name ? `${name} ${Number(day)}` : `${Number(month)}/${Number(day)}`;
}

/** A day as the reader writes it: `2026/08/17`, or the stored `2026-08-17`. */
function fullDate(iso: string): string {
  // English keeps the stored form. It is the one shape that reads the same in
  // every country, and the alternatives — 8/17 and 17/8 — are each other's
  // wrong answer.
  return LANG === "en" ? iso : iso.replace(/-/g, "/");
}

function normalizeWidth(text: string): string {
  return text
    .replace(/[０-９ａ-ｚＡ-Ｚ]/g, (c) => String.fromCharCode(c.charCodeAt(0) - 0xfee0))
    .replace(/[－ー−‐]/g, "-")
    .replace(/／/g, "/")
    .replace(/％/g, "%")
    .replace(/　/g, " ");
}

function clamp(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, value));
}

/**
 * This browser's identity for the session.
 *
 * The server publishes a change before the response to it arrives, so a client
 * cannot tell its own echo from someone else's edit by revision alone. It sends
 * this on every write and ignores the events that carry it back.
 */
const CLIENT_ID = randomId();

/**
 * A per-session identifier.
 *
 * Not `crypto.randomUUID()`: that one only exists in a secure context, so on a
 * plain-HTTP LAN it is undefined and the whole island dies on the first line.
 * This value only has to be different from the other tabs', never unguessable.
 */
function randomId(): string {
  const bytes = new Uint8Array(16);

  if (globalThis.crypto?.getRandomValues) {
    globalThis.crypto.getRandomValues(bytes);
  } else {
    for (let i = 0; i < bytes.length; i++) bytes[i] = Math.floor(Math.random() * 256);
  }

  return [...bytes].map((b) => b.toString(16).padStart(2, "0")).join("");
}

/**
 * Modifier names as this machine spells them.
 *
 * The handlers already accept either key — `ctrlKey || metaKey` — so only the
 * labels need to know which platform they are on.
 */
const ON_MAC = /Mac|iPhone|iPad/.test(navigator.userAgent);
const MOD = ON_MAC ? "⌘" : "Ctrl+";
const ALT = ON_MAC ? "⌥" : "Alt+";

const collapsedKey = (projectId: string) => `fugantt:collapsed:${projectId}`;

/** Folded rows survive a reload. Private browsing may refuse storage; that is fine. */
function loadCollapsed(projectId: string): Set<string> {
  try {
    const stored = window.localStorage.getItem(collapsedKey(projectId));
    return new Set(stored ? (JSON.parse(stored) as string[]) : []);
  } catch {
    return new Set();
  }
}

function saveCollapsed(projectId: string, collapsed: Set<string>): void {
  try {
    window.localStorage.setItem(collapsedKey(projectId), JSON.stringify([...collapsed]));
  } catch {
    // Nothing to do: the fold state is a convenience, not data worth an error.
  }
}

/** How wide each kind of column wants to be, as a grid track. */
const TRACKS: Record<ColumnDef["kind"], string> = {
  name: "minmax(11rem, 1.6fr)",
  date: "6.5rem",
  // These three carry a direction button in the filter row as well as their
  // own short values, and a 3.4rem column has no room for both.
  days: "5.2rem",
  variance: "4.8rem",
  progress: "4.6rem",
  status: "5.5rem",
  text: "minmax(5rem, 0.8fr)",
  suggest: "minmax(5rem, 0.8fr)",
  number: "4.5rem",
  select: "minmax(5rem, 0.6fr)",
};

/**
 * The colours a row can be painted, chosen to stay readable.
 *
 * Pale enough that black text still reads on them, and far enough apart that
 * two rows in different colours are two different marks rather than a gradient.
 */
const BACKGROUNDS = [
  "#fef3c7",
  "#dcfce7",
  "#dbeafe",
  "#fce7f3",
  "#ede9fe",
  "#ffe4e6",
  "#e2e8f0",
];

/** Dark enough to read on white and on every one of the backgrounds above. */
const TEXT_COLOURS = ["#0f172a", "#b91c1c", "#a16207", "#15803d", "#1d4ed8", "#7e22ce"];

const PANE_KEY = "fugantt:pane-width";
const SHOWS_KEY = "fugantt:chart-shows";

/** What gets drawn over the chart. More of it says more, and reads worse. */
type Shows = { start: boolean; end: boolean; worked: boolean; targets: boolean };

function loadShows(): Shows {
  const stored = window.localStorage.getItem(SHOWS_KEY);
  const shows: Shows = { start: true, end: true, worked: true, targets: true };

  if (!stored) return shows;

  try {
    return { ...shows, ...(JSON.parse(stored) as Partial<Shows>) };
  } catch {
    // Broken settings fall back to the default: a screen that will not draw
    // because of a preference has its priorities backwards.
    return shows;
  }
}

function loadPaneWidth(): number {
  const stored = Number(window.localStorage.getItem(PANE_KEY));
  return Number.isFinite(stored) && stored > 0 ? clamp(stored, 160, 1200) : 0;
}

/**
 * Keeps the rows that match, and the ancestors that hold them.
 *
 * Dropping a parent because its own text does not match would orphan the
 * children that do, and the indentation would then be a lie.
 */
function keepMatches(tasks: Task[], hit: (task: Task) => boolean): Task[] {
  const matches = tasks.map(hit);
  const keep = new Array<boolean>(tasks.length).fill(false);

  for (const [index, task] of tasks.entries()) {
    if (!matches[index]) continue;

    keep[index] = true;

    // Everything above this row that is shallower is an ancestor of it.
    let depth = task.depth;
    for (let i = index - 1; i >= 0 && depth > 0; i--) {
      const candidate = tasks[i];
      if (candidate && candidate.depth < depth) {
        keep[i] = true;
        depth = candidate.depth;
      }
    }
  }

  return tasks.filter((_, index) => keep[index]);
}

function rowText(task: Task): string {
  return [
    task.name,
    task.status,
    task.assignee,
    task.note,
    ...task.tags,
    ...Object.values(task.values),
  ]
    .join(" ")
    .toLowerCase();
}

/** The grid: selection, keyboard editing, and the chart beside it. */
class Grid {
  /** Width of one day column. Comes from the project's settings. */
  private get dayWidth(): number {
    return this.data.day_width || 26;
  }

  private row = 0;
  private column = 0;
  private editing = false;
  /** The character that opened the editor, so typing does not lose the keystroke. */
  private seed: string | null = null;
  private error: string | null = null;
  /** True while an IME conversion is open, so nothing may re-render under it. */
  private composing = false;
  /** True while the open editor is being carried to its rebuilt cell. */
  private moving = false;
  /** A passing line about someone else's change, not a problem to fix. */
  private notice: string | null = null;
  private noticeTimer = 0;
  private busy = false;
  /**
   * Where the chart is scrolled to sideways, or null before it has been drawn.
   *
   * Null rather than zero: a plan that starts in April is *at* zero when
   * somebody is reading April, and taking that for "never scrolled" sent them
   * back to today on every edit.
   */
  private scrollLeft: number | null = null;

  /**
   * The summary rows whose subtrees are folded away.
   *
   * Folding is how one person is reading the plan right now, not something
   * about the plan, so it stays in this browser rather than on the server
   * where it would fold everyone else's view too.
   */
  private readonly collapsed: Set<string>;

  /** `data.tasks` minus everything inside a folded row. All indices are into this. */
  private visible: Task[] = [];
  /** One filter per column, ANDed together. Empty entries are ignored. */
  private filters = new Map<string, string>();
  /**
   * The direction each bounded column is asking in, where it is not the
   * default. Chosen by clicking, rather than by remembering what to type.
   */
  private bounds = new Map<string, Bound>();
  /** What to draw over the chart: this person's view of it, not the project's
      setting. */
  private shows: Shows = loadShows();

  /**
   * What this tab has changed, newest last, so it can be put back.
   *
   * Only what this tab did. Somebody else's edit is not this person's to take
   * back, and a stack that outlived the tab would offer to undo work from a
   * morning nobody remembers. Reloading empties it, which is the honest
   * boundary: undo goes back as far as you can still see.
   */
  /**
   * The rows that are actually in the document, as an inclusive range.
   *
   * Everything above and below is a spacer of the right height, so the
   * scrollbar and every row's position are unchanged — the browser is simply
   * not asked to keep eighty thousand nodes it cannot show. Measured: at two
   * thousand rows, style and layout over that tree cost 80ms a keystroke no
   * matter how little work the island itself did.
   */
  private first = 0;
  private last = -1;
  /** Rows kept beyond each edge, so a small scroll redraws nothing. */
  private static readonly OVERSCAN = 8;
  /** Measured from the page: a row's height, and where the rows begin. */
  private rowPixels = 32;
  private rowsTop = 0;
  private scrollTop = 0;

  private done: Step[] = [];
  private undone: Step[] = [];
  /** Set while undoing, so putting a value back is not itself recorded. */
  private replaying = false;
  /** The column whose filter box the caret is in, across a re-render. */
  private filterFocus: { key: string; caret: number | null } | null = null;
  /** How much of the width the left pane takes, dragged by the splitter. */
  private paneWidth = loadPaneWidth();
  /** Whether the table pane still needs holding back to half the window. */
  private capPaneWidth = false;
  /** The last cell pressed, for spotting a double-click ourselves. */
  private lastPress: { row: number; column: number; at: number } | null = null;

  constructor(
    private readonly root: HTMLElement,
    private readonly projectId: string,
    private data: GridData,
  ) {
    this.collapsed = loadCollapsed(projectId);
    LANG = data.language === "en" ? "en" : "ja";
    this.computeVisible();
    this.root.addEventListener("keydown", (event) => this.onKeyDown(event));
    // Every column moves when the window does, and a column pinned to where it
    // used to be sits on top of its neighbour.
    window.addEventListener("resize", () => this.pinColumns());

    this.listen();
    this.render();
  }

  /**
   * Follows other people's changes.
   *
   * The event carries only a revision, so a client that hears one refetches
   * rather than trying to apply someone else's edit. Our own writes come back
   * too, but by then we already hold that revision, so they fall through.
   */
  private listen(): void {
    const source = new EventSource(
      `/api/projects/${encodeURIComponent(this.projectId)}/live`,
    );

    source.addEventListener("change", (event) => {
      const change = JSON.parse((event as MessageEvent<string>).data) as {
        revision: number;
        actor: string;
        client: string | null;
        task_id: string | null;
        kind: string;
      };

      // Our own write, arriving before its own response.
      if (change.client === CLIENT_ID) return;
      if (change.revision <= this.data.revision) return;

      // Somebody else's ordinary edit: ask about the row they touched. Reading
      // the whole plan back was the same four megabytes its writer no longer
      // sends — two people on a long plan meant every keystroke of theirs cost
      // this browser a full read. Anything that moves rows about still does.
      if (change.kind === "cell" && change.task_id) {
        void this.follow(change.task_id, change.actor);
        return;
      }

      void this.refresh(change.actor);
    });
  }

  /** Reloads the grid after someone else changed it, keeping the cursor put. */
  /** Takes in one row somebody else changed, without reading the plan back. */
  private async follow(taskId: string, actor: string): Promise<void> {
    // Not mid-edit: the same rule the whole-plan refresh follows. What is being
    // typed here has not been sent yet, and redrawing over it throws it away.
    if (this.editing || this.composing) return;

    try {
      const response = await fetch(
        `/api/projects/${encodeURIComponent(this.projectId)}/tasks/${encodeURIComponent(taskId)}/patch`,
      );

      // A row that is not there any more, or a plan this browser has fallen
      // behind on: read it properly rather than guess.
      if (!response.ok) {
        void this.refresh(actor);
        return;
      }

      const drew = this.applyPatch((await response.json()) as Patch);
      if (drew === null) {
        void this.refresh(actor);
        return;
      }

      this.showNotice(`${actor} ${t("が更新しました")}`);
      this.repaintRows(drew);
    } catch {
      // A failed follow leaves the stale row in place, which is better than an
      // error banner for something the user did not do.
    }
  }

  private async refresh(actor: string): Promise<void> {
    // Refetching mid-edit would throw away what is being typed — including a
    // conversion that has not been committed yet.
    if (this.editing || this.composing) return;

    const here = this.selected?.id;

    try {
      const response = await fetch(
        `/api/projects/${encodeURIComponent(this.projectId)}/grid`,
      );
      if (!response.ok) return;

      // Somebody else's change is still just some rows moving. Taken before
      // the new table lands, so only those rows are drawn again.
      const shape = this.shape();
      const rows = this.tasks.map((task) => JSON.stringify(task));

      this.setData((await response.json()) as GridData);

      const moved = here ? this.tasks.findIndex((task) => task.id === here) : -1;
      this.select(moved >= 0 ? moved : this.row, this.column);

      this.notice = `${actor} が更新しました`;
      window.clearTimeout(this.noticeTimer);
      this.noticeTimer = window.setTimeout(() => {
        this.notice = null;
        this.paintNotice();
      }, 4000);

      this.repaintChanged(shape, rows);
    } catch {
      // A failed refresh leaves the stale grid in place, which is better than
      // an error banner for something the user did not do.
    }
  }

  // --- data ----------------------------------------------------------------

  private get tasks(): Task[] {
    return this.visible;
  }

  private setData(grid: GridData): void {
    this.data = grid;
    // The server decides the language every time. The island draws; it does
    // not judge.
    LANG = grid.language === "en" ? "en" : "ja";
    this.computeVisible();
  }

  /**
   * Drops every row that sits under a folded one.
   *
   * The server already hands the tree back flattened depth-first, so a folded
   * row's whole subtree is exactly the run of deeper rows that follows it.
   */
  private computeVisible(): void {
    const visible: Task[] = [];
    let foldedAt = -1;

    for (const task of this.data.tasks) {
      if (foldedAt >= 0) {
        if (task.depth > foldedAt) continue;
        foldedAt = -1;
      }

      visible.push(task);

      if (task.has_children && this.collapsed.has(task.id)) foldedAt = task.depth;
    }

    if (!this.filtering) {
      this.visible = visible;
      return;
    }

    // Every filled-in box must match: conditions narrow, they do not widen.
    // A direction like "behind" is a condition in itself, empty box or not.
    const keys = new Set([...this.filters.keys(), ...this.stateColumns.map((c) => c.key)]);

    const conditions = [...keys]
      .map((key) => ({
        column: this.columns.find((column) => column.key === key),
        needle: this.filters.get(key) ?? "",
      }))
      .filter((condition) => condition.column !== undefined);

    this.visible = keepMatches(visible, (task) =>
      conditions.every(({ column, needle }) => this.matches(task, column!, needle)),
    );
  }

  /** Wires the header's filter box, which lives outside the island's markup. */
  private updateFilterCount(): void {
    const label = document.getElementById("fugantt-filter-count");
    if (!label) return;

    label.textContent = "";

    if (this.filtering) {
      label.append(
        element("span", "fg-filter-count", `絞り込み中 ${this.tasks.length} / ${this.data.tasks.length} 行`),
      );

      // Filtering hides rows, and a hidden row is easy to forget about; the way
      // out belongs next to the count that says rows are missing.
      const clear = element("button", "fg-filter-clear", t("解除"));
      clear.type = "button";
      clear.addEventListener("click", () => this.clearFilters());
      label.append(clear);
    }

    // Always, unlike the count: the whole point of a saved view is to reach for
    // it from a table that is not filtered yet.
    //
    // The same few questions get asked every week — "遅れているものだけ",
    // "自分の担当だけ" — and were being typed again every time.
    const saved = element("button", "fg-filter-sets", t("検索条件"));
    saved.type = "button";
    saved.title = t("絞り込みの条件を名前をつけて置いておく");
    if (this.data.filter_sets.length > 0) saved.classList.add("is-on");
    saved.addEventListener("click", () => this.openFilterSets(saved));

    label.append(saved);
  }

  /**
   * The saved conditions, and the way to add one.
   *
   * Everybody's above, this person's below, because the first question about a
   * saved view is whose it is.
   */
  private openFilterSets(anchor: HTMLElement): void {
    const box = anchor.getBoundingClientRect();
    const menu = element("div", "fg-menu fg-sets-menu");
    menu.style.left = `${box.left}px`;
    menu.style.top = `${box.bottom + 2}px`;

    const close = (event?: Event) => {
      if (event && menu.contains(event.target as Node)) return;

      menu.remove();
      document.removeEventListener("mousedown", close);
      document.removeEventListener("keydown", onEscape);
    };

    const onEscape = (event: KeyboardEvent) => {
      if (event.key === "Escape") close();
    };

    const section = (label: string, shared: boolean) => {
      const sets = this.data.filter_sets.filter((set) => set.shared === shared);
      if (sets.length === 0) return;

      menu.append(element("div", "fg-menu-label", label));

      for (const set of sets) {
        const row = element("div", "fg-sets-row");

        const use = element("button", "fg-menu-item", set.name) as HTMLButtonElement;
        use.type = "button";
        use.addEventListener("mousedown", (event) => event.preventDefault());
        use.addEventListener("click", () => {
          close();
          this.applyConditions(set.conditions);
        });

        const drop = element("button", "fg-sets-remove", "×") as HTMLButtonElement;
        drop.type = "button";
        drop.title = t("この条件を消す");
        drop.addEventListener("mousedown", (event) => event.preventDefault());
        drop.addEventListener("click", async () => {
          close();
          await this.send(
            `/api/projects/${encodeURIComponent(this.projectId)}/filters/remove`,
            { method: "POST", body: { id: set.id } },
          );
        });

        row.append(use, drop);
        menu.append(row);
      }
    };

    section(t("みんなの"), true);
    section(t("自分の"), false);

    if (this.data.filter_sets.length > 0) menu.append(element("div", "fg-menu-rule"));

    // Saving is part of the same menu: the conditions being saved are the ones
    // on the screen, and a second place to go would be a second thing to find.
    const name = element("input", "fg-sets-name") as HTMLInputElement;
    name.type = "text";
    name.placeholder = t("いまの条件に名前をつけて保存");

    const mine = element("label", "fg-sets-scope");
    const tick = element("input") as HTMLInputElement;
    tick.type = "checkbox";
    mine.append(tick, element("span", undefined, t("みんなで使う")));

    const save = element("button", "fg-menu-item fg-sets-save", t("保存")) as HTMLButtonElement;
    save.type = "button";
    save.addEventListener("mousedown", (event) => event.preventDefault());
    save.addEventListener("click", async () => {
      if (!name.value.trim()) {
        name.focus();
        return;
      }

      close();
      await this.send(`/api/projects/${encodeURIComponent(this.projectId)}/filters`, {
        method: "POST",
        body: {
          name: name.value.trim(),
          shared: tick.checked,
          conditions: this.conditions(),
        },
      });
    });

    name.addEventListener("keydown", (event) => {
      event.stopPropagation();
      if (event.key === "Enter") save.click();
    });

    menu.append(name, mine, save);
    document.body.append(menu);

    const placed = menu.getBoundingClientRect();
    if (placed.right > window.innerWidth) {
      menu.style.left = `${Math.max(8, window.innerWidth - placed.width - 8)}px`;
    }

    name.focus();

    setTimeout(() => {
      document.addEventListener("mousedown", close);
      document.addEventListener("keydown", onEscape);
    });
  }

  /** The conditions on screen, as one string to keep. */
  private conditions(): string {
    return JSON.stringify({
      filters: Object.fromEntries(this.filters),
      bounds: Object.fromEntries(this.bounds),
    });
  }

  /** Puts a saved set of conditions back on the screen. */
  private applyConditions(stored: string): void {
    let saved: { filters?: Record<string, string>; bounds?: Record<string, string> };

    try {
      saved = JSON.parse(stored) as typeof saved;
    } catch {
      // Stored by an older build, or by hand. Nothing to apply, and nothing
      // worth an error banner over.
      return;
    }

    this.filters = new Map(Object.entries(saved.filters ?? {}));
    this.bounds = new Map(
      Object.entries(saved.bounds ?? {}).map(([key, value]) => [key, value as Bound]),
    );

    this.filterFocus = null;
    this.computeVisible();
    this.select(this.row, this.column);
    this.render();
  }

  /** Empties every filter box. */
  private clearFilters(): void {
    this.filters.clear();
    // The directions go back to the ones the columns were born with: a "behind"
    // left behind would keep filtering after every box had been emptied.
    this.bounds.clear();
    this.filterFocus = null;
    this.computeVisible();
    this.render();
    this.updateFilterCount();
  }

  private get filtering(): boolean {
    return (
      [...this.filters.values()].some((value) => value !== "") || this.stateColumns.length > 0
    );
  }

  /** Which way a column's filter points, once the user has had a say. */
  private boundFor(column: ColumnDef): Bound | undefined {
    const chosen = this.bounds.get(column.key) ?? FILTER_BOUND[column.key];
    if (chosen) return chosen;

    // A project's own field compares the same way when it holds a date or a
    // number. Only a free-text field is about containing characters; filtering a
    // column of dates for "contains 08" means nothing.
    return column.fieldId && (column.kind === "date" || column.kind === "number")
      ? "gte"
      : undefined;
  }

  /** Bounds that are a condition on their own, with nothing to type. */
  private get stateColumns(): ColumnDef[] {
    return this.columns.filter((column) => {
      const bound = this.boundFor(column);
      return bound === "behind" || bound === "ahead";
    });
  }

  private setBound(column: ColumnDef, at: Bound): void {
    this.bounds.set(column.key, at);
    // The box is rebuilt with the row; without this, changing the direction
    // half way through typing a date throws the caret out of it.
    this.filterFocus = { key: column.key, caret: null };
    this.computeVisible();
    this.render();
    this.updateFilterCount();
  }

  /**
   * The list of comparisons a column can be asked in.
   *
   * Five of them (seven on progress) is past what a button can cycle through:
   * from the first to the last would be four clicks, and nothing on screen would
   * say what is coming next.
   */
  private openBoundMenu(column: ColumnDef, chip: HTMLElement): void {
    const choices = BOUND_CHOICES[column.key] ?? BOUND_DEFAULT;
    const current = this.boundFor(column);
    const anchor = chip.getBoundingClientRect();

    const menu = element("div", "fg-menu fg-bound-menu");
    menu.style.left = `${anchor.left}px`;
    menu.style.top = `${anchor.bottom + 2}px`;

    const close = (event?: Event) => {
      if (event && menu.contains(event.target as Node)) return;

      menu.remove();
      document.removeEventListener("mousedown", close);
      document.removeEventListener("keydown", onEscape);
    };

    const onEscape = (event: KeyboardEvent) => {
      if (event.key === "Escape") close();
    };

    for (const at of choices) {
      const button = element("button", "fg-menu-item");
      button.type = "button";
      button.dataset["bound"] = at;
      if (at === current) button.classList.add("is-current");
      button.append(
        element("span", undefined, BOUND_LABEL[at]),
        element("kbd", undefined, BOUND_MARK[at]),
      );
      button.addEventListener("mousedown", (event) => event.preventDefault());
      button.addEventListener("click", () => {
        close();
        this.setBound(column, at);
      });
      menu.append(button);
    }

    this.root.append(menu);

    const box = menu.getBoundingClientRect();
    if (box.right > window.innerWidth) menu.style.left = `${window.innerWidth - box.width - 8}px`;
    if (box.bottom > window.innerHeight) menu.style.top = `${anchor.top - box.height - 2}px`;

    setTimeout(() => {
      document.addEventListener("mousedown", close);
      document.addEventListener("keydown", onEscape);
    });
  }

  /**
   * The list of values a column can hold, with ticks against the chosen ones.
   *
   * Parked on `document.body` like the other menus: the island replaces its own
   * children whenever a filter changes, and a menu inside it would be thrown
   * away by the first tick — leaving no way to make the second.
   */
  private openChoices(column: ColumnDef, choices: string[], anchor: HTMLElement): void {
    const box = anchor.getBoundingClientRect();
    const menu = element("div", "fg-menu fg-pick-menu");
    menu.style.left = `${box.left}px`;
    menu.style.top = `${box.bottom + 2}px`;

    const chosen = new Set((this.filters.get(column.key) ?? "").split("\n").filter(Boolean));

    const close = (event?: Event) => {
      if (event && menu.contains(event.target as Node)) return;

      menu.remove();
      document.removeEventListener("mousedown", close);
      document.removeEventListener("keydown", onEscape);
    };

    const onEscape = (event: KeyboardEvent) => {
      if (event.key === "Escape") close();
    };

    const all = element("button", "fg-menu-item", t("すべて")) as HTMLButtonElement;
    all.type = "button";
    all.addEventListener("mousedown", (event) => event.preventDefault());
    all.addEventListener("click", () => {
      chosen.clear();
      close();
      this.setChoices(column.key, []);
    });
    menu.append(all, element("div", "fg-menu-rule"));

    for (const choice of choices) {
      const item = element("label", "fg-menu-item fg-pick-item");
      const tick = element("input") as HTMLInputElement;
      tick.type = "checkbox";
      tick.checked = chosen.has(choice);
      tick.addEventListener("change", () => {
        if (tick.checked) chosen.add(choice);
        else chosen.delete(choice);

        this.setChoices(column.key, [...chosen]);
      });

      item.append(tick, element("span", undefined, choice));
      menu.append(item);
    }

    document.body.append(menu);

    const placed = menu.getBoundingClientRect();
    if (placed.right > window.innerWidth) {
      menu.style.left = `${Math.max(8, window.innerWidth - placed.width - 8)}px`;
    }

    setTimeout(() => {
      document.addEventListener("mousedown", close);
      document.addEventListener("keydown", onEscape);
    });
  }

  /** Sets which values of a list column are being asked for. */
  private setChoices(key: string, values: string[]): void {
    if (values.length > 0) this.filters.set(key, values.join("\n"));
    else this.filters.delete(key);

    this.filterFocus = null;
    this.computeVisible();
    this.select(this.row, this.column);
    this.render();
  }

  private setFilter(key: string, text: string, caret: number | null): void {
    const value = text.trim().toLowerCase();

    if (value) this.filters.set(key, value);
    else this.filters.delete(key);

    this.filterFocus = { key, caret };
    this.computeVisible();
    this.select(this.row, this.column);
    this.render();
  }

  /**
   * A filter box per column, so conditions combine.
   *
   * One box over the whole row can only ever answer "does this row mention X";
   * asking about two columns at once — a person and a status — needs one box
   * each.
   */
  private renderFilterRow(tracks: string): HTMLElement {
    const row = element("div", "fg-row fg-filters");
    row.style.gridTemplateColumns = tracks;

    this.columns.forEach((column, index) => {
      // Same per-column class as everywhere else, so the pinned name column
      // pins here too and the boxes stay over their columns. The index comes
      // from the loop: `columns` builds a fresh array on every read, so
      // `indexOf` finds nothing for a column it rebuilt and pins the wrong one.
      const cell = element("div", `fg-cell fg-cell-${column.key}`);
      if (index < this.data.frozen_columns) cell.classList.add("is-frozen");

      const current = this.filters.get(column.key) ?? "";

      const choices = this.choicesFor(column);

      if (choices) {
        // A list of choices asks "which of these", and the answer is often more
        // than one: 待ち and 保留 are the same question. A `<select>` can only
        // hold one, so this is a button that opens the list with ticks in it.
        const chosen = current ? current.split("\n") : [];

        const box = element("button", "fg-filter fg-filter-pick");
        box.type = "button";
        box.dataset["column"] = column.key;
        if (chosen.length > 0) box.classList.add("is-on");

        box.textContent =
          chosen.length === 0 ? "" : chosen.length === 1 ? chosen[0]! : `${chosen.length}件`;
        box.title = chosen.length > 0 ? chosen.join("、") : t("選んで絞り込む");

        box.addEventListener("click", () => this.openChoices(column, choices, box));
        cell.append(box);
      } else {
        const bound = this.boundFor(column);

        // A direction is a choice, not a property of the column: a planned
        // start is asked about both ways depending on the day. The chip says
        // which way it points and changes it, so nobody has to know that typing
        // the word works — and it keeps saying so after a value is typed, which
        // a placeholder cannot.
        if (bound) {
          const choices = BOUND_CHOICES[column.key] ?? BOUND_DEFAULT;
          const chip = element("button", "fg-filter-op", BOUND_MARK[bound]);
          chip.type = "button";
          chip.tabIndex = -1;
          chip.title = `いまは「${BOUND_LABEL[bound]}」。クリックで ${choices
            .map((at) => BOUND_LABEL[at])
            .join("・")} から選べます`;
          chip.addEventListener("mousedown", (event) => event.preventDefault());
          chip.addEventListener("click", () => this.openBoundMenu(column, chip));
          cell.append(chip);

          // Behind and on track answer on their own; a box beside them would
          // have nothing to take.
          if (bound === "behind" || bound === "ahead") {
            chip.classList.add("is-wide", "is-on");
            row.append(cell);
            return;
          }
        }

        const input = element("input", "fg-filter");
        input.type = "search";
        input.value = current;
        if (current) input.classList.add("is-on");
        // The row is otherwise a line of empty boxes that reads as a blank task.
        input.placeholder = column.kind === "name" ? t("絞り込み") : "";

        // The funnel only where nothing else says what the box is: beside the
        // direction chip it lands on top of it and reads as ▼以.
        if (!input.placeholder && !bound) input.classList.add("has-funnel");
        if (bound) {
          input.classList.add("has-op");
          input.title =
            column.kind === "date"
              ? t("20260805・8/5・2026-08-05 のどれでも。左のボタンで向きを変えられます")
              : t("左のボタンで「以上」「以下」を切り替えられます");
        }
        input.dataset["column"] = column.key;
        // Filtering rebuilds the grid, and rebuilding the box the IME is
        // composing into tears the composition apart — typing ふ comes out as
        // "fう". Nothing happens until the conversion is confirmed.
        input.addEventListener("input", (event) => {
          if ((event as InputEvent).isComposing) return;

          // Eight digits become a date here too: the box takes what a cell
          // takes, and shows it back the way it will be compared.
          const digits = normalizeWidth(input.value).trim();
          if (column.kind === "date" && /^\d{8}$/.test(digits)) {
            input.value = flexibleDate(digits) ?? input.value;
          }

          this.setFilter(column.key, input.value, input.selectionStart);
        });
        input.addEventListener("compositionend", () =>
          this.setFilter(column.key, input.value, input.selectionStart),
        );
        // The grid's own keys must not fire while a filter is being typed.
        input.addEventListener("keydown", (event) => event.stopPropagation());
        cell.append(input);

        // A date is easier to point at than to type, here as much as in a cell.
        if (column.kind === "date") {
          const picker = element("input", "fg-datepicker fg-filter-picker");
          picker.type = "date";
          picker.tabIndex = -1;
          picker.title = t("カレンダーから選ぶ");
          picker.addEventListener("click", () => {
            try {
              picker.showPicker();
            } catch {
              // Older browsers open it from the indicator on their own.
            }
          });
          picker.addEventListener("change", () => {
            if (picker.value) this.setFilter(column.key, picker.value, null);
          });
          input.classList.add("has-picker");
          cell.append(picker);
        }
      }

      row.append(cell);
    });

    return row;
  }

  /**
   * What the calendar has to say about one day: the holiday's name, and who is
   * away. Both are the kind of thing someone checks by pointing at the date.
   */
  private dayNote(iso: string): string {
    const holiday = this.holidayOn(iso);
    const away = this.data.leaves
      .filter((leave) => leave.start <= iso && iso <= leave.end)
      .map((leave) => (leave.note ? `${leave.assignee}（${leave.note}）` : leave.assignee));

    return [
      holiday ? holiday.name || t("休業日") : "",
      away.length ? `休み: ${[...new Set(away)].join("、")}` : "",
    ]
      .filter(Boolean)
      .join("\n");
  }

  /**
   * The business year and quarter a date falls in.
   *
   * A fiscal year starting in April means 2026-04-01 is the first day of
   * 2026年度 Q1, and 2026-03-31 is the last day of 2025年度 Q4.
   */
  private quarterOf(date: Date): { key: string; label: string } {
    const start = this.data.fiscal_year_start || 4;
    const month = date.getUTCMonth() + 1;
    const offset = (month - start + 12) % 12;
    const year = date.getUTCFullYear() - (month < start ? 1 : 0);
    const quarter = Math.floor(offset / 3) + 1;

    // 年度 is the business year, not the calendar one, so English says FY.
    const label = this.yearLabel(new Date(Date.UTC(year, start - 1, 1)));

    return {
      key: `${year}-${quarter}`,
      label: LANG === "en" ? `FY${label} Q${quarter}` : `${label}年度 Q${quarter}`,
    };
  }

  private monthLabel(date: Date): string {
    const month = date.getUTCMonth() + 1;
    const name = MONTH_NAMES_EN[month - 1];

    return LANG === "en" && name
      ? `${name} ${this.yearLabel(date)}`
      : `${this.yearLabel(date)}年${month}月`;
  }

  /**
   * The year as this project writes it.
   *
   * The era table comes from the server rather than the code: an era is
   * announced about a month before it begins, which is no time at all to get a
   * new build onto every machine running this.
   */
  private yearLabel(date: Date): string {
    const year = date.getUTCFullYear();
    if (!this.data.japanese_era) return String(year);

    // Newest first, so the first era that has already begun is the one in force.
    const iso = date.toISOString().slice(0, 10);
    const era = this.data.eras.find((entry) => entry.from <= iso);
    if (!era) return String(year);

    const nth = year - Number(era.from.slice(0, 4)) + 1;
    return `${era.name}${nth === 1 ? t("元") : nth}`;
  }

  /**
   * A difference in days, written with the unit it was counted in.
   *
   * The chart measures the gap between two bars in calendar days, so a number
   * counted in working days will not match the pixels beside it. Saying which
   * unit it is costs three characters and removes the whole question.
   */
  private varianceText(days: number): string {
    if (days === 0) return "±0";

    const unit = LANG === "en"
      ? ` ${this.workdayBased ? "working days" : "days"}`
      : this.workdayBased ? "営業日" : "日";

    return days > 0 ? `+${days}${unit}` : `${days}${unit}`;
  }

  /**
   * The same number, said the way the row means it.
   *
   * A summary row's variance is the sum of what its children slipped, not how
   * far this bar moved — the bar's own ends are the earliest and the latest of
   * the subtree, and reading the number off them would be wrong.
   */
  private varianceLabel(task: Task, days: number): string {
    const text = this.varianceText(days);
    return task.has_children ? `トータル ${text}` : text;
  }

  /** Whether the day count leaves out weekends or holidays. */
  private get workdayBased(): boolean {
    const counting = this.data.counting;
    // Not `leave`: that only takes days off one person's own tasks, which does
    // not make the project's days into working days.
    return (
      counting.monday ||
      counting.tuesday ||
      counting.wednesday ||
      counting.thursday ||
      counting.friday ||
      counting.saturday ||
      counting.sunday ||
      counting.holidays
    );
  }

  /** The closed set a column offers, or null when it takes free text. */
  private choicesFor(column: ColumnDef): string[] | null {
    if (column.kind === "status") return this.data.statuses.map((status) => status.name);
    return column.kind === "select"
      ? (column.options?.map((option) => option.value) ?? null)
      : null;
  }

  private holidayOn(iso: string): { date: string; name: string } | undefined {
    return this.data.holidays.find((holiday) => holiday.date === iso);
  }

  /** How many rows a folded summary is hiding. */
  private hiddenCount(task: Task): number {
    const all = this.data.tasks;
    const index = all.indexOf(task);
    let count = 0;

    for (let i = index + 1; i < all.length && (all[i]?.depth ?? 0) > task.depth; i++) count++;

    return count;
  }

  private toggleCollapse(task: Task): void {
    if (!task.has_children) return;

    if (this.collapsed.has(task.id)) this.collapsed.delete(task.id);
    else this.collapsed.add(task.id);

    saveCollapsed(this.projectId, this.collapsed);
    this.computeVisible();

    // Folding the row the cursor sits inside would strand the selection, so
    // keep it on a row that still exists.
    this.select(this.row, this.column);
    this.render();
  }

  /**
   * Folds or unfolds the current row.
   *
   * Folding a leaf jumps to its parent instead, which is what pressing "close
   * this" on a row with nothing to close is asking for.
   */
  private fold(close: boolean): void {
    const task = this.selected;
    if (!task) return;

    if (task.has_children && this.collapsed.has(task.id) !== close) {
      this.toggleCollapse(task);
      return;
    }

    if (!close) return;

    // Walk up to the enclosing summary row and select it.
    for (let i = this.row - 1; i >= 0; i--) {
      const candidate = this.tasks[i];
      if (candidate && candidate.depth < task.depth) {
        this.select(i, this.column);
        this.repaintSelection();
        return;
      }
    }
  }

  /** Unfolds whatever is hiding `taskId`, so a moved row does not vanish. */
  private reveal(taskId: string): void {
    const all = this.data.tasks;
    const index = all.findIndex((task) => task.id === taskId);
    if (index < 0) return;

    let depth = all[index]?.depth ?? 0;
    let changed = false;

    for (let i = index - 1; i >= 0 && depth > 0; i--) {
      const candidate = all[i];
      if (!candidate || candidate.depth >= depth) continue;

      depth = candidate.depth;
      changed = this.collapsed.delete(candidate.id) || changed;
    }

    if (changed) {
      saveCollapsed(this.projectId, this.collapsed);
      this.computeVisible();
    }
  }

  private get selected(): Task | undefined {
    return this.tasks[this.row];
  }

  /** The built-in columns followed by whatever the project added. */
  /**
   * Every column there is, hidden ones included.
   *
   * The tooltip is allowed to name a column the table does not show — that is
   * most of the point of it: take the 製品 column off the screen and still be
   * able to ask a bar what product it is for.
   */
  private get everyColumn(): ColumnDef[] {
    return [
      ...BASE_COLUMNS,
      ...this.data.fields.map((field) => ({
        key: field.id,
        label: field.label,
        kind: field.kind as ColumnDef["kind"],
        fieldId: field.id,
        options: field.options,
      })),
    ];
  }

  /** The lines a bar adds to its tooltip, from the columns the project chose. */
  private extraTip(task: Task): string {
    if (this.data.tooltip_columns.length === 0) return "";

    const lines = this.data.tooltip_columns
      .map((key) => this.everyColumn.find((column) => column.key === key))
      .filter((column): column is ColumnDef => column !== undefined)
      .map((column) => {
        const value = this.cellDisplay(task, column).trim();
        // An empty cell says nothing worth two more lines of tooltip.
        return value && value !== "—" ? `${t(column.label)}: ${value}` : "";
      })
      .filter(Boolean);

    return lines.length > 0 ? `\n${lines.join("\n")}` : "";
  }

  private get columns(): ColumnDef[] {
    const hidden = new Set(this.data.hidden_columns);

    const all = [
      // The name column carries the outline, so it is never optional.
      ...BASE_COLUMNS.filter((column) => column.kind === "name" || !hidden.has(column.key)).map(
        // The assignee is a menu of the people on the project rather than free
        // text: 山田 and 山田さん are one person to everyone but a computer.
        (column) =>
          column.key === "assignee"
            ? {
                ...column,
                kind: "select" as const,
                options: this.data.assignees.map((person) => ({
                  value: person.name,
                  color: person.color,
                  background: person.background,
                })),
              }
            : column,
      ),
      ...this.data.fields.map((field) => ({
        key: field.id,
        label: field.label,
        kind: field.kind as ColumnDef["kind"],
        fieldId: field.id,
        options: field.options,
      })),
    ];

    // The server sends the order with every column already placed, including
    // any the stored setting never heard of. Anything missing here is a column
    // this build knows about and that one did not; it keeps its own place.
    const order = this.data.column_order;
    const rank = (column: ColumnDef) => {
      const at = order.indexOf(column.key);
      return at < 0 ? order.length + all.indexOf(column) : at;
    };

    return all.sort((a, b) => rank(a) - rank(b));
  }

  private get selectedColumn(): ColumnDef {
    return this.columns[this.column] ?? BASE_COLUMNS[0]!;
  }

  /**
   * Whether the row is drawn as late.
   *
   * Two different facts, one colour. 予定進捗 is a promise that was not kept;
   * 期限超過 is a date that went by with the work unfinished. Neither is
   * guessed, and a row can be either without being the other.
   */
  private behind(task: Task): boolean {
    return task.delayed || task.overdue > 0;
  }

  /** Whether one cell satisfies one filter box. */
  private matches(task: Task, column: ColumnDef, needle: string): boolean {
    const text = this.cellText(task, column);
    const at = this.boundFor(column);

    // Behind and on track read the checkpoints the plan named. A row that named
    // none is neither: nothing was promised, so nothing was kept or missed, and
    // putting it under 順調 would be the tool making the claim for it.
    if (at === "behind" || at === "ahead") {
      if (at === "behind") return task.delayed;
      return task.targets.length > 0 && !task.delayed;
    }

    // A column with a list of values is asked "which of these", so the answer is
    // a set and the test is equality. Contains would put 完了 under 未完了.
    if (this.choicesFor(column)) {
      const wanted = needle.split("\n").filter(Boolean);
      return wanted.length === 0 || wanted.some((value) => text === value);
    }

    const bound = parseBound(needle, at);

    if (!bound) {
      const wanted = needle.toLowerCase();
      // A date is matched on what the cell shows as well as on what is stored:
      // people type what they can see, and in Japanese that is 2026/08 while
      // the stored form is 2026-08.
      return (
        text.toLowerCase().includes(wanted) ||
        (column.kind === "date" && !!text && fullDate(text).toLowerCase().includes(wanted))
      );
    }
    // A bare direction word is somebody mid-sentence, not a condition.
    if (!bound.limit) return true;

    const value = normalizeWidth(text).trim();

    // A bound asks a question an empty cell cannot answer.
    if (!value) return false;

    if (column.kind === "days" || column.kind === "number" || column.kind === "variance"
      || column.kind === "progress") {
      const left = Number(value.replace(/[^0-9-]/g, ""));
      const right = Number(bound.limit.replace(/[^0-9-]/g, ""));

      if (!Number.isFinite(left) || !Number.isFinite(right)) return false;

      return compare(bound.at, left, right);
    }

    // The box takes a date the same way a cell does; a half-written one stays as
    // it is and compares as a prefix — "equals 2026-08" is the whole month.
    const limit = column.kind === "date" ? (flexibleDate(bound.limit) ?? bound.limit) : bound.limit;
    const cut = value.slice(0, limit.length);

    return compare(bound.at, cut, limit);
  }

  private cellText(task: Task, column: ColumnDef): string {
    if (column.fieldId) return task.values[column.fieldId] ?? "";

    switch (column.key) {
      case "name":
        return task.name;
      case "start":
        return task.start ?? "";
      case "end":
        return task.end ?? "";
      case "actual_start":
        return task.actual_start ?? "";
      case "actual_end":
        return task.actual_end ?? "";
      case "start_variance":
        return task.start_variance === null ? "" : String(task.start_variance);
      case "end_variance":
        return task.end_variance === null ? "" : String(task.end_variance);
      case "days":
        return task.days === null ? "" : String(task.days);
      case "actual_days":
        return task.actual_days === null ? "" : String(task.actual_days);
      case "progress":
        return String(task.progress);
      case "status":
        return task.status;
      case "assignee":
        return task.assignee;
      case "waits":
        // The same shape read or written: `8/17〜8/21, 9/1〜9/3`.
        return task.waits.map((span) => `${short(span.start)}〜${short(span.end)}`).join(", ");
      case "targets":
        // The same shape read or written: `8/20 30%, 8/28 100%`.
        return task.targets.map((target) => `${short(target.date)} ${target.percent}%`).join(", ");
      case "late":
        // Both words, always, so the filter can ask for either. Only one of
        // them is drawn — a column that says 順調 on every quiet row is a
        // column of noise.
        return this.behind(task) ? "遅延" : "順調";
      default:
        return task.note;
    }
  }

  /** What a cell shows when it is not being edited. */
  private cellDisplay(task: Task, column: ColumnDef): string {
    const text = this.cellText(task, column);

    if (column.key === "progress") return `${task.progress}%`;

    // A variance reads as a direction, not a number: "+3" is three days late.
    if (column.kind === "variance") {
      if (!text) return "—";
      return this.varianceText(Number(text));
    }

    if (column.kind === "date") return text ? fullDate(text) : "—";
    if (column.kind === "days") return text || "—";

    return text;
  }

  private editable(task: Task, column: ColumnDef): boolean {
    if (!this.data.can_edit) return false;
    // The day count comes from the dates; nothing writes to it.
    if (column.kind === "days") return false;
    // Nor does anything write to 遅延: it is the reading of two other columns.
    if (column.key === "late" || column.kind === "variance") return false;

    // A summary row takes its schedule from its children; writing to it would
    // be discarded on the next read.
    return !(task.has_children && ROLLED_UP.includes(column.key));
  }

  // --- selection -----------------------------------------------------------

  private select(row: number, column: number): void {
    this.row = clamp(row, 0, Math.max(0, this.tasks.length - 1));
    this.column = clamp(column, 0, this.columns.length - 1);
  }

  private move(rows: number, columns: number): void {
    this.select(this.row + rows, this.column + columns);
    this.repaintSelection();
  }

  /** Tab and Shift+Tab run past the end of a row onto the next one. */
  private step(delta: number): void {
    const width = this.columns.length;
    let index = this.row * width + this.column + delta;
    index = clamp(index, 0, this.tasks.length * width - 1);

    this.row = Math.floor(index / width);
    this.column = index % width;
    this.repaintSelection();
  }

  /**
   * Moves the highlight without rebuilding the grid.
   *
   * Navigation is the most frequent thing anyone does here and it changes two
   * cells, but a full re-render costs ~40ms at 500 rows — well past a frame,
   * and felt as lag on every arrow key. Everything else still re-renders.
   */
  private repaintSelection(): void {
    const grid = this.root.querySelector<HTMLElement>(".fg-grid");
    if (!grid) {
      this.render();
      return;
    }

    // The cursor can walk onto a row that was never drawn.
    this.reachRow(this.row);
    this.markSelection(true);
  }

  /**
   * Puts the marks and the caret on the selected cell.
   *
   * Split from `repaintSelection` because the rows can be replaced underneath
   * it: scrolling to reach a row fires the pane's scroll event a moment later,
   * the window is drawn again, and the cell holding the keyboard goes with it.
   * Whatever draws rows calls this afterwards, and the cursor survives.
   */
  private markSelection(scroll = false): void {
    const grid = this.root.querySelector<HTMLElement>(".fg-grid");
    if (!grid) return;

    for (const marked of grid.querySelectorAll(".is-selected, .is-current")) {
      marked.classList.remove("is-selected", "is-current");
    }

    const rows = grid.querySelectorAll<HTMLElement>(".fg-pane-left .fg-row.fg-data");
    const barRows = grid.querySelectorAll<HTMLElement>(".fg-bar-row");

    const row = rows[this.row - this.first];
    row?.classList.add("is-current");
    barRows[this.row - this.first]?.classList.add("is-current");

    const cell = row?.children[this.column];
    cell?.classList.add("is-selected");

    // `inline` as well as `block`: a narrowed pane clips the right-hand columns,
    // and moving to one that stays off-screen looks exactly like a dead cell.
    if (scroll) cell?.scrollIntoView({ block: "nearest", inline: "nearest" });

    // Only if the keyboard was here to begin with: this also runs on every
    // scroll, and taking focus from somebody who is reading is rude.
    if (!grid.contains(document.activeElement) && document.activeElement !== document.body) {
      return;
    }

    // Carry the caret to the new cell rather than rebuilding it.
    const typist = grid.querySelector<HTMLInputElement>(".fg-editor.is-typist");
    if (typist && cell) {
      typist.value = "";
      cell.append(typist);
      typist.focus({ preventScroll: true });
    } else if (scroll) {
      grid.focus({ preventScroll: true });
    }
  }

  // --- editing -------------------------------------------------------------

  private startEdit(seed: string | null): void {
    const task = this.selected;
    if (!task) return;

    if (!this.editable(task, this.selectedColumn)) {
      this.fail(t("集計行の日付と進捗は子タスクから決まります。"));
      return;
    }

    // A wait is a list of ranges, which one line in a cell cannot hold. This is
    // the one place with a dialog.
    if (this.selectedColumn.key === "waits") {
      this.openWaits(task);
      return;
    }

    // Same reason: 予定進捗 is a list of dates and percentages.
    if (this.selectedColumn.key === "targets") {
      this.openTargets(task);
      return;
    }

    this.editing = true;
    this.seed = seed;

    // One cell becomes an input. Every other row on the screen is still right.
    if (this.repaintRow(this.row)) this.restoreFocus();
    else this.render();
  }

  /**
   * Leave, by assignee.
   *
   * Ordinary work — somebody says they are off next week — so it belongs on the
   * schedule rather than in the project's settings, where it started out.
   *
   * The list itself is company-wide: a person is away from every plan at once.
   * What the dialog shows, and what saving replaces, is the part of it that
   * belongs to the people on this plan.
   */
  private openLeaves(): void {
    const dialog = element("dialog", "fg-dialog") as HTMLDialogElement;
    const rows = element("div", "fg-dialog-rows");

    const addRow = (leave?: {
      assignee: string;
      start: string;
      end: string;
      note: string;
      kind: string;
    }) => {
      const row = element("div", "fg-dialog-row");

      const kind = element("select", "fg-dialog-kind") as HTMLSelectElement;
      for (const [value, label] of [
        ["off", t("休み")],
        // A day worked on a weekend or a holiday: counted rather than skipped.
        ["on", t("出社")],
      ]) {
        const option = element("option", undefined, label) as HTMLOptionElement;
        option.value = value!;
        kind.append(option);
      }
      kind.value = leave?.kind === "on" ? "on" : "off";

      const who = element("select", "fg-dialog-who") as HTMLSelectElement;
      who.append(element("option", undefined, ""));
      for (const person of this.data.assignees) {
        const option = element("option", undefined, person.name) as HTMLOptionElement;
        option.value = person.name;
        who.append(option);
      }
      who.value = leave?.assignee ?? "";

      const start = element("input", "fg-dialog-date") as HTMLInputElement;
      start.type = "date";
      start.value = leave?.start ?? "";

      const end = element("input", "fg-dialog-date") as HTMLInputElement;
      end.type = "date";
      end.value = leave?.end ?? "";

      const note = element("input", "fg-dialog-reason") as HTMLInputElement;
      note.type = "text";
      note.placeholder = t("メモ（任意）");
      note.value = leave?.note ?? "";

      const remove = element("button", "fg-dialog-remove", t("削除")) as HTMLButtonElement;
      remove.type = "button";
      remove.addEventListener("click", () => row.remove());

      row.append(who, kind, start, element("span", "fg-dialog-tilde", "〜"), end, note, remove);
      rows.append(row);
      return who;
    };

    for (const leave of this.data.leaves) addRow(leave);
    if (this.data.leaves.length === 0) addRow();

    const add = element("button", "fg-dialog-add", t("＋ 休暇を追加")) as HTMLButtonElement;
    add.type = "button";
    add.addEventListener("click", () => addRow().focus());

    const save = element("button", "fg-dialog-save", t("保存")) as HTMLButtonElement;
    const cancel = element("button", "fg-dialog-cancel", t("キャンセル")) as HTMLButtonElement;
    cancel.type = "button";
    cancel.addEventListener("click", () => dialog.close());

    save.addEventListener("click", async () => {
      const leaves = [...rows.querySelectorAll(".fg-dialog-row")]
        .map((row) => {
          const [start, end] = [...row.querySelectorAll<HTMLInputElement>(".fg-dialog-date")];
          return {
            assignee: row.querySelector<HTMLSelectElement>(".fg-dialog-who")?.value ?? "",
            kind: row.querySelector<HTMLSelectElement>(".fg-dialog-kind")?.value ?? "off",
            start: start?.value ?? "",
            end: end?.value ?? "",
            note: row.querySelector<HTMLInputElement>(".fg-dialog-reason")?.value ?? "",
          };
        })
        // A row with nothing in it is a row somebody added and did not use.
        .filter((leave) => leave.assignee && leave.start && leave.end);

      dialog.close();

      await this.send(`/api/projects/${encodeURIComponent(this.projectId)}/leaves`, {
        method: "POST",
        body: { leaves },
      });
    });

    dialog.append(
      element("h2", "fg-dialog-title", t("担当者の休暇 / 出社")),
      element(
        "p",
        "fg-dialog-help",
        t("休みの日はその人のタスクの日数にも遅れの判定にも入りません。逆に「出社」は、土日祝でもその日を数えます。") +
          t("予定は人につくので、ここでの登録はその人が出ている全部のプロジェクトに効きます。"),
      ),
      rows,
      add,
    );

    const buttons = element("div", "fg-dialog-buttons");
    buttons.append(cancel, save);
    dialog.append(buttons);

    dialog.addEventListener("keydown", (event) => event.stopPropagation());
    dialog.addEventListener("close", () => {
      dialog.remove();
      this.root.querySelector<HTMLElement>(".fg-grid")?.focus({ preventScroll: true });
    });

    // Not `this.root`: the island replaces its own children on every render,
    // and a dialog parked there vanishes the moment anything redraws.
    document.body.append(dialog);
    dialog.showModal();
    rows.querySelector<HTMLSelectElement>(".fg-dialog-who")?.focus();
  }

  /**
   * Editing 予定進捗.
   *
   * The same dialog as the waits, because it is the same kind of thing: a short
   * list a person keeps on one task. It exists at all because the alternative
   * was reading the plan out of the dates — elapsed over total — and calling a
   * task late for not being linear. Work is not linear. Somebody has to say
   * what the plan is, and this is where they say it.
   */
  private openTargets(task: Task): void {
    const dialog = element("dialog", "fg-dialog") as HTMLDialogElement;
    const rows = element("div", "fg-dialog-rows");

    const addRow = (target?: { date: string; percent: number }) => {
      const row = element("div", "fg-dialog-row");

      const date = element("input", "fg-dialog-date") as HTMLInputElement;
      date.type = "date";
      date.required = true;
      date.value = target?.date ?? "";

      const percent = element("input", "fg-dialog-percent") as HTMLInputElement;
      percent.type = "number";
      percent.min = "0";
      percent.max = "100";
      percent.step = "5";
      percent.value = target === undefined ? "" : String(target.percent);

      const remove = element("button", "fg-dialog-remove", t("削除")) as HTMLButtonElement;
      remove.type = "button";
      remove.addEventListener("click", () => row.remove());

      row.append(
        date,
        element("span", "fg-dialog-tilde", t("までに")),
        percent,
        element("span", "fg-dialog-unit", "%"),
        remove,
      );
      rows.append(row);
      return date;
    };

    for (const target of task.targets) addRow(target);
    if (task.targets.length === 0) addRow();

    const add = element("button", "fg-dialog-add", t("＋ 予定を追加")) as HTMLButtonElement;
    add.type = "button";
    add.addEventListener("click", () => addRow().focus());

    const save = element("button", "fg-dialog-save", t("保存")) as HTMLButtonElement;
    const cancel = element("button", "fg-dialog-cancel", t("キャンセル")) as HTMLButtonElement;
    cancel.type = "button";
    cancel.addEventListener("click", () => dialog.close());

    save.addEventListener("click", async () => {
      const lines: string[] = [];

      for (const row of rows.querySelectorAll(".fg-dialog-row")) {
        const date = row.querySelector<HTMLInputElement>(".fg-dialog-date");
        const percent = row.querySelector<HTMLInputElement>(".fg-dialog-percent");

        // A date with no percentage is half a sentence, and so is the reverse.
        if (!date?.value || !percent?.value) continue;

        lines.push(`${date.value} ${percent.value}%`);
      }

      dialog.close();

      await this.send(`/api/projects/${encodeURIComponent(this.projectId)}/tasks/${task.id}`, {
        method: "POST",
        body: { field: "targets", value: lines.join("\n") },
        follow: task.id,
      });
    });

    dialog.append(
      element("h2", "fg-dialog-title", `予定進捗 — ${task.name || t("（無題）")}`),
      element(
        "p",
        "fg-dialog-help",
        t("その日を過ぎても実進捗が届いていなければ遅れになります。間の日は判定しません。入れなければ、この行は進捗では遅れになりません。"),
      ),
      rows,
      add,
    );

    const buttons = element("div", "fg-dialog-buttons");
    buttons.append(cancel, save);
    dialog.append(buttons);

    dialog.addEventListener("keydown", (event) => event.stopPropagation());
    dialog.addEventListener("close", () => {
      dialog.remove();
      this.root.querySelector<HTMLElement>(".fg-grid")?.focus({ preventScroll: true });
    });

    document.body.append(dialog);
    dialog.showModal();
    rows.querySelector<HTMLInputElement>(".fg-dialog-date")?.focus();
  }

  /**
   * Editing the waits.
   *
   * A cell holds one line, and a wait is a list of ranges with reasons — asking
   * for that as text works for whoever wrote the parser and for nobody else.
   * The dialog is the interface; the text form stays as what it sends.
   */
  private openWaits(task: Task): void {
    const dialog = element("dialog", "fg-dialog") as HTMLDialogElement;
    const rows = element("div", "fg-dialog-rows");

    const addRow = (wait?: { start: string; end: string; reason: string; open: boolean }) => {
      const row = element("div", "fg-dialog-row");

      const start = element("input", "fg-dialog-date") as HTMLInputElement;
      start.type = "date";
      start.required = true;
      start.value = wait?.start ?? "";

      const end = element("input", "fg-dialog-date") as HTMLInputElement;
      end.type = "date";
      // Empty means still waiting, and the days keep counting up to today.
      end.value = wait && !wait.open ? wait.end : "";
      end.placeholder = t("継続中");

      const reason = element("input", "fg-dialog-reason") as HTMLInputElement;
      reason.type = "text";
      reason.placeholder = t("理由（任意）");
      reason.value = wait?.reason ?? "";

      const remove = element("button", "fg-dialog-remove", t("削除")) as HTMLButtonElement;
      remove.type = "button";
      remove.addEventListener("click", () => row.remove());

      row.append(start, element("span", "fg-dialog-tilde", "〜"), end, reason, remove);
      rows.append(row);
      return start;
    };

    for (const wait of task.waits) addRow(wait);
    if (task.waits.length === 0) addRow();

    const add = element("button", "fg-dialog-add", t("＋ 期間を追加")) as HTMLButtonElement;
    add.type = "button";
    add.addEventListener("click", () => addRow().focus());

    const save = element("button", "fg-dialog-save", t("保存")) as HTMLButtonElement;
    const cancel = element("button", "fg-dialog-cancel", t("キャンセル")) as HTMLButtonElement;
    cancel.type = "button";
    cancel.addEventListener("click", () => dialog.close());

    save.addEventListener("click", async () => {
      const lines: string[] = [];

      for (const row of rows.querySelectorAll(".fg-dialog-row")) {
        const [start, end] = [...row.querySelectorAll<HTMLInputElement>(".fg-dialog-date")];
        const reason = row.querySelector<HTMLInputElement>(".fg-dialog-reason")?.value.trim() ?? "";

        if (!start?.value) continue;

        const range = `${start.value}〜${end?.value ?? ""}`;
        lines.push(reason ? `${range} ${reason}` : range);
      }

      dialog.close();

      await this.send(`/api/projects/${encodeURIComponent(this.projectId)}/tasks/${task.id}`, {
        method: "POST",
        body: { field: "waits", value: lines.join("\n") },
        follow: task.id,
      });
    });

    dialog.append(
      element("h2", "fg-dialog-title", `待ち — ${task.name || t("（無題）")}`),
      element(
        "p",
        "fg-dialog-help",
        t("終わりを空にすると「まだ待っている」になり、今日まで数え続けます。待ちの日数は日数からも遅れの判定からも外れます。"),
      ),
      rows,
      add,
    );

    const buttons = element("div", "fg-dialog-buttons");
    buttons.append(cancel, save);
    dialog.append(buttons);

    // The grid's own keys must not fire while the dialog is up.
    dialog.addEventListener("keydown", (event) => event.stopPropagation());
    dialog.addEventListener("close", () => {
      dialog.remove();
      this.root.querySelector<HTMLElement>(".fg-grid")?.focus({ preventScroll: true });
    });

    // Not `this.root`: the island replaces its own children on every render,
    // and a dialog parked there vanishes the moment anything redraws.
    document.body.append(dialog);
    dialog.showModal();
    rows.querySelector<HTMLInputElement>(".fg-dialog-date")?.focus();
  }

  private cancelEdit(): void {
    this.editing = false;
    this.seed = null;

    if (this.repaintRow(this.row)) this.restoreFocus();
    else this.render();
  }

  private async commitEdit(raw: string, after: "down" | "right" | "stay"): Promise<void> {
    const task = this.selected;
    const column = this.selectedColumn;

    // Dates and numbers accept what a Japanese keyboard produces in kana mode,
    // and a date is written back the one way it is stored: type 20260801 and
    // the cell reads 2026-08-01, the same as any other date field.
    const value =
      column.kind === "date"
        ? (flexibleDate(raw) ?? normalizeWidth(raw).trim())
        : column.kind === "progress" || column.kind === "number"
          ? normalizeWidth(raw).trim()
          : raw;

    this.editing = false;
    this.seed = null;

    if (after === "down") this.select(this.row + 1, this.column);
    if (after === "right") this.step(1);

    if (!task || value === this.cellText(task, column)) {
      this.render();
      return;
    }

    // Snapshot before touching anything: the optimistic write goes through the
    // live task objects, so a copy taken afterwards would already hold the new
    // value and roll back to nothing.
    const rollback = structuredClone(this.data);

    // Show the typed value straight away. The server owns the derived numbers,
    // so ancestors stay stale for the length of one round trip.
    this.applyLocally(task, column, value);
    this.render();

    await this.send(`/api/projects/${encodeURIComponent(this.projectId)}/tasks/${task.id}`, {
      method: "POST",
      body: column.fieldId
        ? { field: "custom", field_id: column.fieldId, value }
        : { field: column.key, value },
      rollback,
    });
  }

  private applyLocally(task: Task, column: ColumnDef, value: string): void {
    if (column.fieldId) {
      task.values[column.fieldId] = value;
      return;
    }

    switch (column.key) {
      case "name":
        task.name = value;
        break;
      case "start":
        task.start = value || null;
        break;
      case "end":
        task.end = value || null;
        break;
      case "progress": {
        const parsed = Number(value);
        if (Number.isFinite(parsed)) task.progress = clamp(Math.round(parsed), 0, 100);
        break;
      }
      case "status":
        task.status = value;
        break;
      case "assignee":
        task.assignee = value;
        break;
      default:
        task.note = value;
    }
  }

  // --- rows ----------------------------------------------------------------

  /** Keeps what was typed, then opens a fresh row under it. */
  private async commitAndInsert(raw: string): Promise<void> {
    await this.commitEdit(raw, "stay");
    await this.insertRow();
  }

  private async insertRow(): Promise<void> {
    if (!this.data.can_edit) return;

    const after = this.selected?.id ?? null;

    const result = await this.send(
      `/api/projects/${encodeURIComponent(this.projectId)}/tasks`,
      { method: "POST", body: { after } },
    );

    if (!result?.task_id) return;

    const index = this.tasks.findIndex((task) => task.id === result.task_id);
    if (index >= 0) {
      this.select(index, 0);
      this.startEdit(null);
    }
  }

  /**
   * Moves the row through the outline.
   *
   * At the edges the server changes nothing and answers with the grid as it
   * stands, the way an outliner swallows the keystroke instead of complaining.
   */
  private async moveRow(action: "indent" | "outdent" | "up" | "down"): Promise<void> {
    const task = this.selected;
    if (!task || !this.data.can_edit) return;

    // Clear any leftover explanation, so a stale one is never mistaken for the
    // answer to the move being made now.
    this.notice = null;

    // Noted before it moves: this is the instruction that puts it back.
    const was = this.spotOf(task.id);

    const result = await this.send(
      `/api/projects/${encodeURIComponent(this.projectId)}/tasks/${task.id}/move`,
      { method: "POST", body: { action }, follow: task.id, was: was ?? undefined },
    );

    // The server refused for a reason. Saying it is the difference between
    // "this rule does not apply here" and "this app is broken".
    if (result?.note) this.showNotice(result.note);
    else this.render();
  }

  private async deleteRow(): Promise<void> {
    const task = this.selected;
    if (!task || !this.data.can_edit) return;

    const label = task.name || t("無題のタスク");
    const question = task.has_children
      ? `「${label}」と、その子タスクをすべて削除します。よろしいですか？`
      : `「${label}」を削除します。よろしいですか？`;

    if (!window.confirm(question)) return;

    await this.send(
      `/api/projects/${encodeURIComponent(this.projectId)}/tasks/${task.id}`,
      { method: "DELETE" },
    );

    this.select(this.row, this.column);
    this.render();
  }

  // --- server --------------------------------------------------------------

  private async send(
    url: string,
    options: {
      method: string;
      body?: unknown;
      /** State from before an optimistic edit, to restore if the server says no. */
      rollback?: GridData;
      /** Keep this task selected even if the response moved it to another row. */
      follow?: string;
      /** Where the row stood before this call moved it, so it can be put back. */
      was?: Spot;
    },
  ): Promise<Mutation | null> {
    // Callers that edited optimistically pass the state from before the edit;
    // the rest have not touched anything yet.
    const before = options.rollback ?? this.data;
    this.busy = true;

    // Filled in on the way through, when a response actually arrives.
    let shape: string | null = null;
    let rows: string[] = [];
    // The rows a patch touched, when the answer was a patch. Drawing those and
    // nothing else is the point of the patch: comparing every row against
    // every row would put the cost back where it was taken from.
    let touched: string[] | null = null;

    try {
      const headers: Record<string, string> = { "x-fugantt-client": CLIENT_ID };
      if (options.body) headers["content-type"] = "application/json";

      const response = await fetch(url, {
        method: options.method,
        headers,
        body: options.body ? JSON.stringify(options.body) : undefined,
      });

      if (!response.ok) {
        // The server refused the value, so the optimistic edit was a lie.
        this.setData(before);
        this.fail(await this.reason(response));
        return null;
      }

      const result = (await response.json()) as Mutation;

      if (result.patch) {
        const drew = this.applyPatch(result.patch);

        // The patch did not fit what this browser is holding, so it stops
        // guessing and asks for the plan itself.
        if (drew === null) await this.refetch();
        else touched = drew;
      } else if (result.grid) {
        // Taken before the new table lands, so the two can be compared row by
        // row and only the rows that moved are drawn again.
        shape = this.shape();
        rows = this.tasks.map((task) => JSON.stringify(task));

        this.setData(result.grid);
      }

      this.error = null;

      // Recorded here rather than at each of the eight places that change a
      // cell: this is the one point that knows both what was there (the state
      // this call started from) and what the server made of what was sent.
      // After the answer has landed, so a row that moved can be asked where
      // it ended up.
      this.remember(url, options, before, result);

      // A move changes which row the task sits on, so follow the task rather
      // than staying on a row number that now means something else. Indenting
      // into a folded parent would otherwise hide the row that just moved.
      if (options.follow) this.reveal(options.follow);

      const moved = options.follow
        ? this.tasks.findIndex((task) => task.id === options.follow)
        : -1;

      this.select(moved >= 0 ? moved : this.row, this.column);

      return result;
    } catch {
      this.setData(before);
      this.fail(t("保存できませんでした。接続を確認してください。"));
      return null;
    } finally {
      this.busy = false;

      if (touched) this.repaintRows(touched);
      else if (shape === null) this.render();
      else this.repaintChanged(shape, rows);
    }
  }

  /**
   * Takes the rows one write changed into the plan this browser is holding.
   *
   * Returns the rows to draw again, or `null` when the patch cannot be trusted
   * on top of what is here — a row it builds on is missing, or the plan is not
   * the length the server says it is. Then the caller asks for the whole plan:
   * slow, and right, which is the correct order for those two.
   */
  private applyPatch(patch: Patch): string[] | null {
    const wasVisible = this.tasks.length;
    const structural =
      patch.range_start !== this.data.range_start ||
      patch.range_end !== this.data.range_end ||
      (patch.removed?.length ?? 0) > 0;

    if (patch.removed?.length) {
      const gone = new Set(patch.removed);
      this.data.tasks = this.data.tasks.filter((task) => !gone.has(task.id));
    }

    if (patch.moved && !this.carry(patch.moved)) return null;

    const at = new Map(this.data.tasks.map((task, index) => [task.id, index]));
    const fresh: Task[] = [];

    for (const row of patch.rows) {
      const index = at.get(row.id);
      if (index === undefined) fresh.push(row);
      else this.data.tasks[index] = row;
    }

    if (fresh.length > 0) {
      // `after` names the row the new one follows in the flattened order. Not
      // knowing that row means this browser is holding a different plan.
      const behind = patch.after ? this.data.tasks.findIndex((task) => task.id === patch.after) : -1;
      if (patch.after !== undefined && behind < 0) return null;

      this.data.tasks.splice(behind + 1, 0, ...fresh);
    }

    this.data.revision = patch.revision;
    this.data.range_start = patch.range_start;
    this.data.range_end = patch.range_end;

    if (this.data.tasks.length !== patch.total) return null;

    this.computeVisible();

    // A row arriving, leaving, or slipping through a filter moves every row
    // number after it, and a row number is what the drawn table is indexed by.
    if (structural || patch.moved || fresh.length > 0 || this.tasks.length !== wasVisible) {
      return [];
    }

    return patch.rows.map((row) => row.id);
  }

  /**
   * Moves a row, and everything under it, to where the server put it.
   *
   * The plan is one flat list ordered depth first, so a subtree is the row
   * plus the run of deeper rows behind it — the same fact folding already
   * relies on. Cut that run out, shift its depths by however far the row
   * moved, and put it back after the row it now follows.
   *
   * False when this browser cannot see where it is meant to go, which sends
   * the caller back for the whole plan.
   */
  private carry(moved: { id: string; after?: string; depth: number }): boolean {
    const at = this.data.tasks.findIndex((task) => task.id === moved.id);
    if (at < 0) return false;

    const row = this.data.tasks[at];
    if (!row) return false;

    let end = at + 1;
    while (end < this.data.tasks.length && (this.data.tasks[end]?.depth ?? 0) > row.depth) end++;

    const subtree = this.data.tasks.splice(at, end - at);
    const shift = moved.depth - row.depth;
    for (const task of subtree) task.depth += shift;

    // Looked up after the cut: the row it follows may have been sitting behind
    // the rows that just left.
    const behind = moved.after
      ? this.data.tasks.findIndex((task) => task.id === moved.after)
      : -1;

    if (moved.after !== undefined && behind < 0) return false;

    this.data.tasks.splice(behind + 1, 0, ...subtree);

    return true;
  }

  /** The plan as the server has it, when a patch could not be trusted. */
  private async refetch(): Promise<void> {
    const response = await fetch(
      `/api/projects/${encodeURIComponent(this.projectId)}/grid`,
      { headers: { accept: "application/json" } },
    );

    if (response.ok) this.setData((await response.json()) as GridData);
  }

  /** Draws the rows a patch touched, and leaves the rest of the table alone. */
  private repaintRows(ids: string[]): void {
    const showing = this.root.querySelector(".fg-error") !== null;

    // No ids means the table's shape moved, not just some values in it.
    if (ids.length === 0 || showing !== (this.error !== null)) {
      this.render();
      return;
    }

    for (const id of ids) {
      const index = this.tasks.findIndex((task) => task.id === id);
      // A row the filters are hiding needs no drawing, and that counts as done.
      if (index < 0) continue;

      if (!this.repaintRow(index)) {
        this.render();
        return;
      }
    }

    this.paintNotice();
    this.root.querySelector(".fg-toolbar")?.replaceWith(this.renderToolbar());
    this.updateFilterCount();
    this.restoreFocus();
  }

  /** Files one change away, so Ctrl+Z has something to put back. */
  private remember(
    url: string,
    options: { method: string; body?: unknown; follow?: string; was?: Spot },
    before: GridData,
    result: Mutation,
  ): void {
    if (this.replaying || options.method === "GET") return;

    const body = options.body as { field?: string; field_id?: string } | undefined;
    // From the address rather than from `follow`, which is about where to leave
    // the selection and is not passed by the calls that never move a row.
    const taskId = decodeURIComponent(/\/tasks\/([^/?#]+)$/.exec(url)?.[1] ?? "");

    // A row that moved: it is where the answer put it, and `was` is where the
    // caller saw it standing before it asked.
    if (options.was && result.patch?.moved) {
      const to = this.spotOf(result.patch.moved.id);
      if (to) {
        this.done.push({ kind: "move", taskId: result.patch.moved.id, from: options.was, to });
        this.undone = [];
      }
      return;
    }

    // A row that was added, remembered by where it landed.
    if (url.endsWith("/tasks") && result.task_id) {
      const at = this.spotOf(result.task_id);
      if (at) {
        this.done.push({ kind: "add", taskId: result.task_id, at });
        this.undone = [];
      }
      return;
    }

    // Deleting is not undoable, and not skippable either. A barrier keeps
    // Ctrl+Z from stepping over the gap and undoing something older while the
    // row it belonged to is still missing.
    if (!body?.field || !taskId) {
      this.done.push({ kind: "barrier" });
      this.undone = [];
      return;
    }

    const was = before.tasks.find((task) => task.id === taskId);
    const now = (result.grid?.tasks ?? result.patch?.rows ?? []).find(
      (task) => task.id === taskId,
    );
    if (!was || !now) return;

    const field = body.field;
    const fieldId = body.field_id;

    const step: Step = {
      kind: "cell",
      taskId,
      field,
      fieldId,
      before: {
        send: this.sendableValue(was, field, fieldId),
        stored: this.storedValue(was, field, fieldId),
      },
      after: {
        send: this.sendableValue(now, field, fieldId),
        stored: this.storedValue(now, field, fieldId),
      },
    };

    // The server may have made nothing of it — a value that normalises to what
    // was already there. Nothing happened, so there is nothing to take back.
    if (step.before.stored === step.after.stored) return;


    this.done.push(step);
    // A new change is a new branch: what was undone is no longer ahead of us.
    this.undone = [];
  }

  /**
   * What one field of a task holds, as the server stores it.
   *
   * This is what `expect` is compared against, so it has to match the column
   * exactly — not what the cell shows a person.
   */
  private storedValue(task: Task, field: string, fieldId?: string): string {
    switch (field) {
      case "name":
        return task.name;
      case "start":
        return task.start ?? "";
      case "end":
        return task.end ?? "";
      case "actual_start":
        return task.actual_start ?? "";
      case "actual_end":
        return task.actual_end ?? "";
      case "schedule":
        return `${task.start ?? ""}/${task.end ?? ""}`;
      case "actual_schedule":
        return `${task.actual_start ?? ""}/${task.actual_end ?? ""}`;
      case "progress":
        return String(task.progress);
      case "status":
        return task.status;
      case "assignee":
        return task.assignee;
      case "note":
        return task.note;
      case "waits":
        return task.waits
          .map((wait) => {
            const range = `${wait.start}/${wait.open ? "" : wait.end}`;
            return wait.reason ? `${range}:${wait.reason}` : range;
          })
          .join("\n");
      case "targets":
        return task.targets.map((target) => `${target.date}/${target.percent}`).join("\n");
      case "custom":
        return (fieldId && task.values[fieldId]) || "";
      default:
        return "";
    }
  }

  /**
   * The same value, in a form the server will take back.
   *
   * 待ち is kept as `from/to:reason` and written as `8/17〜8/21 reason`; the
   * parser has never had to read its own output, and teaching it to would be
   * two ways of saying one thing. Everything else is stored as it is written.
   */
  private sendableValue(task: Task, field: string, fieldId?: string): string {
    if (field !== "waits") return this.storedValue(task, field, fieldId);

    return task.waits
      .map((wait) => {
        const range = `${wait.start}〜${wait.open ? "" : wait.end}`;
        return wait.reason ? `${range} ${wait.reason}` : range;
      })
      .join("\n");
  }

  /**
   * Puts back the last change this tab made.
   *
   * The value it is putting back travels with what it expects to find, so a
   * cell somebody else has since touched is refused rather than quietly
   * overwritten. Undo is for taking back your own work.
   */
  private async replay(direction: "undo" | "redo"): Promise<void> {
    const from = direction === "undo" ? this.done : this.undone;

    const step = from.pop();
    if (!step) {
      this.fail(direction === "undo" ? t("取り消せる操作がありません。") : t("やり直せる操作がありません。"));
      return;
    }

    const done =
      step.kind === "cell"
        ? await this.replayCell(step, direction)
        : step.kind === "move"
          ? await this.replayMove(step, direction)
          : step.kind === "add"
            ? await this.replayAdd(step, direction)
            : this.refuse(t("行の削除は取り消せません。もう一度押すと、その前の変更を取り消します。"));

    // Refused — by the server, or by somebody else's edit. The step stays off
    // the stack: pressing again should try the one before it, not this one.
    if (!done) return;

    (direction === "undo" ? this.undone : this.done).push(step);
  }

  /** Says why nothing happened, and answers "not done" for the caller. */
  private refuse(why: string): boolean {
    this.fail(why);
    return false;
  }

  /** Puts one cell back to the value on the other side of the change. */
  private async replayCell(
    step: Extract<Step, { kind: "cell" }>,
    direction: "undo" | "redo",
  ): Promise<boolean> {
    if (!this.data.tasks.some((task) => task.id === step.taskId)) {
      this.fail(t("その行はもうありません。"));
      return false;
    }

    const target = direction === "undo" ? step.before : step.after;
    const expect = direction === "undo" ? step.after.stored : step.before.stored;

    this.replaying = true;
    const result = await this.send(
      `/api/projects/${encodeURIComponent(this.projectId)}/tasks/${step.taskId}`,
      {
        method: "POST",
        body: { field: step.field, field_id: step.fieldId, value: target.send, expect },
        follow: step.taskId,
      },
    );
    this.replaying = false;

    return result !== null;
  }

  /**
   * Puts a row back where it stood.
   *
   * `place` takes a parent and the sibling to land after, which is exactly
   * what was written down before the row was moved — the same instruction,
   * pointed the other way.
   */
  private async replayMove(
    step: Extract<Step, { kind: "move" }>,
    direction: "undo" | "redo",
  ): Promise<boolean> {
    if (!this.data.tasks.some((task) => task.id === step.taskId)) {
      this.fail(t("その行はもうありません。"));
      return false;
    }

    // Somebody else has moved it since. Putting it back where this browser
    // remembers it would undo their move as well as this one.
    const now = this.spotOf(step.taskId);
    const held = direction === "undo" ? step.to : step.from;
    if (!now || now.parent !== held.parent || now.after !== held.after) {
      this.fail(t("その行は誰かが動かしました。"));
      return false;
    }

    const target = direction === "undo" ? step.from : step.to;

    this.replaying = true;
    const result = await this.send(
      `/api/projects/${encodeURIComponent(this.projectId)}/tasks/${step.taskId}/place`,
      { method: "POST", body: target, follow: step.taskId, was: now },
    );
    this.replaying = false;

    return result !== null;
  }

  /**
   * Takes back a row that was added, or puts it back.
   *
   * Undoing removes it — but only while it is still empty. Whatever was typed
   * into it is a change of its own and comes off the stack first; a row with
   * something in it is somebody's work, and Ctrl+Z is not a way to lose it.
   *
   * Redoing adds a row again, which the server gives a new id. Every step that
   * still names the old one is pointed at the new one, so the stack keeps
   * working from here.
   */
  private async replayAdd(
    step: Extract<Step, { kind: "add" }>,
    direction: "undo" | "redo",
  ): Promise<boolean> {
    if (direction === "redo") {
      this.replaying = true;
      const result = await this.send(
        `/api/projects/${encodeURIComponent(this.projectId)}/tasks`,
        { method: "POST", body: { after: step.at.after ?? step.at.parent } },
      );
      this.replaying = false;

      if (!result?.task_id) return false;

      // Put back where it was, in case it went in beside its parent rather
      // than inside it: `after` names a sibling, and the first child has none.
      if (step.at.after === null && step.at.parent !== null) {
        this.replaying = true;
        await this.send(
          `/api/projects/${encodeURIComponent(this.projectId)}/tasks/${result.task_id}/place`,
          { method: "POST", body: step.at, follow: result.task_id },
        );
        this.replaying = false;
      }

      this.rename(step.taskId, result.task_id);
      return true;
    }

    const row = this.data.tasks.find((task) => task.id === step.taskId);
    if (!row) {
      this.fail(t("その行はもうありません。"));
      return false;
    }

    if (row.has_children) {
      this.fail(t("その行には子タスクがあるので、取り消しでは消しません。"));
      return false;
    }

    if (this.written(row)) {
      this.fail(t("書き込みのある行は、取り消しでは消しません。"));
      return false;
    }

    this.replaying = true;
    const result = await this.send(
      `/api/projects/${encodeURIComponent(this.projectId)}/tasks/${step.taskId}`,
      { method: "DELETE" },
    );
    this.replaying = false;

    return result !== null;
  }

  /** Whether anything was ever put in this row. */
  private written(task: Task): boolean {
    return (
      task.name.trim() !== "" ||
      task.start !== null ||
      task.end !== null ||
      task.actual_start !== null ||
      task.actual_end !== null ||
      task.progress !== 0 ||
      task.assignee.trim() !== "" ||
      task.note.trim() !== "" ||
      Object.values(task.values).some((value) => value.trim() !== "")
    );
  }

  /** Points every step at the id a row came back with. */
  private rename(was: string, now: string): void {
    for (const step of [...this.done, ...this.undone]) {
      if (step.kind !== "barrier" && step.taskId === was) step.taskId = now;
    }
  }

  /**
   * Where a row sits among its siblings.
   *
   * The plan is one flat list ordered depth first, so the parent is the first
   * row above it that is shallower, and the previous sibling is the first row
   * above it at the same depth — anything deeper in between belongs to that
   * sibling.
   */
  private spotOf(id: string): Spot | null {
    const at = this.data.tasks.findIndex((task) => task.id === id);
    const row = this.data.tasks[at];
    if (!row) return null;

    let parent: string | null = null;
    let after: string | null = null;

    for (let index = at - 1; index >= 0; index--) {
      const above = this.data.tasks[index];
      if (!above || above.depth > row.depth) continue;

      if (above.depth === row.depth) {
        if (after === null) after = above.id;
        continue;
      }

      parent = above.id;
      break;
    }

    return { parent, after };
  }

  private async reason(response: Response): Promise<string> {
    if (response.status === 403) return t("集計行の日付と進捗は子タスクから決まります。");

    // The framework prefixes its own status text; the message is what matters.
    const text = (await response.text()).replace(/^bad request:\s*/i, "").trim();
    return text || `保存できませんでした（${response.status}）。`;
  }

  private fail(message: string): void {
    this.error = message;
    this.render();
  }

  /** A passing line that clears itself. Not an error, so not the error bar. */
  private showNotice(message: string): void {
    this.notice = message;
    this.paintNotice();

    window.clearTimeout(this.noticeTimer);
    this.noticeTimer = window.setTimeout(() => {
      this.notice = null;
      this.paintNotice();
    }, 4000);
  }

  /**
   * Puts the passing line on screen, or takes it off.
   *
   * On its own, rather than by drawing the table again: the line says somebody
   * else changed something, and at two thousand rows redrawing everything to
   * say so costs a second — for a sentence that leaves again in four.
   */
  private paintNotice(): void {
    const showing = this.root.querySelector(".fg-notice");

    if (this.notice === null) {
      showing?.remove();
      return;
    }

    const line = element("div", "fg-notice", this.notice);
    if (showing) showing.replaceWith(line);
    else this.root.prepend(line);
  }

  // --- keyboard ------------------------------------------------------------

  private onKeyDown(event: KeyboardEvent): void {
    // A key pressed while an IME is converting belongs to the IME. The Enter
    // that confirms 「やまだ→山田」 arrives here too, and taking it as "commit
    // and move down" hands the rest of the conversion to the next row.
    if (event.isComposing || event.keyCode === 229) return;

    if (this.editing) {
      this.onEditKeyDown(event);
      return;
    }

    const meta = event.ctrlKey || event.metaKey;

    // Undo and redo, in both spellings: Ctrl+Y is what Windows presses and
    // ⌘⇧Z is what the Mac does, and neither person should have to learn the
    // other's. The editor keeps its own undo — this only runs between edits.
    if (meta && (event.key === "z" || event.key === "Z")) {
      void this.replay(event.shiftKey ? "redo" : "undo");
      event.preventDefault();
      return;
    }

    if (meta && (event.key === "y" || event.key === "Y")) {
      void this.replay("redo");
      event.preventDefault();
      return;
    }

    // Alt turns the arrows into outline moves, the way every outliner does it.
    // On Windows, Alt+Left/Right is browser back/forward, so the preventDefault
    // at the end of this branch is load-bearing rather than tidy.
    if (event.altKey) {
      switch (event.key) {
        case "ArrowRight":
          void this.moveRow("indent");
          break;
        case "ArrowLeft":
          void this.moveRow("outdent");
          break;
        case "ArrowUp":
          void this.moveRow("up");
          break;
        case "ArrowDown":
          void this.moveRow("down");
          break;
        default:
          return;
      }

      event.preventDefault();
      return;
    }

    switch (event.key) {
      case "ArrowUp":
        this.move(-1, 0);
        break;
      case "ArrowDown":
        this.move(1, 0);
        break;
      case "ArrowLeft":
        if (meta) this.fold(true);
        else this.move(0, -1);
        break;
      case "ArrowRight":
        if (meta) this.fold(false);
        else this.move(0, 1);
        break;
      case "Tab":
        this.step(event.shiftKey ? -1 : 1);
        break;
      // Enter opens the cell rather than stepping past it: the reason to be on
      // a cell is almost always to change it, and ↓ already moves down.
      case "Enter":
        if (meta) void this.insertRow();
        else this.startEdit(null);
        break;
      case "F2":
        this.startEdit(null);
        break;
      case "Home":
        if (meta) this.select(0, 0);
        else this.select(this.row, 0);
        this.repaintSelection();
        break;
      case "End":
        if (meta) this.select(this.tasks.length - 1, this.columns.length - 1);
        else this.select(this.row, this.columns.length - 1);
        this.repaintSelection();
        break;
      case "PageUp":
        this.move(-PAGE_ROWS, 0);
        break;
      case "PageDown":
        this.move(PAGE_ROWS, 0);
        break;
      case "Delete":
      case "Backspace":
        if (meta) void this.deleteRow();
        else void this.commitEdit("", "stay");
        break;
      default:
        // Printable keys, including anything an IME is converting, land in the
        // typist field and open the editor from there.
        return;
    }

    event.preventDefault();
  }

  private onEditKeyDown(event: KeyboardEvent): void {
    const input = event.target as HTMLInputElement | HTMLSelectElement;

    switch (event.key) {
      case "Enter":
        // ⌘Enter means "add a row below" everywhere else in the grid, and a
        // half-typed name is no reason for it to mean something else: typing a
        // list is exactly when it is held down. Without this, every second
        // press only closed the editor, and the row came on the press after.
        if (event.ctrlKey || event.metaKey) void this.commitAndInsert(input.value);
        else void this.commitEdit(input.value, "down");
        break;
      case "Tab":
        void this.commitEdit(input.value, event.shiftKey ? "stay" : "right");
        break;
      case "Escape":
        this.cancelEdit();
        break;
      default:
        return;
    }

    event.preventDefault();
  }

  // --- rendering -----------------------------------------------------------

  private render(): void {
    const chart = this.root.querySelector<HTMLElement>(".fg-pane-chart");
    if (chart) this.scrollLeft = chart.scrollLeft;

    const left = this.root.querySelector<HTMLElement>(".fg-pane-left");
    if (left) this.scrollTop = left.scrollTop;

    const typing = this.filterFocus;
    this.filterFocus = null;

    const parts: HTMLElement[] = [];

    if (this.error) {
      const banner = element("div", "fg-error");
      banner.append(element("span", undefined, this.error));

      const dismiss = element("button", "fg-error-close", t("閉じる"));
      dismiss.type = "button";
      dismiss.addEventListener("click", () => {
        this.error = null;
        this.render();
      });
      banner.append(dismiss);

      parts.push(banner);
    }

    if (this.notice) {
      parts.push(element("div", "fg-notice", this.notice));
    }

    // Only a project with no tasks at all gets the empty state. Filtering down
    // to nothing must keep the filter row on screen, or there is no way back.
    parts.push(this.data.tasks.length === 0 ? this.renderEmpty() : this.renderGrid());
    parts.push(this.renderToolbar());

    this.root.replaceChildren(...parts);

    // 貼った直後の表はいちばん上にいる。この下の `restoreFocus` は選択セルを
    // 見えるところへ持ってくるので、上にいるまま呼ぶと「見えるところ」を作るために
    // 計画のほうが動いてしまう——下のほうで行を1つ足すと、その行が画面のいちばん下に
    // 来て景色が飛ぶ、あれの正体。測る前・焦点を戻す前に、まず元の位置へ。
    const backLeft = this.root.querySelector<HTMLElement>(".fg-pane-left");
    const backChart = this.root.querySelector<HTMLElement>(".fg-pane-chart");
    if (backLeft) backLeft.scrollTop = this.scrollTop;
    if (backChart) {
      backChart.scrollTop = this.scrollTop;
      if (this.scrollLeft !== null) backChart.scrollLeft = this.scrollLeft;
    }

    this.updateFilterCount();

    // Put the caret back where it was: the filter row is rebuilt with the rest
    // of the grid, and losing it would send the next keystroke into a cell.
    if (typing) {
      const box = this.root.querySelector<HTMLInputElement>(
        `.fg-filter[data-column="${typing.key}"]`,
      );

      if (box) {
        box.focus();
        if (typing.caret !== null) box.setSelectionRange(typing.caret, typing.caret);
        return;
      }
    }

    this.restoreFocus();
  }

  /**
   * Redraws one row, both panes, instead of the whole island.
   *
   * Opening an editor changes one cell. Rebuilding every row to do it costs
   * 0.4ms a row — measured — which is nothing at ten rows and two thirds of a
   * second at two thousand. `repaintSelection` already does the same trick for the
   * cursor; this is the same idea for the row under it.
   *
   * Returns false when the row is not on screen, and the caller falls back to
   * drawing everything.
   */
  /**
   * Takes the row height and where the rows begin from the page itself.
   *
   * Both are read off a row that is actually drawn, using the index that row
   * carries — never a field on the island, which the next render moves. Read
   * the two out of step and the origin lands hundreds of pixels from the
   * truth, which draws a window nowhere near what the pane is showing: rows
   * exist, and the screen is blank.
   *
   * Returns false when there is nothing to measure.
   */
  private measureRows(pane: HTMLElement): boolean {
    const row = pane.querySelector<HTMLElement>(".fg-row.fg-data");
    const index = Number(row?.dataset["index"]);
    if (!row || !Number.isFinite(index)) return false;

    this.rowPixels = row.offsetHeight || this.rowPixels;
    this.rowsTop = row.offsetTop - index * this.rowPixels;

    return true;
  }

  /** Which rows are worth having in the document right now. */
  private rowWindow(): { first: number; last: number } {
    const total = this.tasks.length;
    const pane = this.root.querySelector<HTMLElement>(".fg-pane-left");

    // Off the page every time, rather than once at the end of a render: a row
    // is 32 pixels until somebody's own CSS says otherwise, and an origin
    // measured a render ago describes a table that is no longer there.
    if (pane) this.measureRows(pane);

    const top = pane?.scrollTop ?? this.scrollTop;
    const height = pane?.clientHeight || 800;

    // The filter row and the headings sit inside the same scrollport, so the
    // first row starts below them rather than at zero.
    const from = Math.floor((top - this.rowsTop) / this.rowPixels);
    const to = Math.ceil((top + height - this.rowsTop) / this.rowPixels);

    // Whatever the arithmetic says, a plan with rows in it draws some of them.
    // A window that starts past the last row draws nothing at all, and a table
    // showing nothing is indistinguishable from a broken one.
    const last = Math.min(total - 1, to + Grid.OVERSCAN);

    return {
      first: Math.min(Math.max(0, from - Grid.OVERSCAN), Math.max(0, last)),
      last,
    };
  }

  /** A block of nothing, holding the place of the rows that are not drawn. */
  private spacer(rows: number): HTMLElement | null {
    if (rows <= 0) return null;

    const gap = element("div", "fg-spacer");
    gap.style.height = `${rows * this.rowPixels}px`;

    return gap;
  }

  /**
   * Redraws the rows that are on screen, leaving everything else alone.
   *
   * Called on every scroll, so it does as little as it can: if the window has
   * not moved, nothing happens at all.
   */
  private renderWindow(force = false): void {
    const table = this.root.querySelector<HTMLElement>(".fg-table");
    const bars = this.root.querySelector<HTMLElement>(".fg-bars");
    const heading = this.root.querySelector<HTMLElement>(".fg-heading");
    if (!table || !bars || !heading) return;

    // Not mid-conversion: an IME hands its characters to the element it started
    // in, and replacing the rows underneath throws the conversion away.
    if (this.composing) return;

    const view = this.rowWindow();
    if (!force && view.first === this.first && view.last === this.last) return;

    // An open editor holds what somebody is typing. Scrolling used to leave
    // every row alone rather than disturb it, which meant a plan scrolled
    // while a cell was open simply stopped drawing: rows where the pane had
    // been, blank space where it now was. The editor travels instead — into
    // the rebuilt cell if its row is still drawn, and otherwise its value is
    // kept, the same as clicking on another cell keeps it.
    const editor = this.root.querySelector<HTMLInputElement>(".fg-editor:not(.is-typist)");
    const caret = editor ? { from: editor.selectionStart ?? 0, to: editor.selectionEnd ?? 0 } : null;

    if (editor && (this.row < view.first || this.row > view.last)) {
      void this.commitEdit(editor.value, "stay");
      return;
    }

    if (editor) {
      // Moving an element blurs it, and this one commits on blur. It is going
      // straight back into its cell, so that is not a person leaving the cell.
      this.moving = true;
      this.root.querySelector(".fg-grid")?.append(editor);
    }

    this.first = view.first;
    this.last = view.last;

    // The invisible field that carries the keyboard sits in the selected cell,
    // which is about to be replaced with its row. Parked on the grid until
    // there is a cell to put it back in: when it goes out of the document with
    // its row, focus goes with it and the arrow keys stop arriving.
    const typist = this.root.querySelector<HTMLElement>(".fg-editor.is-typist");
    if (typist) this.root.querySelector(".fg-grid")?.append(typist);

    // Built first and swapped in one go. Emptying the table before refilling it
    // leaves the content shorter than the scroll position for an instant, and
    // the browser answers that by scrolling back to the top — which is how a
    // keyboard walk down a long plan used to end at row 22.
    const origin = parseDate(this.data.range_start);
    const tracks = heading.style.gridTemplateColumns;

    const rows: HTMLElement[] = [];
    const drawnBars: HTMLElement[] = [];

    const above = this.spacer(view.first);
    if (above) {
      rows.push(above);
      drawnBars.push(above.cloneNode() as HTMLElement);
    }

    for (let index = view.first; index <= view.last; index++) {
      const task = this.tasks[index];
      if (!task) continue;

      const row = this.renderRow(task, index);
      row.style.gridTemplateColumns = tracks;
      rows.push(row);
      drawnBars.push(this.renderBar(task, origin, index));
    }

    const below = this.spacer(this.tasks.length - 1 - view.last);
    if (below) {
      rows.push(below);
      drawnBars.push(below.cloneNode() as HTMLElement);
    }

    // Everything in these two that is not a row keeps its place: the filter row
    // and the headings above, the day columns behind and today's line in front.
    const keptAbove = [...table.children].filter(
      (child) => !child.classList.contains("fg-data") && !child.classList.contains("fg-spacer"),
    );
    const columns = bars.querySelector(".fg-columns");
    const today = bars.querySelector(".fg-today");

    table.replaceChildren(...keptAbove, ...rows);
    bars.replaceChildren(
      ...(columns ? [columns] : []),
      ...drawnBars,
      ...(today ? [today] : []),
    );

    this.pinColumns();
    // The rows that carried the cursor have just been replaced.
    if (this.row >= view.first && this.row <= view.last) this.markSelection();

    // The editor goes back where it was being typed into, caret and all.
    if (editor) {
      const cell = this.root
        .querySelectorAll<HTMLElement>(".fg-pane-left .fg-row.fg-data")
        [this.row - view.first]?.children[this.column];

      if (cell) {
        cell.querySelector(".fg-editor.is-typist")?.remove();
        cell.append(editor);
        editor.focus({ preventScroll: true });
        if (caret) editor.setSelectionRange(caret.from, caret.to);
      }

      this.moving = false;
    }

    // Parking was only somewhere to stand while the row was replaced, and the
    // rebuilt selected cell comes with a field of its own. Left behind, the
    // parked one covers the island — it is `inset: 0` and the grid is the
    // nearest positioned ancestor — so the wheel lands on an invisible input
    // instead of the pane, and a plan too long to fit stops scrolling.
    const parked = this.root.querySelector(".fg-grid > .fg-editor.is-typist");
    if (parked && this.root.querySelectorAll(".fg-editor.is-typist").length > 1) {
      parked.remove();
    }
  }

  /**
   * Brings a row into the document, and into view, before anything looks for it.
   *
   * Moving the cursor with the keyboard can land on a row that was never drawn.
   */
  private reachRow(index: number): void {
    if (index >= this.first && index <= this.last) return;

    const pane = this.root.querySelector<HTMLElement>(".fg-pane-left");
    if (pane) {
      const wanted = this.rowsTop + index * this.rowPixels;
      const height = pane.clientHeight || 800;

      if (wanted < pane.scrollTop + this.rowsTop) pane.scrollTop = wanted - this.rowsTop;
      else if (wanted > pane.scrollTop + height - this.rowPixels * 2) {
        pane.scrollTop = wanted - height + this.rowPixels * 2;
      }
    }

    this.renderWindow();
  }

  private repaintRow(index: number): boolean {
    const grid = this.root.querySelector<HTMLElement>(".fg-grid");
    const task = this.tasks[index];
    if (!grid || !task) return false;

    // A row nobody can see needs no drawing, and that counts as done.
    if (index < this.first || index > this.last) return true;

    const rows = grid.querySelectorAll<HTMLElement>(".fg-pane-left .fg-row.fg-data");
    const bars = grid.querySelectorAll<HTMLElement>(".fg-bar-row");
    const was = rows[index - this.first];
    const wasBar = bars[index - this.first];
    if (!was || !wasBar) return false;

    const row = this.renderRow(task, index);
    // The track list is measured and set once for the whole table; the new row
    // takes the one already on screen rather than working it out again.
    row.style.gridTemplateColumns = was.style.gridTemplateColumns;

    was.replaceWith(row);
    wasBar.replaceWith(this.renderBar(task, parseDate(this.data.range_start), index));
    this.pinRow(row);

    return true;
  }

  /** Parks one row's frozen cells where the columns before them end. */
  private pinRow(row: HTMLElement): void {
    const left = this.root.querySelector<HTMLElement>(".fg-pane-left");
    if (!left) return;

    const heads = [...left.querySelectorAll<HTMLElement>(".fg-heading .fg-cell")];
    let offset = 0;

    for (let i = 0; i < this.data.frozen_columns && i < heads.length; i++) {
      const cell = row.children[i];
      if (cell instanceof HTMLElement) cell.style.left = `${offset}px`;
      offset += heads[i]!.getBoundingClientRect().width;
    }
  }

  /**
   * Everything the grid draws apart from the rows' own values.
   *
   * Two of these being equal means the table has the same shape it had a
   * moment ago — same columns, same rows in the same order, same dates across
   * the top — and only the contents of some rows can have moved.
   */
  private shape(): string {
    return [
      this.data.range_start,
      this.data.range_end,
      this.data.frozen_columns,
      this.data.day_width,
      this.data.column_order.join(" "),
      this.data.hidden_columns.join(" "),
      this.data.tooltip_columns.join(" "),
      this.data.fields.map((field) => field.id).join(" "),
      this.data.statuses.map((status) => status.name).join(" "),
      this.tasks.map((task) => task.id).join(" "),
    ].join("|");
  }

  /**
   * Draws the difference between the table on screen and the one in hand.
   *
   * A cell edit comes back as the whole grid, because one value can move every
   * ancestor's dates — but almost always only a few rows actually changed, and
   * the rest of the table is already correct on screen.
   */
  private repaintChanged(was: string, before: string[]): void {
    const showing = this.root.querySelector(".fg-error") !== null;

    // Anything that changes the furniture goes back to drawing it all: the
    // banner, the notice, a column appearing, rows arriving or leaving.
    if (was !== this.shape() || showing !== (this.error !== null)) {
      this.render();
      return;
    }

    this.paintNotice();

    const now = this.tasks;
    for (let index = 0; index < now.length; index++) {
      if (before[index] !== JSON.stringify(now[index])) {
        if (!this.repaintRow(index)) {
          this.render();
          return;
        }
      }
    }

    // The toolbar reflects whether a request is in flight, and the count above
    // the table reflects the filters. Both are cheap and neither is a row.
    this.root.querySelector(".fg-toolbar")?.replaceWith(this.renderToolbar());
    this.updateFilterCount();
    this.restoreFocus();
  }

  private renderEmpty(): HTMLElement {
    const empty = element("div", "fg-empty");
    empty.append(element("p", undefined, t("タスクがありません。")));

    if (this.data.can_edit) {
      const add = element("button", "fg-button", t("最初のタスクを追加"));
      add.type = "button";
      add.addEventListener("click", () => void this.insertRow());
      empty.append(add);
    }

    return empty;
  }

  private renderToolbar(): HTMLElement {
    const bar = element("div", "fg-toolbar");


    if (!this.data.can_edit) {
      bar.append(element("span", "fg-hint", t("閲覧のみ")));
      return bar;
    }

    const add = element("button", "fg-button", t("行を追加"));
    add.type = "button";
    add.disabled = this.busy;
    add.addEventListener("click", () => void this.insertRow());

    const remove = element("button", "fg-button fg-button-quiet", t("行を削除"));
    remove.type = "button";
    remove.disabled = this.busy || this.tasks.length === 0;
    remove.addEventListener("click", () => void this.deleteRow());

    const leaves = element("button", "fg-button", t("担当者の休暇/出社"));
    leaves.type = "button";
    leaves.title = t("誰がいつ休み、いつ出るか。日数の数え方に効きます");
    leaves.addEventListener("click", () => this.openLeaves());

    bar.append(add, remove, leaves);

    return bar;
  }

  private renderGrid(): HTMLElement {
    const origin = parseDate(this.data.range_start);
    const days = Math.max(1, dayIndex(this.data.range_end, origin) + 1);

    const left = element("div", "fg-pane-left");
    // One box around every row, so they all take their width from the same
    // place. Sized to its widest row, it is what lets a pinned column travel
    // the full scroll — and what stops each row from sizing its own columns.
    const table = element("div", "fg-table");
    left.append(table);

    // The columns are data now, so the track list has to be too: a fixed one
    // sends the extra columns onto a second, implicit row.
    const tracks = this.columns
      .map((column) => {
        const width = this.data.column_widths[column.key];
        return width ? `${width}px` : TRACKS[column.kind];
      })
      .join(" ");

    const headings = element("div", "fg-row fg-heading");
    headings.style.gridTemplateColumns = tracks;
    this.columns.forEach((column, index) => {
      // Headings are translated as they are drawn. BASE_COLUMNS is built once at
      // load, so translating there would freeze the wording before the language
      // is known. A project's own field names are the users' words: not in the
      // dictionary, and shown as they are.
      const heading = element("div", `fg-cell fg-cell-${column.key}`, t(column.label));
      if (this.workdayBased && (column.kind === "days" || column.kind === "variance")) {
        heading.title = t("土日・祝日を除いた営業日で数えています");
      }
      if (index < this.data.frozen_columns) heading.classList.add("is-frozen");
      headings.append(heading);
    });
    // Filters above the headings, so the labels sit directly over the data.
    table.append(this.renderFilterRow(tracks), headings);

    const chart = element("div", "fg-pane-chart");
    const canvas = element("div", "fg-canvas");
    canvas.style.width = `${days * this.dayWidth}px`;
    canvas.append(this.renderHeader(origin, days));

    const body = element("div", "fg-bars");

    // Day columns sit behind the bars: the week rhythm is what makes a chart
    // readable, and the bars are positioned elements so they paint over this.
    const columns = element("div", "fg-columns");
    for (let i = 0; i < days; i++) {
      const date = new Date(origin + i * DAY_MS);
      const iso = date.toISOString().slice(0, 10);
      const holiday = this.holidayOn(iso);
      const column = element("div", "fg-column");

      const note = this.dayNote(iso);
      if (note) column.title = note;

      if (holiday) {
        column.classList.add("is-holiday");
      } else if (date.getUTCDay() === 6) {
        column.classList.add("is-saturday");
      } else if (date.getUTCDay() === 0) {
        column.classList.add("is-sunday");
      }

      columns.append(column);
    }
    body.append(columns);

    if (this.tasks.length === 0) {
      table.append(element("div", "fg-nomatch", t("条件に合う行がありません。")));
    }

    // Only the rows that can be seen. `renderWindow` keeps this up to date as
    // the pane scrolls; here it is just the first cut.
    const view = this.rowWindow();
    this.first = view.first;
    this.last = view.last;

    const above = this.spacer(view.first);
    if (above) {
      table.append(above);
      body.append(above.cloneNode() as HTMLElement);
    }

    this.tasks.slice(view.first, view.last + 1).forEach((task, offset) => {
      const index = view.first + offset;
      const row = this.renderRow(task, index);
      row.style.gridTemplateColumns = tracks;
      table.append(row);
      body.append(this.renderBar(task, origin, index));
    });

    const todayIndex = dayIndex(this.data.today, origin);
    const below = this.spacer(this.tasks.length - 1 - view.last);
    if (below) {
      table.append(below);
      body.append(below.cloneNode() as HTMLElement);
    }

    if (todayIndex >= 0 && todayIndex < days) {
      const marker = element("div", "fg-today");
      marker.style.left = `${todayIndex * this.dayWidth}px`;
      body.append(marker);
    }

    canvas.append(body);
    chart.append(canvas);

    const grid = element("div", "fg-grid");
    grid.tabIndex = 0;
    this.syncPanes(left, chart);
    if (this.paneWidth) {
      grid.style.setProperty("--fg-pane-width", `${this.paneWidth}px`);
    } else {
      // Untouched, the table takes whatever its columns add up to, which with
      // every column on leaves the chart a sliver. Half the window is the most
      // it gets before somebody drags the splitter themselves.
      this.capPaneWidth = true;
    }

    // The palette is per project, so it arrives with the data rather than
    // living in the stylesheet.
    grid.style.setProperty("--fg-bar-soft", this.data.theme.bar);
    grid.style.setProperty("--fg-bar", this.data.theme.done);
    grid.style.setProperty("--fg-actual", this.data.theme.actual);
    grid.style.setProperty("--fg-today", this.data.theme.today);
    grid.style.setProperty("--fg-summary", this.data.theme.summary);
    grid.style.setProperty("--fg-late", this.data.theme.late);
    grid.style.setProperty("--fg-saturday", this.data.theme.saturday);
    grid.style.setProperty("--fg-sunday", this.data.theme.sunday);
    grid.style.setProperty("--fg-holiday", this.data.theme.holiday);
    grid.style.setProperty("--fg-leave", this.data.theme.leave);
    grid.style.setProperty("--fg-wait", this.data.theme.wait);

    grid.append(left, this.renderSplitter(grid), chart);

    // A schedule opened at the far left rarely shows the part anyone cares
    // about, so bring today into view the first time.
    const target =
      this.scrollLeft ?? (todayIndex >= 0 ? Math.max(0, (todayIndex - 5) * this.dayWidth) : 0);
    requestAnimationFrame(() => {
      chart.scrollLeft = target;

      // Put the page back where it was, and measure what the rows turned out
      // to be: the window is worked out in pixels, and guessing them wrong
      // draws the wrong rows.
      left.scrollTop = this.scrollTop;
      chart.scrollTop = this.scrollTop;

      // Another render may have landed in the meantime — a keystroke and the
      // answer to the one before it easily share a frame. What this callback
      // measures is then a table that is no longer on the page, where every
      // offset reads zero, and `rowsTop` comes out thousands of pixels
      // negative. From there the window is worked out past the end of the
      // plan, nothing is drawn, and the grid is blank for good: the way it
      // used to give out at around a hundred rows of holding down ⌘Enter.
      if (!left.isConnected) return;

      if (this.measureRows(left)) this.renderWindow();

      // Measured rather than guessed: the columns are sized by their content,
      // so their total is only knowable once they are on the page.
      if (this.capPaneWidth) {
        // The chart is the point of the app, so it keeps a usable strip no
        // matter how many columns are on. Anything past that scrolls.
        const cap = Math.max(320, grid.clientWidth - 480);

        // Opened at its full width, the table pushes the chart into a strip and
        // the screen reads as a spreadsheet with a margin. The dates are what
        // the chart is drawn from, so the split starts after 予定終了 and the
        // rest of the columns are a scroll away for whoever wants them.
        const dates = left.querySelector<HTMLElement>(".fg-heading .fg-cell-end");
        const wanted = dates ? dates.offsetLeft + dates.offsetWidth : left.scrollWidth;

        grid.style.setProperty(
          "--fg-pane-width",
          `${Math.max(320, Math.min(wanted, cap, left.scrollWidth))}px`,
        );
      }

      // A frame later: the pane width above changes every track, and pinning
      // to widths measured before it lands leaves the columns overlapping.
      requestAnimationFrame(() => this.pinColumns());
    });

    return grid;
  }

  /**
   * Parks each frozen column where the ones before it end.
   *
   * The offsets are measured rather than computed from the track list: the
   * tracks are relative units, so their pixels only exist once the grid is on
   * the page — and they move again whenever the pane does.
   */
  private pinColumns(): void {
    const left = this.root.querySelector<HTMLElement>(".fg-pane-left");
    if (!left) return;

    const heads = [...left.querySelectorAll<HTMLElement>(".fg-heading .fg-cell")];
    let offset = 0;

    for (let i = 0; i < this.data.frozen_columns && i < heads.length; i++) {
      for (const cell of left.querySelectorAll<HTMLElement>(`.fg-row > :nth-child(${i + 1})`)) {
        cell.style.left = `${offset}px`;
      }
      offset += heads[i]!.getBoundingClientRect().width;
    }
  }

  private renderRow(task: Task, index: number): HTMLElement {
    // `fg-data` marks the rows that hold tasks: the heading and the filter row
    // are also `.fg-row`, and picking them up shifts every index by one.
    const row = element("div", "fg-row fg-data");
    // Which row of the plan this is. The window is worked out in pixels, and
    // the sum only comes out right if the row being measured says where it
    // belongs — a field on the island can be a render out of date by the time
    // anything is measured.
    row.dataset["index"] = String(index);
    if (this.behind(task)) row.classList.add("is-delayed");
    if (index === this.row) row.classList.add("is-current");

    // The row's own colours, if somebody gave it any. Set as a custom property
    // rather than on the row: the cells paint their own backgrounds (selection,
    // hatching, the frozen columns' opaque ground), so each of them has to be
    // able to pick this up rather than sit on top of it.
    if (task.background) row.style.setProperty("--fg-row-bg", task.background);
    if (task.color) row.style.setProperty("--fg-row-color", task.color);
    if (task.background || task.color) row.classList.add("is-painted");

    this.columns.forEach((column, columnIndex) => {
      const cell = element("div", `fg-cell fg-cell-${column.key}`);
      const isSelected = index === this.row && columnIndex === this.column;

      if (isSelected) cell.classList.add("is-selected");

      // Hatching means "this row's value comes from its children", not merely
      // "read-only" — otherwise the always-derived day count would stripe every
      // row and the signal would stop meaning anything.
      if (task.has_children && ROLLED_UP.includes(column.key)) {
        cell.classList.add("is-derived");
      }

      // The indent belongs to the cell, not to its contents, so the editor
      // opens exactly where the text was.
      if (column.kind === "name") {
        cell.style.paddingLeft = `${12 + task.depth * 16}px`;
        if (task.has_children) cell.classList.add("is-summary");
      }

      // The twisty stays put while the name is being edited, so the text does
      // not slide sideways when the editor opens.
      if (column.kind === "name") {
        if (this.data.can_edit) cell.append(this.renderHandle(task, index));
        cell.append(this.renderTwisty(task));
      }

      if (isSelected && this.editing) {
        cell.classList.add("is-editing");
        cell.append(this.renderEditor(task, column));
        if (column.kind === "date") cell.append(this.renderDatePicker());
      } else if (column.key === "late") {
        // Before the kind-based branches: this column is a select so that its
        // filter offers 遅延 and 順調, and the select branch would otherwise
        // print 順調 on every quiet row — a column of noise.
        if (this.behind(task)) {
          const mark = element("span", "fg-late-mark", t("遅延"));
          mark.title = task.delayed
            ? t("予定進捗に届いていません")
            : t("予定終了を過ぎて、実施終了が入っていません");
          cell.append(mark);
        }
      } else if (column.kind === "name") {
        const text = element("span", "fg-name-text", task.name || t("（無題）"));
        if (!task.name) text.classList.add("is-placeholder");
        cell.append(text);

        if (task.has_children && this.collapsed.has(task.id)) {
          cell.append(element("span", "fg-folded", `+${this.hiddenCount(task)}`));
        }

        for (const tag of task.tags) cell.append(element("span", "fg-tag", tag));
      } else if (column.kind === "status") {
        if (task.status) {
          const pill = element("span", "fg-status", task.status);
          // The colour comes with the state rather than from a rule per name:
          // the names are the project's to choose.
          const colour = this.data.statuses.find((status) => status.name === task.status)?.color;
          if (colour) pill.style.background = colour;
          cell.append(pill);
        }
      } else if (column.kind === "select") {
        // A master list is a set of states as much as the status column is, so
        // an entry that was given colours is drawn with them.
        const value = this.cellText(task, column);
        const option = column.options?.find((entry) => entry.value === value);

        if (value && (option?.color || option?.background)) {
          const pill = element("span", "fg-status", value);
          if (option.color) pill.style.color = option.color;
          if (option.background) pill.style.background = option.background;
          cell.append(pill);
        } else if (value) {
          cell.append(element("span", undefined, value));
        }
      } else if (column.key === "targets") {
        if (this.editable(task, column)) {
          const open = element("button", "fg-wait-edit", task.targets.length === 0 ? "＋" : "✎");
          open.type = "button";
          open.title = t("予定進捗を登録する");
          open.addEventListener("mousedown", (event) => event.stopPropagation());
          open.addEventListener("click", () => {
            this.select(index, columnIndex);
            this.openTargets(task);
          });
          cell.append(open);
        }

        // A checkpoint that has passed and was not met is the one thing on this
        // row worth a colour. The rest are just what the plan says.
        for (const target of task.targets) {
          const pill = element("span", "fg-target-pill", `${short(target.date)} ${target.percent}%`);
          if (target.missed) pill.classList.add("is-missed");
          else if (target.due) pill.classList.add("is-met");
          pill.title = target.missed
            ? t("この日までに届いていません")
            : target.due
              ? t("達成")
              : t("これから");
          cell.append(pill);
        }
      } else if (column.key === "waits") {
        // The button comes first: the cell clips what runs past its width, and
        // a row with two waits in it would push the way in off the edge.
        if (this.editable(task, column)) {
          const open = element("button", "fg-wait-edit", task.waits.length === 0 ? "＋" : "✎");
          open.type = "button";
          open.title = t("待ちの期間を登録する");
          open.addEventListener("mousedown", (event) => event.stopPropagation());
          open.addEventListener("click", () => {
            this.select(index, columnIndex);
            this.openWaits(task);
          });
          cell.append(open);
        }

        // A wait is a state: stopped. Without a colour it reads as quiet, having
        // none of the red that lateness gets. Ones still open read darker, with a
        // trailing 〜.
        for (const wait of task.waits) {
          const label = wait.open
            ? `${short(wait.start)}〜`
            : `${short(wait.start)}〜${short(wait.end)}`;

          const pill = element("span", "fg-wait-pill", label);
          if (wait.open) pill.classList.add("is-open");
          // Entered, but outside the task's own dates: it counts for nothing,
          // and saying so is better than looking like it worked.
          if (wait.days === 0 && task.start && task.end) pill.classList.add("is-idle");
          if (wait.reason) {
            pill.append(element("span", "fg-wait-why", wait.reason));
          }
          pill.title = [
            wait.reason || t("待ち"),
            wait.open ? t("（継続中）") : "",
            wait.days === 0 && task.start && task.end ? t("予定の期間の外なので日数には効きません") : "",
          ]
            .filter(Boolean)
            .join(" ");
          cell.append(pill);
        }

      } else if (column.kind === "variance") {
        const span = element("span", undefined, this.cellDisplay(task, column));
        if (task.has_children) {
          cell.title = t("子タスクのずれを足したものです（この行の日付の差ではありません）");
        }
        const days = column.key === "start_variance" ? task.start_variance : task.end_variance;
        if (days !== null && days > 0) span.classList.add("is-late");
        if (days !== null && days < 0) span.classList.add("is-early");
        cell.append(span);
      } else {
        cell.append(element("span", undefined, this.cellDisplay(task, column)));
      }

      // The selected cell always carries a real text field, even when it looks
      // like plain text. An IME has no way to start composing into a focused
      // `div`, so without this, typing Japanese into a cell does nothing at all.
      if (isSelected && !this.editing) cell.append(this.renderTypist());

      if (columnIndex < this.data.frozen_columns) cell.classList.add("is-frozen");

      cell.addEventListener("mousedown", (event) => {
        if (this.editing) return;
        event.preventDefault();

        // The browser never raises `dblclick` here: the first click rebuilds
        // the cell, so the two clicks land on different elements and Chrome
        // has nothing to raise it on. Timing the pair is what works.
        const now = Date.now();
        const again =
          this.lastPress?.row === index &&
          this.lastPress.column === columnIndex &&
          now - this.lastPress.at < 400;

        this.lastPress = { row: index, column: columnIndex, at: now };

        this.select(index, columnIndex);
        this.repaintSelection();

        // Same as Enter: the reason to point at a cell is usually to change it.
        if (again) this.startEdit(null);
      });

      cell.addEventListener("contextmenu", (event) => {
        event.preventDefault();
        this.select(index, columnIndex);
        this.repaintSelection();
        this.openMenu(event.clientX, event.clientY);
      });

      row.append(cell);
    });

    return row;
  }

  /**
   * The grip that starts a row drag.
   *
   * Dragging from the cell itself would fight text selection and the click that
   * moves the cursor, so the gesture gets its own small target.
   */
  private renderHandle(task: Task, index: number): HTMLElement {
    // Drawn in CSS, with no text of its own: a glyph here would end up in the
    // row's text content, and so in anything copied or read aloud.
    const handle = element("span", "fg-handle");
    handle.title = t("ドラッグで移動");
    handle.setAttribute("aria-hidden", "true");
    handle.addEventListener("pointerdown", (event) => this.beginRowDrag(event, task, index));
    return handle;
  }

  /**
   * Drags a row to a new place in the outline.
   *
   * Vertical movement picks the gap to drop into; horizontal movement picks the
   * depth, clamped to what the surrounding rows allow. Showing both as a line
   * before the drop is the whole point — the keyboard moves are faster but give
   * no preview, which is what made them hard to trust.
   */
  private beginRowDrag(event: PointerEvent, task: Task, index: number): void {
    if (event.button !== 0 || !this.data.can_edit) return;

    event.preventDefault();
    event.stopPropagation();

    const grid = this.root.querySelector<HTMLElement>(".fg-grid");
    const rows = [
      ...this.root.querySelectorAll<HTMLElement>(".fg-pane-left .fg-row.fg-data"),
    ];
    if (!grid) return;

    // The dragged row takes its subtree along, so neither it nor its
    // descendants are candidates for the drop.
    const subtree = this.subtreeLength(index);
    const excluded = new Set<number>();
    for (let i = index; i < index + subtree; i++) excluded.add(i);

    const indicator = element("div", "fg-drop");
    grid.append(indicator);
    // The rows on screen are a window onto the list, so a task's number and its
    // place in the document are two different things from here on.
    rows[index - this.first]?.classList.add("is-dragging");

    const startX = event.clientX;
    let target: { at: number; depth: number } | null = null;

    const preview = (move: PointerEvent) => {
      // The gap the pointer is nearest to, as a task number.
      let at = this.first + rows.length;
      for (const [i, row] of rows.entries()) {
        const box = row.getBoundingClientRect();
        if (move.clientY < box.top + box.height / 2) {
          at = this.first + i;
          break;
        }
      }

      // Landing anywhere inside the dragged subtree means "stay put".
      while (excluded.has(at) && at < this.tasks.length) at++;

      const previous = this.tasks[at - 1];
      const next = this.tasks[at];

      // Deeper than the row above plus one would have no parent; shallower than
      // the row below would orphan it.
      const maxDepth = previous ? previous.depth + 1 : 0;
      const minDepth = next && !excluded.has(at) ? next.depth : 0;
      const wanted = task.depth + Math.round((move.clientX - startX) / 16);
      const depth = clamp(wanted, Math.min(minDepth, maxDepth), maxDepth);

      target = { at, depth };

      const edge = rows[at - this.first] ?? rows[rows.length - 1];
      if (!edge) return;

      const box = edge.getBoundingClientRect();
      const gridBox = grid.getBoundingClientRect();

      indicator.style.top = `${
        (at < this.first + rows.length ? box.top : box.bottom) - gridBox.top
      }px`;
      indicator.style.left = `${12 + depth * 16}px`;
    };

    const finish = async () => {
      detach();
      indicator.remove();
      rows[index - this.first]?.classList.remove("is-dragging");

      if (!target) return;

      const drop = this.dropTarget(target.at, target.depth, excluded);
      if (drop.parent === (task.id as string | null)) return;

      const was = this.spotOf(task.id);

      await this.send(
        `/api/projects/${encodeURIComponent(this.projectId)}/tasks/${task.id}/place`,
        { method: "POST", body: drop, follow: task.id, was: was ?? undefined },
      );
    };

    const cancel = () => {
      detach();
      indicator.remove();
      rows[index - this.first]?.classList.remove("is-dragging");
    };

    function detach(): void {
      window.removeEventListener("pointermove", preview);
      window.removeEventListener("pointerup", finish);
      window.removeEventListener("pointercancel", cancel);
    }

    // On the window, not the grip: anything that re-renders mid-drag — an SSE
    // update from someone else, say — would detach the grip and the gesture
    // would die without a sound.
    window.addEventListener("pointermove", preview);
    window.addEventListener("pointerup", finish);
    window.addEventListener("pointercancel", cancel);
  }

  /**
   * Drags the boundary between the done and planned parts of a bar.
   *
   * Percent is what the column holds, so the bar's width is the scale: the
   * pointer's position along it is the number.
   */
  private beginProgressDrag(
    event: PointerEvent,
    task: Task,
    bar: HTMLElement,
    box: DOMRect,
    index: number,
  ): void {
    event.preventDefault();
    event.stopPropagation();

    this.select(index, 0);
    this.repaintSelection();

    const fill = bar.querySelector<HTMLElement>(".fg-bar-fill");
    let progress = task.progress;

    bar.classList.add("is-dragging");

    const preview = (move: PointerEvent) => {
      progress = clamp(Math.round(((move.clientX - box.left) / box.width) * 100), 0, 100);
      if (fill) fill.style.width = `${progress}%`;
      bar.title = `${task.name} — ${progress}%`;
    };

    const finish = async () => {
      detach();
      bar.classList.remove("is-dragging");

      if (progress === task.progress) {
        this.render();
        return;
      }

      const rollback = structuredClone(this.data);
      task.progress = progress;
      this.render();

      await this.send(`/api/projects/${encodeURIComponent(this.projectId)}/tasks/${task.id}`, {
        method: "POST",
        body: { field: "progress", value: String(progress) },
        rollback,
        follow: task.id,
      });
    };

    const cancel = () => {
      detach();
      bar.classList.remove("is-dragging");
      this.render();
    };

    function detach(): void {
      window.removeEventListener("pointermove", preview);
      window.removeEventListener("pointerup", finish);
      window.removeEventListener("pointercancel", cancel);
    }

    window.addEventListener("pointermove", preview);
    window.addEventListener("pointerup", finish);
    window.addEventListener("pointercancel", cancel);
  }

  /** How many visible rows the row at `index` owns, itself included. */
  private subtreeLength(index: number): number {
    const depth = this.tasks[index]?.depth ?? 0;
    let length = 1;

    while (this.tasks[index + length] && (this.tasks[index + length]?.depth ?? 0) > depth) {
      length++;
    }

    return length;
  }

  /**
   * Turns "this gap, at this depth" into the parent and preceding sibling the
   * server needs.
   */
  private dropTarget(
    at: number,
    depth: number,
    excluded: Set<number>,
  ): { parent: string | null; after: string | null } {
    let parent: string | null = null;
    let after: string | null = null;

    for (let i = at - 1; i >= 0; i--) {
      if (excluded.has(i)) continue;

      const row = this.tasks[i];
      if (!row) continue;

      if (after === null && row.depth === depth) after = row.id;

      if (row.depth < depth) {
        parent = row.id;
        break;
      }
    }

    return { parent, after };
  }

  /**
   * The fold control. Leaf rows get an empty one of the same width so names
   * stay on a single left edge within a level.
   */
  private renderTwisty(task: Task): HTMLElement {
    if (!task.has_children) return element("span", "fg-twisty is-leaf");

    const folded = this.collapsed.has(task.id);
    const button = element("button", "fg-twisty");
    button.type = "button";
    button.textContent = folded ? "▶" : "▼";
    button.title = folded ? t("展開する") : t("折りたたむ");
    button.setAttribute("aria-expanded", folded ? "false" : "true");
    button.tabIndex = -1;

    // mousedown, so the cell's own handler does not select and re-render first.
    button.addEventListener("mousedown", (event) => {
      event.preventDefault();
      event.stopPropagation();
      this.toggleCollapse(task);
    });

    return button;
  }

  /**
   * The invisible field that holds the caret while a cell is merely selected.
   *
   * Typing turns it into the editor in place — the same element, so an IME
   * composition already in flight is never interrupted. Recreating the input at
   * that moment would drop the characters being converted.
   */
  private renderTypist(): HTMLInputElement {
    const input = element("input", "fg-editor is-typist");
    input.type = "text";
    input.value = "";
    input.autocomplete = "off";
    input.setAttribute("aria-label", t("セルの入力"));

    input.addEventListener("compositionstart", () => {
      this.composing = true;
      this.beginTyping(input);
    });
    input.addEventListener("compositionend", () => {
      this.composing = false;
    });
    input.addEventListener("input", () => {
      if (!this.editing) this.beginTyping(input);
    });
    input.addEventListener("blur", () => {
      // Only commit if this field became the editor. F2 opens a fresh editor
      // and re-renders, which blurs this one out of existence — committing its
      // empty value there would wipe the cell being opened.
      if (this.editing && !this.moving && !input.classList.contains("is-typist")) {
        void this.commitEdit(input.value, "stay");
      }
    });

    return input;
  }

  /**
   * Turns the typist into the editor without re-rendering.
   *
   * Typing replaces a cell's contents the way it does in a spreadsheet, so the
   * field starting empty is already the right value — nothing to transfer.
   */
  private beginTyping(input: HTMLInputElement): void {
    const task = this.selected;
    const column = this.selectedColumn;

    if (!task || !this.editable(task, column)) {
      input.value = "";
      this.startEdit(null);
      return;
    }

    // A closed set of values is a menu; typing at it opens the menu instead.
    if (column.kind === "status" || column.kind === "select") {
      input.value = "";
      this.startEdit(null);
      return;
    }

    this.editing = true;
    this.seed = null;
    input.classList.remove("is-typist");
    input.closest(".fg-cell")?.classList.add("is-editing");
  }

  /**
   * The calendar beside a date editor.
   *
   * A real `type="date"` field would bring its own, but at the cost of typing
   * the date straight through, so the picker gets its own hidden field.
   */
  private renderDatePicker(): HTMLElement {
    const picker = element("input", "fg-datepicker");
    picker.type = "date";
    picker.tabIndex = -1;
    picker.title = t("カレンダーから選ぶ");

    picker.addEventListener("mousedown", (event) => event.stopPropagation());

    // Chrome only opens the calendar from its own indicator, and the click that
    // reaches it has already blurred the editor beside it — which used to
    // commit, re-render, and take this element away before the picker opened.
    picker.addEventListener("click", () => {
      try {
        picker.showPicker();
      } catch {
        // Older browsers open it from the indicator on their own.
      }
    });
    picker.addEventListener("change", () => {
      const editor = picker
        .closest(".fg-cell")
        ?.querySelector<HTMLInputElement>("input.fg-editor");

      if (editor) editor.value = picker.value;
      void this.commitEdit(picker.value, "stay");
    });

    return picker;
  }

  private renderEditor(task: Task, column: ColumnDef): HTMLElement {
    // A closed set of values is a menu, not a text field: typing a status by
    // hand is slower and can be wrong.
    const choices = this.choicesFor(column);

    if (choices) {
      const select = element("select", "fg-editor");
      // A blank entry is how a select value gets cleared.
      if (column.kind !== "status") select.append(element("option", undefined, ""));

      for (const choice of choices) {
        const option = element("option", undefined, choice);
        option.value = choice;
        select.append(option);
      }
      select.value = this.cellText(task, column);

      // Focus alone leaves the list closed, which reads as "no choices here".
      requestAnimationFrame(() => {
        try {
          select.showPicker();
        } catch {
          // Older browsers just leave it closed; the arrow keys still work.
        }
      });

      // Choosing from the menu is the whole interaction; commit on change.
      select.addEventListener("change", () => void this.commitEdit(select.value, "stay"));
      select.addEventListener("blur", () => {
        if (this.editing) void this.commitEdit(select.value, "stay");
      });

      return select;
    }

    const input = element("input", "fg-editor");
    input.type = "text";
    // Opened on what the cell was showing. The editor takes any of the usual
    // spellings back — 2026/08/05, 20260805, 8/5 — and stores the one form.
    const showing = this.cellText(task, column);
    input.value = this.seed ?? (column.kind === "date" && showing ? fullDate(showing) : showing);

    // Same idea for the project's own free-text-with-choices columns.
    if (column.kind === "suggest" && column.options?.length) {
      const list = element("datalist");
      list.id = `fg-list-${column.key}`;
      for (const choice of column.options) {
        const option = element("option") as HTMLOptionElement;
        option.value = choice.value;
        list.append(option);
      }
      input.setAttribute("list", list.id);
      this.root.querySelector(".fg-grid")?.append(list);
    }

    // The syntax is the whole interface here, so the field says it.
    if (column.key === "waits") {
      input.placeholder = t("8/17〜8/21 他部署（終わり省略で継続中）");
    }

    if (column.kind === "date") {
      // Deliberately not `type="date"`: that field takes input segment by
      // segment, so "2026-08-03" can no longer be typed straight through, and
      // it swallows Escape. The calendar comes from the button beside it.
      input.placeholder = "20260805 / 8-5";
      input.inputMode = "numeric";

      // Eight digits become a date as they are typed, so the field reads the
      // way it will be stored before anything is committed.
      input.addEventListener("input", () => {
        const digits = normalizeWidth(input.value).trim();
        if (!/^\d{8}$/.test(digits)) return;

        const iso = flexibleDate(digits);
        if (iso) {
          input.value = iso;
          input.setSelectionRange(iso.length, iso.length);
        }
      });
      // The calendar button sits inside the right edge; without room for it the
      // date runs underneath and the cell looks broken.
      input.classList.add("has-picker");
    } else if (column.kind === "progress" || column.kind === "number") {
      input.inputMode = "numeric";
    }

    // Clicking away is a commit, the same as leaving a cell in a spreadsheet —
    // except when what was clicked is this cell's own calendar button, which
    // exists to fill this very editor.
    input.addEventListener("blur", (event) => {
      const next = (event as FocusEvent).relatedTarget as HTMLElement | null;
      if (next?.classList.contains("fg-datepicker")) return;

      if (this.editing) void this.commitEdit(input.value, "stay");
    });

    return input;
  }

  /**
   * Chooses what is drawn over the chart.
   *
   * The variances and the days worked are all there because somebody needs them,
   * and all of them at once fills the chart with lines and numbers. What is worth
   * seeing changes with the person and the day, so it is switched on the screen
   * rather than in the settings.
   */
  private renderShowToggle(): HTMLElement {
    const button = element("button", "fg-shows", t("表示"));
    button.type = "button";
    button.tabIndex = -1;
    button.title = t("チャートに出すものを選びます");

    const choices: [keyof Shows, string][] = [
      ["start", "開始差異"],
      ["end", "終了差異"],
      ["worked", "実作業日数"],
      ["targets", "予定進捗"],
    ];

    if (choices.some(([key]) => !this.shows[key])) button.classList.add("is-on");

    button.addEventListener("mousedown", (event) => event.preventDefault());
    button.addEventListener("click", () => {
      const anchor = button.getBoundingClientRect();
      const menu = element("div", "fg-menu fg-shows-menu");
      menu.style.left = `${anchor.left}px`;
      menu.style.top = `${anchor.bottom + 2}px`;

      const close = (event?: Event) => {
        if (event && menu.contains(event.target as Node)) return;

        menu.remove();
        document.removeEventListener("mousedown", close);
        document.removeEventListener("keydown", onEscape);
      };

      const onEscape = (event: KeyboardEvent) => {
        if (event.key === "Escape") close();
      };

      for (const [key, label] of choices) {
        const item = element("label", "fg-menu-item fg-shows-item");
        const box = element("input") as HTMLInputElement;
        box.type = "checkbox";
        box.checked = this.shows[key];
        box.dataset["shows"] = key;
        box.addEventListener("change", () => {
          this.shows = { ...this.shows, [key]: box.checked };
          window.localStorage.setItem(SHOWS_KEY, JSON.stringify(this.shows));
          this.render();
        });

        item.append(box, element("span", undefined, t(label)));
        menu.append(item);
      }

      // Kept outside the island: every tick redraws it, and a menu inside would
      // be thrown away on the spot — leaving no way to tick a second box.
      document.body.append(menu);

      const box = menu.getBoundingClientRect();
      if (box.right > window.innerWidth) menu.style.left = `${window.innerWidth - box.width - 8}px`;

      setTimeout(() => {
        document.addEventListener("mousedown", close);
        document.addEventListener("keydown", onEscape);
      });
    });

    return button;
  }

  private renderHeader(origin: number, days: number): HTMLElement {
    const header = element("div", "fg-chart-header");
    // Stands in for the filter row on the other side. Without it every bar sits
    // one row above where it belongs.
    const spacer = element("div", "fg-filter-spacer");

    // The chart side has the filter row's height going spare, which is where the
    // switch goes: it decides how much is drawn over the bars, so it belongs
    // near them.
    spacer.append(this.renderShowToggle());

    const quarters = element("div", "fg-quarters");
    const months = element("div", "fg-months");
    const daysRow = element("div", "fg-days");

    let monthStart = 0;
    let quarterStart = 0;

    for (let i = 0; i <= days; i++) {
      const date = new Date(origin + i * DAY_MS);
      const isBoundary = i === days || date.getUTCDate() === 1;

      if (isBoundary && i > monthStart) {
        const first = new Date(origin + monthStart * DAY_MS);
        const width = (i - monthStart) * this.dayWidth;
        const label = element("div", "fg-month");
        label.style.width = `${width}px`;

        // A band too narrow for its own name would print it over the next
        // month's — the sticky text has nowhere to sit.
        if (width >= 56) {
          // The text pins to the left edge while its month is in view, so a
          // scrolled chart never leaves the visible weeks unlabelled.
          label.append(element("span", undefined, this.monthLabel(first)));
        }
        months.append(label);
        monthStart = i;
      }

      // The business year starts where the project says it does, which in Japan
      // is almost never January.
      if (isBoundary && i > quarterStart) {
        const first = new Date(origin + quarterStart * DAY_MS);
        const q = this.quarterOf(first);
        const previous = quarters.lastElementChild;

        if (previous && previous.getAttribute("data-quarter") === q.key) {
          const width = Number(previous.getAttribute("data-days")) + (i - quarterStart);
          previous.setAttribute("data-days", String(width));
          (previous as HTMLElement).style.width = `${width * this.dayWidth}px`;
        } else {
          const label = element("div", "fg-quarter");
          label.setAttribute("data-quarter", q.key);
          label.setAttribute("data-days", String(i - quarterStart));
          label.style.width = `${(i - quarterStart) * this.dayWidth}px`;
          label.append(element("span", undefined, q.label));
          quarters.append(label);
        }

        quarterStart = i;
      }

      if (i === days) break;

      const day = date.getUTCDay();
      const iso = date.toISOString().slice(0, 10);
      const holiday = this.holidayOn(iso);
      const cell = element("div", "fg-day");

      // The weekday under the date: every Japanese schedule prints it, and
      // counting to a Friday off the month grid is nobody's idea of a plan.
      cell.append(
        element("span", "fg-date", String(date.getUTCDate())),
        element("span", "fg-weekday", weekday(day)),
      );

      const note = this.dayNote(iso);
      if (note) cell.title = note;

      if (holiday) {
        cell.classList.add("is-holiday");
      } else if (day === 6) {
        cell.classList.add("is-saturday");
      } else if (day === 0) {
        cell.classList.add("is-sunday");
      }

      if (iso === this.data.today) cell.classList.add("is-today");
      daysRow.append(cell);
    }

    // The left pane's filter row has no counterpart over the chart, and without
    // one every bar sits a row above where it belongs.
    // Whether the business year and quarter band is drawn is a setting.
    if (this.data.quarters) header.append(spacer, quarters, months, daysRow);
    else header.append(spacer, months, daysRow);

    return header;
  }

  /**
   * One row, both bars.
   *
   * The plan is drawn as an outline and the actual work sits inside it, so the
   * difference between them is the shape rather than a second row to compare
   * against.
   */
  private renderBar(task: Task, origin: number, index: number): HTMLElement {
    const row = element("div", "fg-bar-row");

    // A chart row can be selected like a grid row. Selectable on one side only,
    // the same row would have places that answer and places that do not. The
    // column stays put: the chart has no columns, so a press cannot name one.
    row.addEventListener("mousedown", () => {
      if (this.editing) return;

      this.select(index, this.column);
      // This marks both sides and hands the keyboard back to the grid.
      this.repaintSelection();
    });
    if (index === this.row) row.classList.add("is-current");

    const span = (from: string | null, to: string | null) => {
      if (!from || !to) return null;

      const start = dayIndex(from, origin);
      const length = Math.max(1, dayIndex(to, origin) - start + 1);

      return { start, length };
    };

    // Leave. Whoever is on this row, the days they are away are shaded in this
    // row alone — a weekend is everyone's, a holiday is the project's, and this
    // is one person's. Drawn first so the bars keep their own colours on top.
    const away = task.assignee.trim();
    for (const leave of away ? this.data.leaves : []) {
      if (leave.assignee.trim() !== away || leave.kind === "on") continue;

      const slice = span(leave.start, leave.end);
      if (!slice) continue;

      const cells = element("div", "fg-leave");
      cells.style.left = `${slice.start * this.dayWidth}px`;
      cells.style.width = `${slice.length * this.dayWidth}px`;
      cells.title = `${leave.assignee} 休み${leave.note ? `（${leave.note}）` : ""}`;
      row.append(cells);
    }

    // Waiting. Drawn over the row like leave, but hatched: nobody was away, the
    // work itself was stopped. Days inside these ranges are not counted.
    for (const wait of task.waits) {
      // Clipped to the plan: a wait outside the task's own dates takes nothing
      // out of it, and drawing it there is a hatch floating over no bar.
      const from = task.start && wait.start < task.start ? task.start : wait.start;
      const to = task.end && wait.end > task.end ? task.end : wait.end;
      if (!task.start || !task.end || from > to) continue;

      const slice = span(from, to);
      if (!slice) continue;

      const gap = element("div", "fg-wait");
      if (wait.open) gap.classList.add("is-open");
      gap.style.left = `${slice.start * this.dayWidth}px`;
      gap.style.width = `${slice.length * this.dayWidth}px`;
      gap.title = [
        `待ち ${short(wait.start)}〜${wait.open ? "" : short(wait.end)}`,
        wait.reason,
        wait.open ? t("（継続中）") : "",
      ]
        .filter(Boolean)
        .join(" ");
      row.append(gap);
    }

    const planned = span(task.start, task.end);
    // Work that has started and not finished is drawn up to today: it is a
    // length, not a dot.
    const actual = span(task.actual_start, task.actual_end ?? this.data.today);

    if (!planned && !actual) return row;

    if (planned) {
      const bar = element("div", "fg-bar");
      if (this.behind(task)) bar.classList.add("is-delayed");
      if (task.has_children) bar.classList.add("is-summary");
      // The height does not depend on whether an actual exists. Bars of differing
      // thickness on one screen look like they mean different things.
      bar.classList.add("is-plan");

      bar.dataset["task"] = task.id;
      bar.dataset["progress"] = String(task.progress);
      bar.style.left = `${planned.start * this.dayWidth}px`;
      bar.style.width = `${planned.length * this.dayWidth}px`;
      bar.title = `予定 ${task.start} 〜 ${task.end}（${task.progress}%）${this.extraTip(task)}`;

      // Progress is always measured against the plan bar. Put on the actual, the
      // same 60% lands in a different place row by row, and an unfinished actual
      // runs to today — so the fill advances on days when nothing was done.
      const fill = element("div", "fg-bar-fill");
      fill.style.width = `${task.progress}%`;
      bar.append(fill);

      if (this.editable(task, column("start"))) {
        bar.classList.add("is-draggable");

        const knob = element("span", "fg-grip fg-grip-progress");
        // Kept inside the bar. At 0% its left half hung outside and at 100% its
        // right half did, leaving half a handle to grab — and overlapping the
        // test for the bar's own ends.
        const width = planned.length * this.dayWidth;
        const grip = this.dayWidth;
        // Not flush with the end. Sharing a spot with the end grip at 0% and
        // 100% means only whichever is on top can be grabbed, and either the
        // progress or the date becomes immovable. It stops GRIP_WIDTH inside.
        knob.style.left = `${clamp(
          (width * task.progress) / 100 - grip / 2,
          GRIP_WIDTH,
          Math.max(GRIP_WIDTH, width - grip - GRIP_WIDTH),
        )}px`;
        knob.title = t("ドラッグで進捗を変える");
        bar.append(knob);

        bar.append(
          element("span", "fg-grip fg-grip-start"),
          element("span", "fg-grip fg-grip-end"),
        );
        bar.addEventListener("pointerdown", (event) =>
          this.beginDrag(event, task, bar, planned.start, planned.length, index),
        );
      }

      row.append(bar);

      // 予定進捗, at the amount it promised, with the date written out.
      //
      // The other way round was tried first — the mark at the date, the amount
      // in words — and it failed in the hand rather than on paper. This bar is
      // where progress is set: grab it and the x axis is a percentage. So a
      // mark on it is read as a percentage whatever it was meant to be, and a
      // fill that reaches it reads as a promise kept. It happened twice in one
      // afternoon, both times to people who knew how it was built.
      //
      // Whatever is compared has to share a scale, and the fill is what this is
      // compared with. The date is compared with nothing here — it only decides
      // whether the promise counts yet — so the date is the part that is
      // written down.
      const due = this.shows.targets ? task.targets.filter((target) => target.due) : [];
      const promised = due.reduce<Task["targets"][number] | null>(
        (worst, target) => (worst === null || target.percent >= worst.percent ? target : worst),
        null,
      );

      // What is missing, drawn as the gap it is: from where the work got to, to
      // where it was meant to be. Caught up, it disappears — and the bar goes
      // back to being a plain blue bar, which is the whole of the feedback.
      if (promised && promised.percent > task.progress) {
        const behind = element("div", "fg-bar-behind");
        behind.style.left = `${task.progress}%`;
        behind.style.width = `${promised.percent - task.progress}%`;
        behind.title = `${promised.date} までに ${promised.percent}%（いま ${task.progress}%）`;
        bar.append(behind);

        const label = element(
          "div",
          "fg-target-label is-missed",
          `${short(promised.date)} ${promised.percent}%`,
        );
        label.style.left = `${planned.start * this.dayWidth + (planned.length * this.dayWidth * promised.percent) / 100 + 4}px`;
        label.title = behind.title;
        row.append(label);
      }

      // Promises whose day has not come, at the level they ask for. The date
      // beside them says which is which, and that they are not due yet.
      for (const target of this.shows.targets ? task.targets : []) {
        if (target.due) continue;

        const left =
          planned.start * this.dayWidth + (planned.length * this.dayWidth * target.percent) / 100;

        const mark = element("div", "fg-target");
        mark.style.left = `${left}px`;
        mark.title = `${target.date} までに ${target.percent}%`;
        row.append(mark);

        const label = element(
          "div",
          "fg-target-label",
          `${short(target.date)} ${target.percent}%`,
        );
        label.style.left = `${left + 4}px`;
        label.title = mark.title;
        row.append(label);
      }
    }

    if (actual) {
      const bar = element("div", "fg-actual");
      bar.style.left = `${actual.start * this.dayWidth}px`;
      bar.style.width = `${actual.length * this.dayWidth}px`;
      bar.title =
        (task.actual_end
          ? `実施 ${task.actual_start} 〜 ${task.actual_end}`
          : `実施 ${task.actual_start} 〜（進行中）`) + this.extraTip(task);
      if (!task.actual_end) bar.classList.add("is-open");

      // No fill here: the actual bar answers "when did this run", and the plan
      // bar answers "how much is done". One question each.
      if (this.editable(task, column("actual_start"))) {
        bar.classList.add("is-draggable");

        bar.append(
          element("span", "fg-grip fg-grip-start"),
          element("span", "fg-grip fg-grip-end"),
        );
        bar.addEventListener("pointerdown", (event) =>
          this.beginActualDrag(event, task, bar, actual.start, actual.length, index),
        );
      }

      row.append(bar);
    }

    // Days worked, immediately right of the actual bar: with "how many days"
    // beside "when", the length and the number need not be matched up by eye.
    if (this.shows.worked && actual && task.actual_days !== null && task.actual_days > 0) {
      const worked = element(
        "div",
        "fg-worked",
        LANG === "en"
          ? `${task.actual_days}d worked`
          : `実作業 ${task.actual_days}${this.workdayBased ? "営業日" : "日"}`,
      );
      worked.style.left = `${(actual.start + actual.length) * this.dayWidth + 6}px`;
      worked.title = t("実際に動いた日数。終わっていなければ今日まで数えます");
      row.append(worked);
    }

    // The start variance, drawn at the left edge where the two starts part —
    // reading it off the same end as the finish would hide which one slipped.
    if (this.shows.start && task.start_variance !== null && task.start_variance !== 0 && planned && actual) {
      const label = element(
        "div",
        "fg-variance is-start",
        this.varianceLabel(task, task.start_variance),
      );
      label.classList.add(task.start_variance > 0 ? "is-late" : "is-early");
      // Right-aligned into the space before the bar, so it never covers it.
      label.style.right = `calc(100% - ${Math.min(planned.start, actual.start) * this.dayWidth - 6}px)`;
      row.append(label);
    }

    // The number people actually look for: how far off the plan it ran. Placed
    // past whichever bar ends last — put it at the actual's end and a task that
    // finished early writes its number across the plan it beat.
    if (this.shows.end && task.end_variance !== null && task.end_variance !== 0 && planned) {
      const ends = [planned, actual]
        .filter((span) => span !== null)
        .map((span) => span!.start + span!.length);

      const label = element("div", "fg-variance", this.varianceLabel(task, task.end_variance));
      label.classList.add(task.end_variance > 0 ? "is-late" : "is-early");
      label.style.left = `${Math.max(...ends) * this.dayWidth + 6}px`;
      row.append(label);
    }

    return row;
  }

  /**
   * Drags the actual bar, or stretches it by an end.
   *
   * An unfinished bar has no right edge to speak of — it is drawn to today —
   * so dragging its body moves the start alone and only the right grip writes
   * an end. Inventing a finish date because someone nudged a bar sideways is
   * exactly the kind of record nobody would trust afterwards.
   */
  private beginActualDrag(
    event: PointerEvent,
    task: Task,
    bar: HTMLElement,
    from: number,
    span: number,
    index: number,
  ): void {
    if (event.button !== 0 || !task.actual_start) return;

    event.preventDefault();
    event.stopPropagation();

    const box = bar.getBoundingClientRect();
    const offset = event.clientX - box.left;
    const open = !task.actual_end;

    const mode: "move" | "start" | "end" =
      offset >= box.width - GRIP_WIDTH ? "end" : offset <= GRIP_WIDTH ? "start" : "move";

    this.select(index, 0);
    this.repaintSelection();

    const startX = event.clientX;
    const origin = parseDate(this.data.range_start);
    let shift = 0;

    // The move and up events are taken on the window while the pointer is down.
    // Held on the bar, a redraw mid-drag took the element and its listeners with
    // it, and letting go did nothing at all.
    bar.classList.add("is-dragging");

    const preview = (moveEvent: PointerEvent) => {
      shift = Math.round((moveEvent.clientX - startX) / this.dayWidth);

      if (mode === "start") shift = Math.min(shift, span - 1);
      if (mode === "end") shift = Math.max(shift, -(span - 1));

      const left = mode === "end" ? from : from + shift;
      const width = mode === "move" ? span : mode === "start" ? span - shift : span + shift;

      bar.style.left = `${left * this.dayWidth}px`;
      bar.style.width = `${width * this.dayWidth}px`;
    };

    const finish = async () => {
      window.removeEventListener("pointermove", preview);
      window.removeEventListener("pointerup", finish);
      window.removeEventListener("pointercancel", cancel);
      bar.classList.remove("is-dragging");

      if (shift === 0) {
        this.render();
        return;
      }

      const start = shiftDate(origin, from + (mode === "end" ? 0 : shift));
      const end = shiftDate(origin, from + span - 1 + (mode === "start" ? 0 : shift));

      // An open bar keeps its lack of an end unless the end itself was dragged.
      const edit =
        open && mode !== "end"
          ? { field: "actual_start", value: start }
          : open
            ? { field: "actual_end", value: end }
            : { field: "actual_schedule", value: `${start}/${end}` };

      await this.send(`/api/projects/${encodeURIComponent(this.projectId)}/tasks/${task.id}`, {
        method: "POST",
        body: edit,
        follow: task.id,
      });
    };

    const cancel = () => {
      window.removeEventListener("pointermove", preview);
      window.removeEventListener("pointerup", finish);
      window.removeEventListener("pointercancel", cancel);
      this.render();
    };

    window.addEventListener("pointermove", preview);
    window.addEventListener("pointerup", finish);
    window.addEventListener("pointercancel", cancel);
  }

  /**
   * Drags a bar, or stretches it by an end.
   *
   * The preview moves the element directly rather than re-rendering — a full
   * render per pointermove would be the 40ms path — and one commit goes out on
   * release. Start and end travel together in a single `schedule` write so the
   * row never passes through end-before-start.
   */
  private beginDrag(
    event: PointerEvent,
    task: Task,
    bar: HTMLElement,
    from: number,
    span: number,
    index: number,
  ): void {
    if (event.button !== 0 || !task.start || !task.end) return;

    event.preventDefault();
    event.stopPropagation();

    const box = bar.getBoundingClientRect();
    const offset = event.clientX - box.left;
    const fillEdge = (box.width * task.progress) / 100;

    // Pressed on the handle means progress, wherever that is. At 0% and 100% it
    // sits right beside the end grip, and deciding by distance let the end win
    // every time. The test is the handle's own coordinates rather than what was
    // under the pointer, so nothing drawn on top can change the answer.
    const knob = bar.querySelector<HTMLElement>(".fg-grip-progress")?.getBoundingClientRect();
    const onKnob = !!knob && event.clientX >= knob.left && event.clientX <= knob.right;

    // The ends win over the progress edge: away from 0% and 100% they are far
    // apart, and moving the bar is the more common intent.
    const mode: "move" | "start" | "end" | "progress" = onKnob
      ? "progress"
      : offset <= GRIP_WIDTH
        ? "start"
        : offset >= box.width - GRIP_WIDTH
          ? "end"
          : Math.abs(offset - fillEdge) <= GRIP_WIDTH
            ? "progress"
            : "move";

    if (mode === "progress") {
      this.beginProgressDrag(event, task, bar, box, index);
      return;
    }

    this.select(index, 0);
    this.repaintSelection();

    const startX = event.clientX;
    const origin = parseDate(this.data.range_start);
    let shift = 0;

    // The move and up events are taken on the window while the pointer is down,
    // so a redraw mid-drag cannot take the listeners away with the element.
    bar.classList.add("is-dragging");

    const preview = (moveEvent: PointerEvent) => {
      shift = Math.round((moveEvent.clientX - startX) / this.dayWidth);

      // A bar cannot be stretched shorter than the day it starts on.
      if (mode === "start") shift = Math.min(shift, span - 1);
      if (mode === "end") shift = Math.max(shift, -(span - 1));

      const left = mode === "end" ? from : from + shift;
      const width = mode === "move" ? span : mode === "start" ? span - shift : span + shift;

      bar.style.left = `${left * this.dayWidth}px`;
      bar.style.width = `${width * this.dayWidth}px`;
    };

    const finish = async () => {
      window.removeEventListener("pointermove", preview);
      window.removeEventListener("pointerup", finish);
      window.removeEventListener("pointercancel", cancel);
      bar.classList.remove("is-dragging");

      if (shift === 0) {
        this.render();
        return;
      }

      const start = shiftDate(origin, from + (mode === "end" ? 0 : shift));
      const end = shiftDate(origin, from + span - 1 + (mode === "start" ? 0 : shift));

      await this.send(`/api/projects/${encodeURIComponent(this.projectId)}/tasks/${task.id}`, {
        method: "POST",
        body: { field: "schedule", value: `${start}/${end}` },
        follow: task.id,
      });
    };

    const cancel = () => {
      window.removeEventListener("pointermove", preview);
      window.removeEventListener("pointerup", finish);
      window.removeEventListener("pointercancel", cancel);
      this.render();
    };

    window.addEventListener("pointermove", preview);
    window.addEventListener("pointerup", finish);
    window.addEventListener("pointercancel", cancel);
  }

  /**
   * Keeps the two panes at the same vertical position.
   *
   * Each scrolls on its own so that each can have sticky headers; a row and its
   * bar would otherwise drift apart as soon as anyone scrolled.
   */
  private syncPanes(left: HTMLElement, chart: HTMLElement): void {
    let mirroring = false;

    const follow = (from: HTMLElement, to: HTMLElement) => () => {
      if (mirroring) return;

      mirroring = true;
      to.scrollTop = from.scrollTop;
      // Cleared next frame: assigning scrollTop fires the other listener.
      requestAnimationFrame(() => {
        mirroring = false;
      });
    };

    left.addEventListener("scroll", follow(left, chart));
    chart.addEventListener("scroll", follow(chart, left));

    // The rows that are drawn follow the scroll. Both panes fire it, and only
    // a scroll that moves the window costs anything.
    const track = (pane: HTMLElement) => () => {
      this.scrollTop = pane.scrollTop;
      this.renderWindow();
    };

    left.addEventListener("scroll", track(left));
    chart.addEventListener("scroll", track(chart));
  }

  /**
   * The handle between the table and the chart.
   *
   * Widening the chart is the main thing anyone wants from this screen, and the
   * columns are the only thing in the way.
   */
  private renderSplitter(grid: HTMLElement): HTMLElement {
    const splitter = element("div", "fg-splitter");
    splitter.title = t("ドラッグで幅を変える");

    splitter.addEventListener("pointerdown", (event) => {
      event.preventDefault();

      const left = grid.querySelector<HTMLElement>(".fg-pane-left");
      if (!left) return;

      const startX = event.clientX;
      const startWidth = left.getBoundingClientRect().width;

      const drag = (move: PointerEvent) => {
        this.paneWidth = clamp(startWidth + move.clientX - startX, 160, 1200);
        grid.style.setProperty("--fg-pane-width", `${this.paneWidth}px`);
        this.pinColumns();
      };

      const stop = () => {
        window.removeEventListener("pointermove", drag);
        window.removeEventListener("pointerup", stop);
        window.localStorage.setItem(PANE_KEY, String(this.paneWidth));
      };

      window.addEventListener("pointermove", drag);
      window.addEventListener("pointerup", stop);
    });

    return splitter;
  }

  /**
   * The right-click menu.
   *
   * The outline moves are all on Alt+arrow, which nobody discovers on their
   * own; this is where they are named.
   */
  private openMenu(x: number, y: number): void {
    const task = this.selected;
    if (!task) return;

    const menu = element("div", "fg-menu");
    menu.style.left = `${x}px`;
    menu.style.top = `${y}px`;

    const close = (event?: Event) => {
      // A mousedown inside the menu is someone choosing an item. Closing on it
      // would detach the button before its own click could fire, and the menu
      // would do nothing at all.
      if (event && menu.contains(event.target as Node)) return;

      menu.remove();
      document.removeEventListener("mousedown", close);
      document.removeEventListener("keydown", onEscape);
    };

    const onEscape = (event: KeyboardEvent) => {
      if (event.key === "Escape") close();
    };

    const item = (label: string, shortcut: string, run: () => void) => {
      const button = element("button", "fg-menu-item");
      button.type = "button";
      button.append(element("span", undefined, label), element("kbd", undefined, shortcut));
      button.addEventListener("click", () => {
        close();
        run();
      });

      // The grid keeps focus on itself, so the button never gets it; without
      // this the pointer press would land on the cell behind the menu.
      button.addEventListener("mousedown", (event) => event.preventDefault());
      menu.append(button);
    };

    if (task.has_children) {
      const folded = this.collapsed.has(task.id);
      item(folded ? t("展開する") : t("折りたたむ"), folded ? `${MOD}→` : `${MOD}←`, () =>
        this.toggleCollapse(task),
      );
      menu.append(element("div", "fg-menu-rule"));
    }

    item(t("子タスクにする"), `${ALT}→`, () => void this.moveRow("indent"));
    item(t("階層を戻す"), `${ALT}←`, () => void this.moveRow("outdent"));
    item(t("上へ移動"), `${ALT}↑`, () => void this.moveRow("up"));
    item(t("下へ移動"), `${ALT}↓`, () => void this.moveRow("down"));
    menu.append(element("div", "fg-menu-rule"));
    item(t("下に行を追加"), `${MOD}Enter`, () => void this.insertRow());
    item(t("行を削除"), `${MOD}Delete`, () => void this.deleteRow());

    if (this.editable(task, column("name"))) {
      menu.append(element("div", "fg-menu-rule"));
      menu.append(this.renderPalette(task, close));
    }

    this.root.append(menu);

    // Keep the menu on screen when the click lands near an edge.
    const box = menu.getBoundingClientRect();
    if (box.right > window.innerWidth) menu.style.left = `${x - box.width}px`;
    if (box.bottom > window.innerHeight) menu.style.top = `${y - box.height}px`;

    // Deferred so the click that opened the menu does not immediately close it.
    setTimeout(() => {
      document.addEventListener("mousedown", close);
      document.addEventListener("keydown", onEscape);
    });
  }

  /**
   * The colours a row can be given, in the right-click menu.
   *
   * People were already marking rows by writing ★ or 【重要】 into the task
   * name, which is a colour with the wrong tool: it sorts, it exports, it is
   * part of the name for ever. This is the same intent with none of that.
   *
   * Swatches rather than a colour picker. A picker offers sixteen million
   * answers to a question with about eight, and half of them are unreadable
   * under black text.
   */
  private renderPalette(task: Task, close: () => void): HTMLElement {
    const block = element("div", "fg-palette");

    const row = (
      label: string,
      field: "background" | "color",
      choices: string[],
      current: string,
    ) => {
      const line = element("div", "fg-palette-row");
      line.append(element("span", "fg-palette-label", label));

      for (const colour of choices) {
        const swatch = element("button", "fg-swatch") as HTMLButtonElement;
        swatch.type = "button";
        swatch.title = colour;
        swatch.style.background = field === "background" ? colour : "#fff";
        if (field === "color") {
          swatch.style.color = colour;
          swatch.textContent = "A";
        }
        if (current.toLowerCase() === colour) swatch.classList.add("is-current");

        swatch.addEventListener("mousedown", (event) => event.preventDefault());
        swatch.addEventListener("click", () => {
          close();
          void this.paint(task, field, current.toLowerCase() === colour ? "" : colour);
        });

        line.append(swatch);
      }

      block.append(line);
    };

    row(t("背景"), "background", BACKGROUNDS, task.background);
    row(t("文字"), "color", TEXT_COLOURS, task.color);

    const clear = element("button", "fg-menu-item", t("色を消す")) as HTMLButtonElement;
    clear.type = "button";
    clear.addEventListener("mousedown", (event) => event.preventDefault());
    clear.addEventListener("click", () => {
      close();
      void this.paint(task, "both", "");
    });
    block.append(clear);

    return block;
  }

  /** Writes one of the row's colours, or clears both. */
  private async paint(
    task: Task,
    which: "background" | "color" | "both",
    colour: string,
  ): Promise<void> {
    const url = `/api/projects/${encodeURIComponent(this.projectId)}/tasks/${task.id}`;

    for (const field of which === "both" ? ["background", "color"] : [which]) {
      await this.send(url, {
        method: "POST",
        body: { field, value: colour },
        follow: task.id,
      });
    }
  }

  /** Keeps the keyboard where the user left it across a full re-render. */
  private restoreFocus(): void {
    if (this.editing) {
      const editor = this.root.querySelector<HTMLElement>(".fg-editor");
      if (editor instanceof HTMLInputElement) {
        editor.focus();
        // A seeded editor continues the word; an opened one replaces it.
        if (this.seed === null) editor.select();
        else editor.setSelectionRange(editor.value.length, editor.value.length);
      } else {
        editor?.focus();
      }
      return;
    }

    const typist = this.root.querySelector<HTMLInputElement>(".fg-editor.is-typist");
    if (typist) typist.focus({ preventScroll: true });
    else this.root.querySelector<HTMLElement>(".fg-grid")?.focus({ preventScroll: true });

    this.root
      .querySelector(".fg-cell.is-selected")
      ?.scrollIntoView({ block: "nearest", inline: "nearest" });
  }
}

async function start(): Promise<void> {
  const root = document.getElementById("fugantt-grid");
  if (!root) return;

  const projectId = root.dataset["project"];
  if (!projectId) return;

  try {
    const response = await fetch(`/api/projects/${encodeURIComponent(projectId)}/grid`, {
      headers: { accept: "application/json" },
    });

    if (!response.ok) throw new Error(`HTTP ${response.status}`);

    new Grid(root, projectId, (await response.json()) as GridData);
  } catch (error) {
    root.replaceChildren(
      element("p", "fg-empty", t("スケジュールを読み込めませんでした。再読み込みしてください。")),
    );
    console.error("fugantt: failed to load the grid", error);
  }
}

void start();
