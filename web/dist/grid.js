"use strict";
(() => {
  // src/grid.ts
  function column(key) {
    return BASE_COLUMNS.find((entry) => entry.key === key) ?? BASE_COLUMNS[0];
  }
  var BASE_COLUMNS = [
    { key: "name", label: "\u30BF\u30B9\u30AF", kind: "name" },
    // Late, as a column rather than as red text. A colour cannot be filtered,
    // sorted or exported, and the text colour now belongs to whoever painted the
    // row. A column can be asked a question: show me only these.
    {
      key: "late",
      label: "\u9045\u5EF6",
      kind: "select",
      options: [
        { value: "\u9045\u5EF6", color: "", background: "" },
        { value: "\u9806\u8ABF", color: "", background: "" }
      ]
    },
    // Who and what state, before any dates: the two things read at a glance.
    { key: "assignee", label: "\u62C5\u5F53\u8005", kind: "text" },
    { key: "status", label: "\u30B9\u30C6\u30FC\u30BF\u30B9", kind: "status" },
    // The plan, then what happened, in the same four columns each: when it
    // starts, when it ends, how many days, how far along. Read down one and then
    // the other and the pairs line up.
    { key: "start", label: "\u4E88\u5B9A\u958B\u59CB", kind: "date" },
    { key: "end", label: "\u4E88\u5B9A\u7D42\u4E86", kind: "date" },
    { key: "days", label: "\u4E88\u5B9A\u65E5\u6570", kind: "days" },
    // 予定進捗: entered, never derived. A list of "by this date, this much",
    // which is the only thing that can say whether the work is behind.
    { key: "targets", label: "\u4E88\u5B9A\u9032\u6357", kind: "text" },
    { key: "actual_start", label: "\u5B9F\u65BD\u958B\u59CB", kind: "date" },
    { key: "actual_end", label: "\u5B9F\u65BD\u7D42\u4E86", kind: "date" },
    // Days actually worked. Counted up to today while it is still running.
    { key: "actual_days", label: "\u5B9F\u4F5C\u696D\u65E5\u6570", kind: "days" },
    { key: "progress", label: "\u5B9F\u9032\u6357", kind: "progress" },
    // The subtraction, after both sides it subtracts.
    { key: "start_variance", label: "\u958B\u59CB\u5DEE\u7570", kind: "variance" },
    { key: "end_variance", label: "\u7D42\u4E86\u5DEE\u7570", kind: "variance" },
    { key: "waits", label: "\u5F85\u3061", kind: "text" },
    // Last on purpose: free text is the widest column and the least often read,
    // so it is the one that should run off the edge rather than push anything.
    { key: "note", label: "\u30B3\u30E1\u30F3\u30C8", kind: "text" }
  ];
  var ROLLED_UP = [
    "actual_days",
    "start",
    "end",
    "actual_start",
    "actual_end",
    "days",
    "start_variance",
    "end_variance",
    "progress"
  ];
  var BOUND_LABEL = {
    gte: "\u4EE5\u4E0A",
    lte: "\u4EE5\u4E0B",
    eq: "\u4E00\u81F4",
    gt: "\u8D85\u904E",
    lt: "\u672A\u6E80",
    behind: "\u9045\u308C",
    ahead: "\u9806\u8ABF"
  };
  var BOUND_MARK = {
    gte: "\u2267",
    lte: "\u2266",
    eq: "\uFF1D",
    gt: "\uFF1E",
    lt: "\uFF1C",
    behind: "\u9045\u308C",
    ahead: "\u9806\u8ABF"
  };
  var BOUND_CHOICES = {
    // Progress is more often asked as "only what is behind" than as a percentage,
    // and there is no number to type for that.
    progress: ["gte", "lte", "eq", "gt", "lt", "behind", "ahead"]
  };
  var BOUND_DEFAULT = ["gte", "lte", "eq", "gt", "lt"];
  var FILTER_BOUND = {
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
    end_variance: "gte"
  };
  var EN = {
    // columns
    "\u30BF\u30B9\u30AF": "Task",
    "\u4E88\u5B9A\u958B\u59CB": "Planned start",
    "\u4E88\u5B9A\u7D42\u4E86": "Planned end",
    "\u5B9F\u65BD\u958B\u59CB": "Actual start",
    "\u5B9F\u65BD\u7D42\u4E86": "Actual end",
    "\u4E88\u5B9A\u65E5\u6570": "Planned days",
    "\u5B9F\u4F5C\u696D\u65E5\u6570": "Actual days",
    "\u8868\u793A": "Show",
    "\u30C1\u30E3\u30FC\u30C8\u306B\u51FA\u3059\u3082\u306E\u3092\u9078\u3073\u307E\u3059": "Choose what to draw on the chart",
    "\u5B9F\u969B\u306B\u52D5\u3044\u305F\u65E5\u6570\u3002\u7D42\u308F\u3063\u3066\u3044\u306A\u3051\u308C\u3070\u4ECA\u65E5\u307E\u3067\u6570\u3048\u307E\u3059": "Days actually worked; counted up to today while it is still running",
    "\u958B\u59CB\u5DEE\u7570": "Start variance",
    "\u7D42\u4E86\u5DEE\u7570": "End variance",
    "\u4E88\u5B9A\u9032\u6357": "Planned",
    "\u9045\u5EF6": "Late",
    "\u4E88\u5B9A\u9032\u6357\u306B\u5C4A\u3044\u3066\u3044\u307E\u305B\u3093": "Not up to the checkpoint it promised",
    "\u4E88\u5B9A\u7D42\u4E86\u3092\u904E\u304E\u3066\u3001\u5B9F\u65BD\u7D42\u4E86\u304C\u5165\u3063\u3066\u3044\u307E\u305B\u3093": "Past its planned end, with no actual end",
    "\u8272\u3092\u6D88\u3059": "Clear the colour",
    "\u80CC\u666F": "Background",
    "\u6587\u5B57": "Text",
    "\u5B9F\u9032\u6357": "Progress",
    "\u9032\u6357": "Progress",
    "\u30B9\u30C6\u30FC\u30BF\u30B9": "Status",
    "\u62C5\u5F53\u8005": "Assignee",
    "\u30B3\u30E1\u30F3\u30C8": "Note",
    "\u5F85\u3061": "Waiting",
    // filtering
    "\u4EE5\u4E0A": "at least",
    "\u4EE5\u4E0B": "at most",
    "\u4E00\u81F4": "equals",
    "\u8D85\u904E": "more than",
    "\u672A\u6E80": "less than",
    "\u9045\u308C": "behind",
    "\u9806\u8ABF": "on track",
    "\u89E3\u9664": "Clear",
    "\u691C\u7D22\u6761\u4EF6": "Saved filters",
    "\u7D5E\u308A\u8FBC\u307F\u306E\u6761\u4EF6\u3092\u540D\u524D\u3092\u3064\u3051\u3066\u7F6E\u3044\u3066\u304A\u304F": "Keep a set of filters under a name",
    "\u3044\u307E\u306E\u6761\u4EF6\u306B\u540D\u524D\u3092\u3064\u3051\u3066\u4FDD\u5B58": "Name these filters to keep them",
    "\u307F\u3093\u306A\u3067\u4F7F\u3046": "Share with everybody",
    "\u307F\u3093\u306A\u306E": "Everybody's",
    "\u81EA\u5206\u306E": "Mine",
    "\u3053\u306E\u6761\u4EF6\u3092\u6D88\u3059": "Forget this one",
    "\u7D5E\u308A\u8FBC\u307F": "Filter",
    "20260805\u30FB8/5\u30FB2026-08-05 \u306E\u3069\u308C\u3067\u3082\u3002\u5DE6\u306E\u30DC\u30BF\u30F3\u3067\u5411\u304D\u3092\u5909\u3048\u3089\u308C\u307E\u3059": "20260805, 8/5 or 2026-08-05 all work. The button on the left changes the comparison.",
    "\u5DE6\u306E\u30DC\u30BF\u30F3\u3067\u300C\u4EE5\u4E0A\u300D\u300C\u4EE5\u4E0B\u300D\u3092\u5207\u308A\u66FF\u3048\u3089\u308C\u307E\u3059": "The button on the left switches between at least and at most.",
    "\u30AB\u30EC\u30F3\u30C0\u30FC\u304B\u3089\u9078\u3076": "Pick from a calendar",
    // calendar and units
    "\u4F11\u696D\u65E5": "Closed",
    "\u65E5": "d",
    "\u55B6\u696D\u65E5": "working days",
    "\u5143": "was",
    // dialogs
    "\u4F11\u307F": "Away",
    "\u51FA\u793E": "Working",
    "\u30E1\u30E2\uFF08\u4EFB\u610F\uFF09": "Note (optional)",
    "\u524A\u9664": "Delete",
    "\uFF0B \u4F11\u6687\u3092\u8FFD\u52A0": "+ Add leave",
    "\u4FDD\u5B58": "Save",
    "\u30AD\u30E3\u30F3\u30BB\u30EB": "Cancel",
    "\u62C5\u5F53\u8005\u306E\u4F11\u6687 / \u51FA\u793E": "Leave and working days",
    "\u62C5\u5F53\u8005\u306E\u4F11\u6687/\u51FA\u793E": "Leave and working days",
    "\u4F11\u307F\u306E\u65E5\u306F\u305D\u306E\u4EBA\u306E\u30BF\u30B9\u30AF\u306E\u65E5\u6570\u306B\u3082\u9045\u308C\u306E\u5224\u5B9A\u306B\u3082\u5165\u308A\u307E\u305B\u3093\u3002\u9006\u306B\u300C\u51FA\u793E\u300D\u306F\u3001\u571F\u65E5\u795D\u3067\u3082\u305D\u306E\u65E5\u3092\u6570\u3048\u307E\u3059\u3002": "Days away count towards neither the day count nor the delay of that person's tasks. Working days do the opposite: they count even on a weekend or a holiday.",
    "\u4E88\u5B9A\u306F\u4EBA\u306B\u3064\u304F\u306E\u3067\u3001\u3053\u3053\u3067\u306E\u767B\u9332\u306F\u305D\u306E\u4EBA\u304C\u51FA\u3066\u3044\u308B\u5168\u90E8\u306E\u30D7\u30ED\u30B8\u30A7\u30AF\u30C8\u306B\u52B9\u304D\u307E\u3059\u3002": "Leave belongs to the person, so what you record here applies to every project they are on.",
    "\u7D99\u7D9A\u4E2D": "still open",
    "\u7406\u7531\uFF08\u4EFB\u610F\uFF09": "Reason (optional)",
    "\uFF0B \u671F\u9593\u3092\u8FFD\u52A0": "+ Add a period",
    "\u5F85\u3061\u306E\u671F\u9593\u3092\u767B\u9332\u3059\u308B": "Record the waiting periods",
    "\u4E88\u5B9A\u9032\u6357\u3092\u767B\u9332\u3059\u308B": "Record what should be done by when",
    "\uFF0B \u4E88\u5B9A\u3092\u8FFD\u52A0": "+ Add a checkpoint",
    "\u307E\u3067\u306B": "by then",
    "\u305D\u306E\u65E5\u3092\u904E\u304E\u3066\u3082\u5B9F\u9032\u6357\u304C\u5C4A\u3044\u3066\u3044\u306A\u3051\u308C\u3070\u9045\u308C\u306B\u306A\u308A\u307E\u3059\u3002\u9593\u306E\u65E5\u306F\u5224\u5B9A\u3057\u307E\u305B\u3093\u3002\u5165\u308C\u306A\u3051\u308C\u3070\u3001\u3053\u306E\u884C\u306F\u9032\u6357\u3067\u306F\u9045\u308C\u306B\u306A\u308A\u307E\u305B\u3093\u3002": "Once that date has passed, the row is behind if the work has not reached that percentage. Nothing is judged in between, and a row with no checkpoints is never behind on progress.",
    "\u3053\u306E\u65E5\u307E\u3067\u306B\u5C4A\u3044\u3066\u3044\u307E\u305B\u3093": "Not there by this date",
    "\u9054\u6210": "Met",
    "\u3053\u308C\u304B\u3089": "Still to come",
    "\u7D42\u308F\u308A\u3092\u7A7A\u306B\u3059\u308B\u3068\u300C\u307E\u3060\u5F85\u3063\u3066\u3044\u308B\u300D\u306B\u306A\u308A\u3001\u4ECA\u65E5\u307E\u3067\u6570\u3048\u7D9A\u3051\u307E\u3059\u3002\u5F85\u3061\u306E\u65E5\u6570\u306F\u65E5\u6570\u304B\u3089\u3082\u9045\u308C\u306E\u5224\u5B9A\u304B\u3089\u3082\u5916\u308C\u307E\u3059\u3002": "Leave the end empty for work that is still waiting; it counts up to today. Waiting days are excluded from the day count and from the delay.",
    "\uFF08\u7D99\u7D9A\u4E2D\uFF09": "(still waiting)",
    "\u4E88\u5B9A\u306E\u671F\u9593\u306E\u5916\u306A\u306E\u3067\u65E5\u6570\u306B\u306F\u52B9\u304D\u307E\u305B\u3093": "Outside the planned dates, so it changes nothing",
    "8/17\u301C8/21 \u4ED6\u90E8\u7F72\uFF08\u7D42\u308F\u308A\u7701\u7565\u3067\u7D99\u7D9A\u4E2D\uFF09": "8/17-8/21 another team (omit the end while it is still waiting)",
    // the grid
    "\uFF08\u7121\u984C\uFF09": "(untitled)",
    "\u7121\u984C\u306E\u30BF\u30B9\u30AF": "Untitled task",
    "\u304C\u66F4\u65B0\u3057\u307E\u3057\u305F": "made a change",
    "\u4ECA\u65E5": "Today",
    "\u4ECA\u65E5\u306E\u3068\u3053\u308D\u3078\u623B\u308B": "Back to today",
    "\u4FDD\u5B58\u3067\u304D\u307E\u305B\u3093\u3067\u3057\u305F\u3002\u63A5\u7D9A\u3092\u78BA\u8A8D\u3057\u3066\u304F\u3060\u3055\u3044\u3002": "Could not save. Check the connection.",
    "\u53D6\u308A\u6D88\u305B\u308B\u64CD\u4F5C\u304C\u3042\u308A\u307E\u305B\u3093\u3002": "Nothing to undo.",
    "\u3084\u308A\u76F4\u305B\u308B\u64CD\u4F5C\u304C\u3042\u308A\u307E\u305B\u3093\u3002": "Nothing to redo.",
    "\u305D\u306E\u884C\u306F\u3082\u3046\u3042\u308A\u307E\u305B\u3093\u3002": "That row is gone.",
    "\u305D\u306E\u884C\u306F\u8AB0\u304B\u304C\u52D5\u304B\u3057\u307E\u3057\u305F\u3002": "Somebody else moved that row.",
    "\u305D\u306E\u884C\u306B\u306F\u5B50\u30BF\u30B9\u30AF\u304C\u3042\u308B\u306E\u3067\u3001\u53D6\u308A\u6D88\u3057\u3067\u306F\u6D88\u3057\u307E\u305B\u3093\u3002": "That row has child tasks, so undo will not remove it.",
    "\u66F8\u304D\u8FBC\u307F\u306E\u3042\u308B\u884C\u306F\u3001\u53D6\u308A\u6D88\u3057\u3067\u306F\u6D88\u3057\u307E\u305B\u3093\u3002": "That row has something in it, so undo will not remove it.",
    "\u884C\u306E\u524A\u9664\u306F\u53D6\u308A\u6D88\u305B\u307E\u305B\u3093\u3002\u3082\u3046\u4E00\u5EA6\u62BC\u3059\u3068\u3001\u305D\u306E\u524D\u306E\u5909\u66F4\u3092\u53D6\u308A\u6D88\u3057\u307E\u3059\u3002": "Deleting a row cannot be undone. Press again to undo the change before it.",
    "\u9589\u3058\u308B": "Close",
    "\u30BF\u30B9\u30AF\u304C\u3042\u308A\u307E\u305B\u3093\u3002": "No tasks yet.",
    "\u6700\u521D\u306E\u30BF\u30B9\u30AF\u3092\u8FFD\u52A0": "Add the first task",
    "\u95B2\u89A7\u306E\u307F": "Read only",
    "\u884C\u3092\u8FFD\u52A0": "Add a row",
    "\u884C\u3092\u524A\u9664": "Delete the row",
    "\u8AB0\u304C\u3044\u3064\u4F11\u307F\u3001\u3044\u3064\u51FA\u308B\u304B\u3002\u65E5\u6570\u306E\u6570\u3048\u65B9\u306B\u52B9\u304D\u307E\u3059": "Who is away and who is in. It changes how days are counted.",
    "\u571F\u65E5\u30FB\u795D\u65E5\u3092\u9664\u3044\u305F\u55B6\u696D\u65E5\u3067\u6570\u3048\u3066\u3044\u307E\u3059": "Counted in working days, weekends and holidays excluded",
    "\u6761\u4EF6\u306B\u5408\u3046\u884C\u304C\u3042\u308A\u307E\u305B\u3093\u3002": "Nothing matches.",
    "\u3053\u306E\u30D7\u30ED\u30B8\u30A7\u30AF\u30C8\u306F\u8AAD\u3080\u3060\u3051\u3067\u3059\u3002": "You can read this project, not change it.",
    "\u65E5\u6570\u306F\u65E5\u4ED8\u304B\u3089\u6570\u3048\u307E\u3059\u3002": "The day count comes from the dates.",
    "\u9045\u5EF6\u306F\u4E88\u5B9A\u9032\u6357\u3068\u5B9F\u9032\u6357\u304B\u3089\u6C7A\u307E\u308A\u307E\u3059\u3002": "Late is read from the promised progress against the real one.",
    "\u5DEE\u7570\u306F\u4E88\u5B9A\u3068\u5B9F\u65BD\u306E\u5DEE\u3067\u3059\u3002": "A variance is the gap between the plan and what happened.",
    "\u96C6\u8A08\u884C\u306E\u65E5\u4ED8\u3068\u9032\u6357\u306F\u5B50\u30BF\u30B9\u30AF\u304B\u3089\u6C7A\u307E\u308A\u307E\u3059\u3002": "A summary row's dates and progress come from its children.",
    "\u5B50\u30BF\u30B9\u30AF\u306E\u305A\u308C\u3092\u8DB3\u3057\u305F\u3082\u306E\u3067\u3059\uFF08\u3053\u306E\u884C\u306E\u65E5\u4ED8\u306E\u5DEE\u3067\u306F\u3042\u308A\u307E\u305B\u3093\uFF09": "The sum of the children's slippage, not the difference between this row's own dates",
    "\u30C9\u30E9\u30C3\u30B0\u3067\u79FB\u52D5": "Drag to move",
    "\u5C55\u958B\u3059\u308B": "Expand",
    "\u6298\u308A\u305F\u305F\u3080": "Collapse",
    "\u30BB\u30EB\u306E\u5165\u529B": "Cell editor",
    "\u30C9\u30E9\u30C3\u30B0\u3067\u9032\u6357\u3092\u5909\u3048\u308B": "Drag to change the progress",
    "\u30C9\u30E9\u30C3\u30B0\u3067\u5E45\u3092\u5909\u3048\u308B": "Drag to resize",
    "\u5B50\u30BF\u30B9\u30AF\u306B\u3059\u308B": "Make it a child",
    "\u968E\u5C64\u3092\u623B\u3059": "Move it back out",
    "\u4E0A\u3078\u79FB\u52D5": "Move up",
    "\u4E0B\u3078\u79FB\u52D5": "Move down",
    "\u4E0B\u306B\u884C\u3092\u8FFD\u52A0": "Add a row below",
    "\u30B9\u30B1\u30B8\u30E5\u30FC\u30EB\u3092\u8AAD\u307F\u8FBC\u3081\u307E\u305B\u3093\u3067\u3057\u305F\u3002\u518D\u8AAD\u307F\u8FBC\u307F\u3057\u3066\u304F\u3060\u3055\u3044\u3002": "Could not load the schedule. Please reload."
  };
  var LANG = "ja";
  function t(ja) {
    return LANG === "en" ? EN[ja] ?? ja : ja;
  }
  var WEEKDAYS_EN = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
  var WEEKDAYS = ["\u65E5", "\u6708", "\u706B", "\u6C34", "\u6728", "\u91D1", "\u571F"];
  function weekday(at) {
    return (LANG === "en" ? WEEKDAYS_EN[at] : WEEKDAYS[at]) ?? "";
  }
  var DAY_MS = 864e5;
  var PAGE_ROWS = 10;
  var GRIP_WIDTH = 7;
  function parseDate(text) {
    const [year, month, day] = text.split("-").map(Number);
    return Date.UTC(year ?? 1970, (month ?? 1) - 1, day ?? 1);
  }
  function dayIndex(date, origin) {
    return Math.round((parseDate(date) - origin) / DAY_MS);
  }
  function shiftDate(origin, offset) {
    return new Date(origin + offset * DAY_MS).toISOString().slice(0, 10);
  }
  function element(tag, className, text) {
    const node = document.createElement(tag);
    if (className) node.className = className;
    if (text !== void 0) node.textContent = text;
    return node;
  }
  function parseBound(text, fallback) {
    const value = normalizeWidth(text).trim();
    if (!value) return null;
    const SIGNS = {
      ">=": "gte",
      "=>": "gte",
      "\u2267": "gte",
      "\u2265": "gte",
      "<=": "lte",
      "=<": "lte",
      "\u2266": "lte",
      "\u2264": "lte",
      ">": "gt",
      "\uFF1E": "gt",
      "<": "lt",
      "\uFF1C": "lt",
      "=": "eq",
      "\uFF1D": "eq"
    };
    const written = /^(>=|<=|=>|=<|≧|≥|≦|≤|＞|＜|＝|>|<|=)\s*(.*)$/.exec(value);
    if (written) {
      return { at: SIGNS[written[1]] ?? "eq", limit: written[2].trim() };
    }
    const WORDS = [
      [/(以上|以降|いじょう|いこう)$/, "gte"],
      [/(以下|以前|まで|いか|いぜん)$/, "lte"],
      [/(超過|超|より後|より大きい)$/, "gt"],
      [/(未満|より前|より小さい)$/, "lt"],
      [/(と同じ|一致|ちょうど)$/, "eq"]
    ];
    for (const [pattern, at] of WORDS) {
      if (pattern.test(value)) return { at, limit: value.replace(pattern, "").trim() };
    }
    return fallback ? { at: fallback, limit: value } : null;
  }
  function compare(at, left, right) {
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
        return left === right;
    }
  }
  function flexibleDate(text) {
    const value = normalizeWidth(text).trim().replace(/[/.年月]/g, "-").replace(/日/g, "").replace(/-+$/, "");
    const year = (/* @__PURE__ */ new Date()).getUTCFullYear();
    let iso = null;
    if (/^\d+$/.test(value)) {
      if (value.length === 8) iso = `${value.slice(0, 4)}-${value.slice(4, 6)}-${value.slice(6)}`;
      else if (value.length === 4) iso = `${year}-${value.slice(0, 2)}-${value.slice(2)}`;
    } else {
      const parts = value.split("-").filter(Boolean);
      const pad = (part, width) => part.padStart(width, "0");
      if (parts.length === 2) iso = `${year}-${pad(parts[0], 2)}-${pad(parts[1], 2)}`;
      else if (parts.length === 3) iso = `${pad(parts[0], 4)}-${pad(parts[1], 2)}-${pad(parts[2], 2)}`;
    }
    if (!iso || !/^\d{4}-\d{2}-\d{2}$/.test(iso)) return null;
    const parsed = /* @__PURE__ */ new Date(`${iso}T00:00:00Z`);
    return Number.isNaN(parsed.getTime()) || parsed.toISOString().slice(0, 10) !== iso ? null : iso;
  }
  var MONTHS_EN = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
  var MONTH_NAMES_EN = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December"
  ];
  function short(iso) {
    const [, month, day] = iso.split("-");
    if (!month || !day) return iso;
    const name = MONTHS_EN[Number(month) - 1];
    return LANG === "en" && name ? `${name} ${Number(day)}` : `${Number(month)}/${Number(day)}`;
  }
  function fullDate(iso) {
    return LANG === "en" ? iso : iso.replace(/-/g, "/");
  }
  function normalizeWidth(text) {
    return text.replace(/[０-９ａ-ｚＡ-Ｚ]/g, (c) => String.fromCharCode(c.charCodeAt(0) - 65248)).replace(/[－ー−‐]/g, "-").replace(/／/g, "/").replace(/％/g, "%").replace(/　/g, " ");
  }
  function clamp(value, min, max) {
    return Math.min(max, Math.max(min, value));
  }
  var CLIENT_ID = randomId();
  function randomId() {
    const bytes = new Uint8Array(16);
    if (globalThis.crypto?.getRandomValues) {
      globalThis.crypto.getRandomValues(bytes);
    } else {
      for (let i = 0; i < bytes.length; i++) bytes[i] = Math.floor(Math.random() * 256);
    }
    return [...bytes].map((b) => b.toString(16).padStart(2, "0")).join("");
  }
  var ON_MAC = /Mac|iPhone|iPad/.test(navigator.userAgent);
  var MOD = ON_MAC ? "\u2318" : "Ctrl+";
  var ALT = ON_MAC ? "\u2325" : "Alt+";
  var collapsedKey = (projectId) => `fugantt:collapsed:${projectId}`;
  function loadCollapsed(projectId) {
    try {
      const stored = window.localStorage.getItem(collapsedKey(projectId));
      return new Set(stored ? JSON.parse(stored) : []);
    } catch {
      return /* @__PURE__ */ new Set();
    }
  }
  function saveCollapsed(projectId, collapsed) {
    try {
      window.localStorage.setItem(collapsedKey(projectId), JSON.stringify([...collapsed]));
    } catch {
    }
  }
  var TRACKS = {
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
    select: "minmax(5rem, 0.6fr)"
  };
  var BACKGROUNDS = [
    "#fef3c7",
    "#dcfce7",
    "#dbeafe",
    "#fce7f3",
    "#ede9fe",
    "#ffe4e6",
    "#e2e8f0"
  ];
  var TEXT_COLOURS = ["#0f172a", "#b91c1c", "#a16207", "#15803d", "#1d4ed8", "#7e22ce"];
  var PANE_KEY = "fugantt:pane-width";
  var SHOWS_KEY = "fugantt:chart-shows";
  function loadShows() {
    const stored = window.localStorage.getItem(SHOWS_KEY);
    const shows = { start: true, end: true, worked: true, targets: true };
    if (!stored) return shows;
    try {
      return { ...shows, ...JSON.parse(stored) };
    } catch {
      return shows;
    }
  }
  function loadPaneWidth() {
    const stored = Number(window.localStorage.getItem(PANE_KEY));
    return Number.isFinite(stored) && stored > 0 ? clamp(stored, 160, 1200) : 0;
  }
  function keepMatches(tasks, hit) {
    const matches = tasks.map(hit);
    const keep = new Array(tasks.length).fill(false);
    for (const [index, task] of tasks.entries()) {
      if (!matches[index]) continue;
      keep[index] = true;
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
  var _Grid = class _Grid {
    constructor(root, projectId, data) {
      this.root = root;
      this.projectId = projectId;
      this.data = data;
      this.row = 0;
      this.column = 0;
      this.editing = false;
      /** The character that opened the editor, so typing does not lose the keystroke. */
      this.seed = null;
      this.error = null;
      /** True while an IME conversion is open, so nothing may re-render under it. */
      this.composing = false;
      /** True while the open editor is being carried to its rebuilt cell. */
      this.moving = false;
      /** A passing line about someone else's change, not a problem to fix. */
      this.notice = null;
      this.noticeTimer = 0;
      this.busy = false;
      /**
       * Where the chart is scrolled to sideways, or null before it has been drawn.
       *
       * Null rather than zero: a plan that starts in April is *at* zero when
       * somebody is reading April, and taking that for "never scrolled" sent them
       * back to today on every edit.
       */
      this.scrollLeft = null;
      /** `data.tasks` minus everything inside a folded row. All indices are into this. */
      this.visible = [];
      /** One filter per column, ANDed together. Empty entries are ignored. */
      this.filters = /* @__PURE__ */ new Map();
      /**
       * The direction each bounded column is asking in, where it is not the
       * default. Chosen by clicking, rather than by remembering what to type.
       */
      this.bounds = /* @__PURE__ */ new Map();
      /** What to draw over the chart: this person's view of it, not the project's
          setting. */
      this.shows = loadShows();
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
      this.first = 0;
      this.last = -1;
      /** Measured from the page: a row's height, and where the rows begin. */
      this.rowPixels = 32;
      this.rowsTop = 0;
      this.scrollTop = 0;
      this.done = [];
      this.undone = [];
      /** Set while undoing, so putting a value back is not itself recorded. */
      this.replaying = false;
      /** The column whose filter box the caret is in, across a re-render. */
      this.filterFocus = null;
      /** How much of the width the left pane takes, dragged by the splitter. */
      this.paneWidth = loadPaneWidth();
      /** Whether the table pane still needs holding back to half the window. */
      this.capPaneWidth = false;
      /** The last cell pressed, for spotting a double-click ourselves. */
      this.lastPress = null;
      this.collapsed = loadCollapsed(projectId);
      LANG = data.language === "en" ? "en" : "ja";
      this.computeVisible();
      this.root.addEventListener("keydown", (event) => this.onKeyDown(event));
      window.addEventListener("resize", () => this.pinColumns());
      this.listen();
      this.render();
    }
    /** Width of one day column. Comes from the project's settings. */
    get dayWidth() {
      return this.data.day_width || 26;
    }
    /**
     * Follows other people's changes.
     *
     * The event carries only a revision, so a client that hears one refetches
     * rather than trying to apply someone else's edit. Our own writes come back
     * too, but by then we already hold that revision, so they fall through.
     */
    listen() {
      const source = new EventSource(
        `/api/projects/${encodeURIComponent(this.projectId)}/live`
      );
      source.addEventListener("change", (event) => {
        const change = JSON.parse(event.data);
        if (change.client === CLIENT_ID) return;
        if (change.revision <= this.data.revision) return;
        if (change.kind === "cell" && change.task_id) {
          void this.follow(change.task_id, change.actor);
          return;
        }
        void this.refresh(change.actor);
      });
    }
    /** Reloads the grid after someone else changed it, keeping the cursor put. */
    /** Takes in one row somebody else changed, without reading the plan back. */
    async follow(taskId, actor) {
      if (this.editing || this.composing) return;
      try {
        const response = await fetch(
          `/api/projects/${encodeURIComponent(this.projectId)}/tasks/${encodeURIComponent(taskId)}/patch`
        );
        if (!response.ok) {
          void this.refresh(actor);
          return;
        }
        const drew = this.applyPatch(await response.json());
        if (drew === null) {
          void this.refresh(actor);
          return;
        }
        this.showNotice(`${actor} ${t("\u304C\u66F4\u65B0\u3057\u307E\u3057\u305F")}`);
        this.repaintRows(drew);
      } catch {
      }
    }
    async refresh(actor) {
      if (this.editing || this.composing) return;
      const here = this.selected?.id;
      try {
        const response = await fetch(
          `/api/projects/${encodeURIComponent(this.projectId)}/grid`
        );
        if (!response.ok) return;
        const shape = this.shape();
        const rows = this.tasks.map((task) => JSON.stringify(task));
        this.setData(await response.json());
        const moved = here ? this.tasks.findIndex((task) => task.id === here) : -1;
        this.select(moved >= 0 ? moved : this.row, this.column);
        this.notice = `${actor} \u304C\u66F4\u65B0\u3057\u307E\u3057\u305F`;
        window.clearTimeout(this.noticeTimer);
        this.noticeTimer = window.setTimeout(() => {
          this.notice = null;
          this.paintNotice();
        }, 4e3);
        this.repaintChanged(shape, rows);
      } catch {
      }
    }
    // --- data ----------------------------------------------------------------
    get tasks() {
      return this.visible;
    }
    setData(grid) {
      this.data = grid;
      LANG = grid.language === "en" ? "en" : "ja";
      this.computeVisible();
    }
    /**
     * Drops every row that sits under a folded one.
     *
     * The server already hands the tree back flattened depth-first, so a folded
     * row's whole subtree is exactly the run of deeper rows that follows it.
     */
    computeVisible() {
      const visible = [];
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
      const keys = /* @__PURE__ */ new Set([...this.filters.keys(), ...this.stateColumns.map((c) => c.key)]);
      const conditions = [...keys].map((key) => ({
        column: this.columns.find((column2) => column2.key === key),
        needle: this.filters.get(key) ?? ""
      })).filter((condition) => condition.column !== void 0);
      this.visible = keepMatches(
        visible,
        (task) => conditions.every(({ column: column2, needle }) => this.matches(task, column2, needle))
      );
    }
    /** Wires the header's filter box, which lives outside the island's markup. */
    updateFilterCount() {
      const label = document.getElementById("fugantt-filter-count");
      if (!label) return;
      label.textContent = "";
      if (this.filtering) {
        label.append(
          element("span", "fg-filter-count", `\u7D5E\u308A\u8FBC\u307F\u4E2D ${this.tasks.length} / ${this.data.tasks.length} \u884C`)
        );
        const clear = element("button", "fg-filter-clear", t("\u89E3\u9664"));
        clear.type = "button";
        clear.addEventListener("click", () => this.clearFilters());
        label.append(clear);
      }
      const saved = element("button", "fg-filter-sets", t("\u691C\u7D22\u6761\u4EF6"));
      saved.type = "button";
      saved.title = t("\u7D5E\u308A\u8FBC\u307F\u306E\u6761\u4EF6\u3092\u540D\u524D\u3092\u3064\u3051\u3066\u7F6E\u3044\u3066\u304A\u304F");
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
    openFilterSets(anchor) {
      const box = anchor.getBoundingClientRect();
      const menu = element("div", "fg-menu fg-sets-menu");
      menu.style.left = `${box.left}px`;
      menu.style.top = `${box.bottom + 2}px`;
      const close = (event) => {
        if (event && menu.contains(event.target)) return;
        menu.remove();
        document.removeEventListener("mousedown", close);
        document.removeEventListener("keydown", onEscape);
      };
      const onEscape = (event) => {
        if (event.key === "Escape") close();
      };
      const section = (label, shared) => {
        const sets = this.data.filter_sets.filter((set) => set.shared === shared);
        if (sets.length === 0) return;
        menu.append(element("div", "fg-menu-label", label));
        for (const set of sets) {
          const row = element("div", "fg-sets-row");
          const use = element("button", "fg-menu-item", set.name);
          use.type = "button";
          use.addEventListener("mousedown", (event) => event.preventDefault());
          use.addEventListener("click", () => {
            close();
            this.applyConditions(set.conditions);
          });
          const drop = element("button", "fg-sets-remove", "\xD7");
          drop.type = "button";
          drop.title = t("\u3053\u306E\u6761\u4EF6\u3092\u6D88\u3059");
          drop.addEventListener("mousedown", (event) => event.preventDefault());
          drop.addEventListener("click", async () => {
            close();
            await this.send(
              `/api/projects/${encodeURIComponent(this.projectId)}/filters/remove`,
              { method: "POST", body: { id: set.id } }
            );
          });
          row.append(use, drop);
          menu.append(row);
        }
      };
      section(t("\u307F\u3093\u306A\u306E"), true);
      section(t("\u81EA\u5206\u306E"), false);
      if (this.data.filter_sets.length > 0) menu.append(element("div", "fg-menu-rule"));
      const name = element("input", "fg-sets-name");
      name.type = "text";
      name.placeholder = t("\u3044\u307E\u306E\u6761\u4EF6\u306B\u540D\u524D\u3092\u3064\u3051\u3066\u4FDD\u5B58");
      const mine = element("label", "fg-sets-scope");
      const tick = element("input");
      tick.type = "checkbox";
      mine.append(tick, element("span", void 0, t("\u307F\u3093\u306A\u3067\u4F7F\u3046")));
      const save = element("button", "fg-menu-item fg-sets-save", t("\u4FDD\u5B58"));
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
            conditions: this.conditions()
          }
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
    conditions() {
      return JSON.stringify({
        filters: Object.fromEntries(this.filters),
        bounds: Object.fromEntries(this.bounds)
      });
    }
    /** Puts a saved set of conditions back on the screen. */
    applyConditions(stored) {
      let saved;
      try {
        saved = JSON.parse(stored);
      } catch {
        return;
      }
      this.filters = new Map(Object.entries(saved.filters ?? {}));
      this.bounds = new Map(
        Object.entries(saved.bounds ?? {}).map(([key, value]) => [key, value])
      );
      this.filterFocus = null;
      this.computeVisible();
      this.select(this.row, this.column);
      this.render();
    }
    /** Empties every filter box. */
    clearFilters() {
      this.filters.clear();
      this.bounds.clear();
      this.filterFocus = null;
      this.computeVisible();
      this.render();
      this.updateFilterCount();
    }
    get filtering() {
      return [...this.filters.values()].some((value) => value !== "") || this.stateColumns.length > 0;
    }
    /** Which way a column's filter points, once the user has had a say. */
    boundFor(column2) {
      const chosen = this.bounds.get(column2.key) ?? FILTER_BOUND[column2.key];
      if (chosen) return chosen;
      return column2.fieldId && (column2.kind === "date" || column2.kind === "number") ? "gte" : void 0;
    }
    /** Bounds that are a condition on their own, with nothing to type. */
    get stateColumns() {
      return this.columns.filter((column2) => {
        const bound = this.boundFor(column2);
        return bound === "behind" || bound === "ahead";
      });
    }
    setBound(column2, at) {
      this.bounds.set(column2.key, at);
      this.filterFocus = { key: column2.key, caret: null };
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
    openBoundMenu(column2, chip) {
      const choices = BOUND_CHOICES[column2.key] ?? BOUND_DEFAULT;
      const current = this.boundFor(column2);
      const anchor = chip.getBoundingClientRect();
      const menu = element("div", "fg-menu fg-bound-menu");
      menu.style.left = `${anchor.left}px`;
      menu.style.top = `${anchor.bottom + 2}px`;
      const close = (event) => {
        if (event && menu.contains(event.target)) return;
        menu.remove();
        document.removeEventListener("mousedown", close);
        document.removeEventListener("keydown", onEscape);
      };
      const onEscape = (event) => {
        if (event.key === "Escape") close();
      };
      for (const at of choices) {
        const button = element("button", "fg-menu-item");
        button.type = "button";
        button.dataset["bound"] = at;
        if (at === current) button.classList.add("is-current");
        button.append(
          element("span", void 0, BOUND_LABEL[at]),
          element("kbd", void 0, BOUND_MARK[at])
        );
        button.addEventListener("mousedown", (event) => event.preventDefault());
        button.addEventListener("click", () => {
          close();
          this.setBound(column2, at);
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
    openChoices(column2, choices, anchor) {
      const box = anchor.getBoundingClientRect();
      const menu = element("div", "fg-menu fg-pick-menu");
      menu.style.left = `${box.left}px`;
      menu.style.top = `${box.bottom + 2}px`;
      const chosen = new Set((this.filters.get(column2.key) ?? "").split("\n").filter(Boolean));
      const close = (event) => {
        if (event && menu.contains(event.target)) return;
        menu.remove();
        document.removeEventListener("mousedown", close);
        document.removeEventListener("keydown", onEscape);
      };
      const onEscape = (event) => {
        if (event.key === "Escape") close();
      };
      const all = element("button", "fg-menu-item", t("\u3059\u3079\u3066"));
      all.type = "button";
      all.addEventListener("mousedown", (event) => event.preventDefault());
      all.addEventListener("click", () => {
        chosen.clear();
        close();
        this.setChoices(column2.key, []);
      });
      menu.append(all, element("div", "fg-menu-rule"));
      for (const choice of choices) {
        const item = element("label", "fg-menu-item fg-pick-item");
        const tick = element("input");
        tick.type = "checkbox";
        tick.checked = chosen.has(choice);
        tick.addEventListener("change", () => {
          if (tick.checked) chosen.add(choice);
          else chosen.delete(choice);
          this.setChoices(column2.key, [...chosen]);
        });
        item.append(tick, element("span", void 0, choice));
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
    setChoices(key, values) {
      if (values.length > 0) this.filters.set(key, values.join("\n"));
      else this.filters.delete(key);
      this.filterFocus = null;
      this.computeVisible();
      this.select(this.row, this.column);
      this.render();
    }
    setFilter(key, text, caret) {
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
    renderFilterRow(tracks) {
      const row = element("div", "fg-row fg-filters");
      row.style.gridTemplateColumns = tracks;
      this.columns.forEach((column2, index) => {
        const cell = element("div", `fg-cell fg-cell-${column2.key}`);
        if (index < this.data.frozen_columns) cell.classList.add("is-frozen");
        const current = this.filters.get(column2.key) ?? "";
        const choices = this.choicesFor(column2);
        if (choices) {
          const chosen = current ? current.split("\n") : [];
          const box = element("button", "fg-filter fg-filter-pick");
          box.type = "button";
          box.dataset["column"] = column2.key;
          if (chosen.length > 0) box.classList.add("is-on");
          box.textContent = chosen.length === 0 ? "" : chosen.length === 1 ? chosen[0] : `${chosen.length}\u4EF6`;
          box.title = chosen.length > 0 ? chosen.join("\u3001") : t("\u9078\u3093\u3067\u7D5E\u308A\u8FBC\u3080");
          box.addEventListener("click", () => this.openChoices(column2, choices, box));
          cell.append(box);
        } else {
          const bound = this.boundFor(column2);
          if (bound) {
            const choices2 = BOUND_CHOICES[column2.key] ?? BOUND_DEFAULT;
            const chip = element("button", "fg-filter-op", BOUND_MARK[bound]);
            chip.type = "button";
            chip.tabIndex = -1;
            chip.title = `\u3044\u307E\u306F\u300C${BOUND_LABEL[bound]}\u300D\u3002\u30AF\u30EA\u30C3\u30AF\u3067 ${choices2.map((at) => BOUND_LABEL[at]).join("\u30FB")} \u304B\u3089\u9078\u3079\u307E\u3059`;
            chip.addEventListener("mousedown", (event) => event.preventDefault());
            chip.addEventListener("click", () => this.openBoundMenu(column2, chip));
            cell.append(chip);
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
          input.placeholder = column2.kind === "name" ? t("\u7D5E\u308A\u8FBC\u307F") : "";
          if (!input.placeholder && !bound) input.classList.add("has-funnel");
          if (bound) {
            input.classList.add("has-op");
            input.title = column2.kind === "date" ? t("20260805\u30FB8/5\u30FB2026-08-05 \u306E\u3069\u308C\u3067\u3082\u3002\u5DE6\u306E\u30DC\u30BF\u30F3\u3067\u5411\u304D\u3092\u5909\u3048\u3089\u308C\u307E\u3059") : t("\u5DE6\u306E\u30DC\u30BF\u30F3\u3067\u300C\u4EE5\u4E0A\u300D\u300C\u4EE5\u4E0B\u300D\u3092\u5207\u308A\u66FF\u3048\u3089\u308C\u307E\u3059");
          }
          input.dataset["column"] = column2.key;
          input.addEventListener("input", (event) => {
            if (event.isComposing) return;
            const digits = normalizeWidth(input.value).trim();
            if (column2.kind === "date" && /^\d{8}$/.test(digits)) {
              input.value = flexibleDate(digits) ?? input.value;
            }
            this.setFilter(column2.key, input.value, input.selectionStart);
          });
          input.addEventListener(
            "compositionend",
            () => this.setFilter(column2.key, input.value, input.selectionStart)
          );
          input.addEventListener("keydown", (event) => event.stopPropagation());
          cell.append(input);
          if (column2.kind === "date") {
            const picker = element("input", "fg-datepicker fg-filter-picker");
            picker.type = "date";
            picker.tabIndex = -1;
            picker.title = t("\u30AB\u30EC\u30F3\u30C0\u30FC\u304B\u3089\u9078\u3076");
            picker.addEventListener("click", () => {
              try {
                picker.showPicker();
              } catch {
              }
            });
            picker.addEventListener("change", () => {
              if (picker.value) this.setFilter(column2.key, picker.value, null);
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
    dayNote(iso) {
      const holiday = this.holidayOn(iso);
      const away = this.data.leaves.filter((leave) => leave.start <= iso && iso <= leave.end).map((leave) => leave.note ? `${leave.assignee}\uFF08${leave.note}\uFF09` : leave.assignee);
      return [
        holiday ? holiday.name || t("\u4F11\u696D\u65E5") : "",
        away.length ? `\u4F11\u307F: ${[...new Set(away)].join("\u3001")}` : ""
      ].filter(Boolean).join("\n");
    }
    /**
     * The business year and quarter a date falls in.
     *
     * A fiscal year starting in April means 2026-04-01 is the first day of
     * 2026年度 Q1, and 2026-03-31 is the last day of 2025年度 Q4.
     */
    quarterOf(date) {
      const start2 = this.data.fiscal_year_start || 4;
      const month = date.getUTCMonth() + 1;
      const offset = (month - start2 + 12) % 12;
      const year = date.getUTCFullYear() - (month < start2 ? 1 : 0);
      const quarter = Math.floor(offset / 3) + 1;
      const label = this.yearLabel(new Date(Date.UTC(year, start2 - 1, 1)));
      return {
        key: `${year}-${quarter}`,
        label: LANG === "en" ? `FY${label} Q${quarter}` : `${label}\u5E74\u5EA6 Q${quarter}`
      };
    }
    monthLabel(date) {
      const month = date.getUTCMonth() + 1;
      const name = MONTH_NAMES_EN[month - 1];
      return LANG === "en" && name ? `${name} ${this.yearLabel(date)}` : `${this.yearLabel(date)}\u5E74${month}\u6708`;
    }
    /**
     * The year as this project writes it.
     *
     * The era table comes from the server rather than the code: an era is
     * announced about a month before it begins, which is no time at all to get a
     * new build onto every machine running this.
     */
    yearLabel(date) {
      const year = date.getUTCFullYear();
      if (!this.data.japanese_era) return String(year);
      const iso = date.toISOString().slice(0, 10);
      const era = this.data.eras.find((entry) => entry.from <= iso);
      if (!era) return String(year);
      const nth = year - Number(era.from.slice(0, 4)) + 1;
      return `${era.name}${nth === 1 ? t("\u5143") : nth}`;
    }
    /**
     * A difference in days, written with the unit it was counted in.
     *
     * The chart measures the gap between two bars in calendar days, so a number
     * counted in working days will not match the pixels beside it. Saying which
     * unit it is costs three characters and removes the whole question.
     */
    varianceText(days) {
      if (days === 0) return "\xB10";
      const unit = LANG === "en" ? ` ${this.workdayBased ? "working days" : "days"}` : this.workdayBased ? "\u55B6\u696D\u65E5" : "\u65E5";
      return days > 0 ? `+${days}${unit}` : `${days}${unit}`;
    }
    /**
     * The same number, said the way the row means it.
     *
     * A summary row's variance is the sum of what its children slipped, not how
     * far this bar moved — the bar's own ends are the earliest and the latest of
     * the subtree, and reading the number off them would be wrong.
     */
    varianceLabel(task, days) {
      const text = this.varianceText(days);
      return task.has_children ? `\u30C8\u30FC\u30BF\u30EB ${text}` : text;
    }
    /** Whether the day count leaves out weekends or holidays. */
    get workdayBased() {
      const counting = this.data.counting;
      return counting.monday || counting.tuesday || counting.wednesday || counting.thursday || counting.friday || counting.saturday || counting.sunday || counting.holidays;
    }
    /** The closed set a column offers, or null when it takes free text. */
    choicesFor(column2) {
      if (column2.kind === "status") return this.data.statuses.map((status) => status.name);
      return column2.kind === "select" ? column2.options?.map((option) => option.value) ?? null : null;
    }
    holidayOn(iso) {
      return this.data.holidays.find((holiday) => holiday.date === iso);
    }
    /** How many rows a folded summary is hiding. */
    hiddenCount(task) {
      const all = this.data.tasks;
      const index = all.indexOf(task);
      let count = 0;
      for (let i = index + 1; i < all.length && (all[i]?.depth ?? 0) > task.depth; i++) count++;
      return count;
    }
    toggleCollapse(task) {
      if (!task.has_children) return;
      if (this.collapsed.has(task.id)) this.collapsed.delete(task.id);
      else this.collapsed.add(task.id);
      saveCollapsed(this.projectId, this.collapsed);
      this.computeVisible();
      this.select(this.row, this.column);
      this.render();
    }
    /**
     * Folds or unfolds the current row.
     *
     * Folding a leaf jumps to its parent instead, which is what pressing "close
     * this" on a row with nothing to close is asking for.
     */
    fold(close) {
      const task = this.selected;
      if (!task) return;
      if (task.has_children && this.collapsed.has(task.id) !== close) {
        this.toggleCollapse(task);
        return;
      }
      if (!close) return;
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
    reveal(taskId) {
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
    get selected() {
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
    get everyColumn() {
      return [
        ...BASE_COLUMNS,
        ...this.data.fields.map((field) => ({
          key: field.id,
          label: field.label,
          kind: field.kind,
          fieldId: field.id,
          options: field.options
        }))
      ];
    }
    /** The lines a bar adds to its tooltip, from the columns the project chose. */
    extraTip(task) {
      if (this.data.tooltip_columns.length === 0) return "";
      const lines = this.data.tooltip_columns.map((key) => this.everyColumn.find((column2) => column2.key === key)).filter((column2) => column2 !== void 0).map((column2) => {
        const value = this.cellDisplay(task, column2).trim();
        return value && value !== "\u2014" ? `${t(column2.label)}: ${value}` : "";
      }).filter(Boolean);
      return lines.length > 0 ? `
${lines.join("\n")}` : "";
    }
    get columns() {
      const hidden = new Set(this.data.hidden_columns);
      const all = [
        // The name column carries the outline, so it is never optional.
        ...BASE_COLUMNS.filter((column2) => column2.kind === "name" || !hidden.has(column2.key)).map(
          // The assignee is a menu of the people on the project rather than free
          // text: 山田 and 山田さん are one person to everyone but a computer.
          (column2) => column2.key === "assignee" ? {
            ...column2,
            kind: "select",
            options: this.data.assignees.map((person) => ({
              value: person.name,
              color: person.color,
              background: person.background
            }))
          } : column2
        ),
        ...this.data.fields.map((field) => ({
          key: field.id,
          label: field.label,
          kind: field.kind,
          fieldId: field.id,
          options: field.options
        }))
      ];
      const order = this.data.column_order;
      const rank = (column2) => {
        const at = order.indexOf(column2.key);
        return at < 0 ? order.length + all.indexOf(column2) : at;
      };
      return all.sort((a, b) => rank(a) - rank(b));
    }
    get selectedColumn() {
      return this.columns[this.column] ?? BASE_COLUMNS[0];
    }
    /**
     * Whether the row is drawn as late.
     *
     * Two different facts, one colour. 予定進捗 is a promise that was not kept;
     * 期限超過 is a date that went by with the work unfinished. Neither is
     * guessed, and a row can be either without being the other.
     */
    behind(task) {
      return task.delayed || task.overdue > 0;
    }
    /** Whether one cell satisfies one filter box. */
    matches(task, column2, needle) {
      const text = this.cellText(task, column2);
      const at = this.boundFor(column2);
      if (at === "behind" || at === "ahead") {
        if (at === "behind") return task.delayed;
        return task.targets.length > 0 && !task.delayed;
      }
      if (this.choicesFor(column2)) {
        const wanted = needle.split("\n").filter(Boolean);
        return wanted.length === 0 || wanted.some((value2) => text === value2);
      }
      const bound = parseBound(needle, at);
      if (!bound) {
        const wanted = needle.toLowerCase();
        return text.toLowerCase().includes(wanted) || column2.kind === "date" && !!text && fullDate(text).toLowerCase().includes(wanted);
      }
      if (!bound.limit) return true;
      const value = normalizeWidth(text).trim();
      if (!value) return false;
      if (column2.kind === "days" || column2.kind === "number" || column2.kind === "variance" || column2.kind === "progress") {
        const left = Number(value.replace(/[^0-9-]/g, ""));
        const right = Number(bound.limit.replace(/[^0-9-]/g, ""));
        if (!Number.isFinite(left) || !Number.isFinite(right)) return false;
        return compare(bound.at, left, right);
      }
      const limit = column2.kind === "date" ? flexibleDate(bound.limit) ?? bound.limit : bound.limit;
      const cut = value.slice(0, limit.length);
      return compare(bound.at, cut, limit);
    }
    cellText(task, column2) {
      if (column2.fieldId) return task.values[column2.fieldId] ?? "";
      switch (column2.key) {
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
          return task.waits.map((span) => `${short(span.start)}\u301C${short(span.end)}`).join(", ");
        case "targets":
          return task.targets.map((target) => `${short(target.date)} ${target.percent}%`).join(", ");
        case "late":
          return this.behind(task) ? "\u9045\u5EF6" : "\u9806\u8ABF";
        default:
          return task.note;
      }
    }
    /** What a cell shows when it is not being edited. */
    cellDisplay(task, column2) {
      const text = this.cellText(task, column2);
      if (column2.key === "progress") return `${task.progress}%`;
      if (column2.kind === "variance") {
        if (!text) return "\u2014";
        return this.varianceText(Number(text));
      }
      if (column2.kind === "date") return text ? fullDate(text) : "\u2014";
      if (column2.kind === "days") return text || "\u2014";
      return text;
    }
    editable(task, column2) {
      return this.refusal(task, column2) === null;
    }
    /**
     * Why this cell cannot be typed into, in the words of this cell.
     *
     * There are four reasons and there used to be one sentence, so opening 遅延
     * on a row with no children answered with a fact about summary rows — true
     * of something else, and no help at all with what was just clicked.
     */
    refusal(task, column2) {
      if (!this.data.can_edit) return t("\u3053\u306E\u30D7\u30ED\u30B8\u30A7\u30AF\u30C8\u306F\u8AAD\u3080\u3060\u3051\u3067\u3059\u3002");
      if (column2.kind === "days") return t("\u65E5\u6570\u306F\u65E5\u4ED8\u304B\u3089\u6570\u3048\u307E\u3059\u3002");
      if (column2.key === "late") return t("\u9045\u5EF6\u306F\u4E88\u5B9A\u9032\u6357\u3068\u5B9F\u9032\u6357\u304B\u3089\u6C7A\u307E\u308A\u307E\u3059\u3002");
      if (column2.kind === "variance") return t("\u5DEE\u7570\u306F\u4E88\u5B9A\u3068\u5B9F\u65BD\u306E\u5DEE\u3067\u3059\u3002");
      if (task.has_children && ROLLED_UP.includes(column2.key)) {
        return t("\u96C6\u8A08\u884C\u306E\u65E5\u4ED8\u3068\u9032\u6357\u306F\u5B50\u30BF\u30B9\u30AF\u304B\u3089\u6C7A\u307E\u308A\u307E\u3059\u3002");
      }
      return null;
    }
    // --- selection -----------------------------------------------------------
    select(row, column2) {
      this.row = clamp(row, 0, Math.max(0, this.tasks.length - 1));
      this.column = clamp(column2, 0, this.columns.length - 1);
    }
    move(rows, columns) {
      this.select(this.row + rows, this.column + columns);
      this.repaintSelection();
    }
    /** Tab and Shift+Tab run past the end of a row onto the next one. */
    step(delta) {
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
    repaintSelection() {
      const grid = this.root.querySelector(".fg-grid");
      if (!grid) {
        this.render();
        return;
      }
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
    markSelection(scroll = false) {
      const grid = this.root.querySelector(".fg-grid");
      if (!grid) return;
      for (const marked of grid.querySelectorAll(".is-selected, .is-current")) {
        marked.classList.remove("is-selected", "is-current");
      }
      const rows = grid.querySelectorAll(".fg-pane-left .fg-row.fg-data");
      const barRows = grid.querySelectorAll(".fg-bar-row");
      const row = rows[this.row - this.first];
      row?.classList.add("is-current");
      barRows[this.row - this.first]?.classList.add("is-current");
      const cell = row?.children[this.column];
      cell?.classList.add("is-selected");
      if (scroll) cell?.scrollIntoView({ block: "nearest", inline: "nearest" });
      if (!grid.contains(document.activeElement) && document.activeElement !== document.body) {
        return;
      }
      const typist = grid.querySelector(".fg-editor.is-typist");
      if (typist && cell) {
        typist.value = "";
        cell.append(typist);
        typist.focus({ preventScroll: true });
      } else if (scroll) {
        grid.focus({ preventScroll: true });
      }
    }
    // --- editing -------------------------------------------------------------
    startEdit(seed) {
      const task = this.selected;
      if (!task) return;
      const refused = this.refusal(task, this.selectedColumn);
      if (refused !== null) {
        this.fail(refused);
        return;
      }
      if (this.selectedColumn.key === "waits") {
        this.openWaits(task);
        return;
      }
      if (this.selectedColumn.key === "targets") {
        this.openTargets(task);
        return;
      }
      this.editing = true;
      this.seed = seed;
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
    openLeaves() {
      const dialog = element("dialog", "fg-dialog");
      const rows = element("div", "fg-dialog-rows");
      const addRow = (leave) => {
        const row = element("div", "fg-dialog-row");
        const kind = element("select", "fg-dialog-kind");
        for (const [value, label] of [
          ["off", t("\u4F11\u307F")],
          // A day worked on a weekend or a holiday: counted rather than skipped.
          ["on", t("\u51FA\u793E")]
        ]) {
          const option = element("option", void 0, label);
          option.value = value;
          kind.append(option);
        }
        kind.value = leave?.kind === "on" ? "on" : "off";
        const who = element("select", "fg-dialog-who");
        who.append(element("option", void 0, ""));
        for (const person of this.data.assignees) {
          const option = element("option", void 0, person.name);
          option.value = person.name;
          who.append(option);
        }
        who.value = leave?.assignee ?? "";
        const start2 = element("input", "fg-dialog-date");
        start2.type = "date";
        start2.value = leave?.start ?? "";
        const end = element("input", "fg-dialog-date");
        end.type = "date";
        end.value = leave?.end ?? "";
        const note = element("input", "fg-dialog-reason");
        note.type = "text";
        note.placeholder = t("\u30E1\u30E2\uFF08\u4EFB\u610F\uFF09");
        note.value = leave?.note ?? "";
        const remove = element("button", "fg-dialog-remove", t("\u524A\u9664"));
        remove.type = "button";
        remove.addEventListener("click", () => row.remove());
        row.append(who, kind, start2, element("span", "fg-dialog-tilde", "\u301C"), end, note, remove);
        rows.append(row);
        return who;
      };
      for (const leave of this.data.leaves) addRow(leave);
      if (this.data.leaves.length === 0) addRow();
      const add = element("button", "fg-dialog-add", t("\uFF0B \u4F11\u6687\u3092\u8FFD\u52A0"));
      add.type = "button";
      add.addEventListener("click", () => addRow().focus());
      const save = element("button", "fg-dialog-save", t("\u4FDD\u5B58"));
      const cancel = element("button", "fg-dialog-cancel", t("\u30AD\u30E3\u30F3\u30BB\u30EB"));
      cancel.type = "button";
      cancel.addEventListener("click", () => dialog.close());
      save.addEventListener("click", async () => {
        const leaves = [...rows.querySelectorAll(".fg-dialog-row")].map((row) => {
          const [start2, end] = [...row.querySelectorAll(".fg-dialog-date")];
          return {
            assignee: row.querySelector(".fg-dialog-who")?.value ?? "",
            kind: row.querySelector(".fg-dialog-kind")?.value ?? "off",
            start: start2?.value ?? "",
            end: end?.value ?? "",
            note: row.querySelector(".fg-dialog-reason")?.value ?? ""
          };
        }).filter((leave) => leave.assignee && leave.start && leave.end);
        dialog.close();
        await this.send(`/api/projects/${encodeURIComponent(this.projectId)}/leaves`, {
          method: "POST",
          body: { leaves }
        });
      });
      dialog.append(
        element("h2", "fg-dialog-title", t("\u62C5\u5F53\u8005\u306E\u4F11\u6687 / \u51FA\u793E")),
        element(
          "p",
          "fg-dialog-help",
          t("\u4F11\u307F\u306E\u65E5\u306F\u305D\u306E\u4EBA\u306E\u30BF\u30B9\u30AF\u306E\u65E5\u6570\u306B\u3082\u9045\u308C\u306E\u5224\u5B9A\u306B\u3082\u5165\u308A\u307E\u305B\u3093\u3002\u9006\u306B\u300C\u51FA\u793E\u300D\u306F\u3001\u571F\u65E5\u795D\u3067\u3082\u305D\u306E\u65E5\u3092\u6570\u3048\u307E\u3059\u3002") + t("\u4E88\u5B9A\u306F\u4EBA\u306B\u3064\u304F\u306E\u3067\u3001\u3053\u3053\u3067\u306E\u767B\u9332\u306F\u305D\u306E\u4EBA\u304C\u51FA\u3066\u3044\u308B\u5168\u90E8\u306E\u30D7\u30ED\u30B8\u30A7\u30AF\u30C8\u306B\u52B9\u304D\u307E\u3059\u3002")
        ),
        rows,
        add
      );
      const buttons = element("div", "fg-dialog-buttons");
      buttons.append(cancel, save);
      dialog.append(buttons);
      dialog.addEventListener("keydown", (event) => event.stopPropagation());
      dialog.addEventListener("close", () => {
        dialog.remove();
        this.root.querySelector(".fg-grid")?.focus({ preventScroll: true });
      });
      document.body.append(dialog);
      dialog.showModal();
      rows.querySelector(".fg-dialog-who")?.focus();
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
    openTargets(task) {
      const dialog = element("dialog", "fg-dialog");
      const rows = element("div", "fg-dialog-rows");
      const addRow = (target) => {
        const row = element("div", "fg-dialog-row");
        const date = element("input", "fg-dialog-date");
        date.type = "date";
        date.required = true;
        date.value = target?.date ?? "";
        const percent = element("input", "fg-dialog-percent");
        percent.type = "number";
        percent.min = "0";
        percent.max = "100";
        percent.step = "5";
        percent.value = target === void 0 ? "" : String(target.percent);
        const remove = element("button", "fg-dialog-remove", t("\u524A\u9664"));
        remove.type = "button";
        remove.addEventListener("click", () => row.remove());
        row.append(
          date,
          element("span", "fg-dialog-tilde", t("\u307E\u3067\u306B")),
          percent,
          element("span", "fg-dialog-unit", "%"),
          remove
        );
        rows.append(row);
        return date;
      };
      for (const target of task.targets) addRow(target);
      if (task.targets.length === 0) addRow();
      const add = element("button", "fg-dialog-add", t("\uFF0B \u4E88\u5B9A\u3092\u8FFD\u52A0"));
      add.type = "button";
      add.addEventListener("click", () => addRow().focus());
      const save = element("button", "fg-dialog-save", t("\u4FDD\u5B58"));
      const cancel = element("button", "fg-dialog-cancel", t("\u30AD\u30E3\u30F3\u30BB\u30EB"));
      cancel.type = "button";
      cancel.addEventListener("click", () => dialog.close());
      save.addEventListener("click", async () => {
        const lines = [];
        for (const row of rows.querySelectorAll(".fg-dialog-row")) {
          const date = row.querySelector(".fg-dialog-date");
          const percent = row.querySelector(".fg-dialog-percent");
          if (!date?.value || !percent?.value) continue;
          lines.push(`${date.value} ${percent.value}%`);
        }
        dialog.close();
        await this.send(`/api/projects/${encodeURIComponent(this.projectId)}/tasks/${task.id}`, {
          method: "POST",
          body: { field: "targets", value: lines.join("\n") },
          follow: task.id
        });
      });
      dialog.append(
        element("h2", "fg-dialog-title", `\u4E88\u5B9A\u9032\u6357 \u2014 ${task.name || t("\uFF08\u7121\u984C\uFF09")}`),
        element(
          "p",
          "fg-dialog-help",
          t("\u305D\u306E\u65E5\u3092\u904E\u304E\u3066\u3082\u5B9F\u9032\u6357\u304C\u5C4A\u3044\u3066\u3044\u306A\u3051\u308C\u3070\u9045\u308C\u306B\u306A\u308A\u307E\u3059\u3002\u9593\u306E\u65E5\u306F\u5224\u5B9A\u3057\u307E\u305B\u3093\u3002\u5165\u308C\u306A\u3051\u308C\u3070\u3001\u3053\u306E\u884C\u306F\u9032\u6357\u3067\u306F\u9045\u308C\u306B\u306A\u308A\u307E\u305B\u3093\u3002")
        ),
        rows,
        add
      );
      const buttons = element("div", "fg-dialog-buttons");
      buttons.append(cancel, save);
      dialog.append(buttons);
      dialog.addEventListener("keydown", (event) => event.stopPropagation());
      dialog.addEventListener("close", () => {
        dialog.remove();
        this.root.querySelector(".fg-grid")?.focus({ preventScroll: true });
      });
      document.body.append(dialog);
      dialog.showModal();
      rows.querySelector(".fg-dialog-date")?.focus();
    }
    /**
     * Editing the waits.
     *
     * A cell holds one line, and a wait is a list of ranges with reasons — asking
     * for that as text works for whoever wrote the parser and for nobody else.
     * The dialog is the interface; the text form stays as what it sends.
     */
    openWaits(task) {
      const dialog = element("dialog", "fg-dialog");
      const rows = element("div", "fg-dialog-rows");
      const addRow = (wait) => {
        const row = element("div", "fg-dialog-row");
        const start2 = element("input", "fg-dialog-date");
        start2.type = "date";
        start2.required = true;
        start2.value = wait?.start ?? "";
        const end = element("input", "fg-dialog-date");
        end.type = "date";
        end.value = wait && !wait.open ? wait.end : "";
        end.placeholder = t("\u7D99\u7D9A\u4E2D");
        const reason = element("input", "fg-dialog-reason");
        reason.type = "text";
        reason.placeholder = t("\u7406\u7531\uFF08\u4EFB\u610F\uFF09");
        reason.value = wait?.reason ?? "";
        const remove = element("button", "fg-dialog-remove", t("\u524A\u9664"));
        remove.type = "button";
        remove.addEventListener("click", () => row.remove());
        row.append(start2, element("span", "fg-dialog-tilde", "\u301C"), end, reason, remove);
        rows.append(row);
        return start2;
      };
      for (const wait of task.waits) addRow(wait);
      if (task.waits.length === 0) addRow();
      const add = element("button", "fg-dialog-add", t("\uFF0B \u671F\u9593\u3092\u8FFD\u52A0"));
      add.type = "button";
      add.addEventListener("click", () => addRow().focus());
      const save = element("button", "fg-dialog-save", t("\u4FDD\u5B58"));
      const cancel = element("button", "fg-dialog-cancel", t("\u30AD\u30E3\u30F3\u30BB\u30EB"));
      cancel.type = "button";
      cancel.addEventListener("click", () => dialog.close());
      save.addEventListener("click", async () => {
        const lines = [];
        for (const row of rows.querySelectorAll(".fg-dialog-row")) {
          const [start2, end] = [...row.querySelectorAll(".fg-dialog-date")];
          const reason = row.querySelector(".fg-dialog-reason")?.value.trim() ?? "";
          if (!start2?.value) continue;
          const range = `${start2.value}\u301C${end?.value ?? ""}`;
          lines.push(reason ? `${range} ${reason}` : range);
        }
        dialog.close();
        await this.send(`/api/projects/${encodeURIComponent(this.projectId)}/tasks/${task.id}`, {
          method: "POST",
          body: { field: "waits", value: lines.join("\n") },
          follow: task.id
        });
      });
      dialog.append(
        element("h2", "fg-dialog-title", `\u5F85\u3061 \u2014 ${task.name || t("\uFF08\u7121\u984C\uFF09")}`),
        element(
          "p",
          "fg-dialog-help",
          t("\u7D42\u308F\u308A\u3092\u7A7A\u306B\u3059\u308B\u3068\u300C\u307E\u3060\u5F85\u3063\u3066\u3044\u308B\u300D\u306B\u306A\u308A\u3001\u4ECA\u65E5\u307E\u3067\u6570\u3048\u7D9A\u3051\u307E\u3059\u3002\u5F85\u3061\u306E\u65E5\u6570\u306F\u65E5\u6570\u304B\u3089\u3082\u9045\u308C\u306E\u5224\u5B9A\u304B\u3089\u3082\u5916\u308C\u307E\u3059\u3002")
        ),
        rows,
        add
      );
      const buttons = element("div", "fg-dialog-buttons");
      buttons.append(cancel, save);
      dialog.append(buttons);
      dialog.addEventListener("keydown", (event) => event.stopPropagation());
      dialog.addEventListener("close", () => {
        dialog.remove();
        this.root.querySelector(".fg-grid")?.focus({ preventScroll: true });
      });
      document.body.append(dialog);
      dialog.showModal();
      rows.querySelector(".fg-dialog-date")?.focus();
    }
    cancelEdit() {
      this.editing = false;
      this.seed = null;
      if (this.repaintRow(this.row)) this.restoreFocus();
      else this.render();
    }
    async commitEdit(raw, after) {
      const task = this.selected;
      const column2 = this.selectedColumn;
      const value = column2.kind === "date" ? flexibleDate(raw) ?? normalizeWidth(raw).trim() : column2.kind === "progress" || column2.kind === "number" ? normalizeWidth(raw).trim() : raw;
      this.editing = false;
      this.seed = null;
      if (after === "down") this.select(this.row + 1, this.column);
      if (after === "right") this.step(1);
      if (!task || value === this.cellText(task, column2)) {
        this.render();
        return;
      }
      const rollback = structuredClone(this.data);
      this.applyLocally(task, column2, value);
      this.render();
      await this.send(`/api/projects/${encodeURIComponent(this.projectId)}/tasks/${task.id}`, {
        method: "POST",
        body: column2.fieldId ? { field: "custom", field_id: column2.fieldId, value } : { field: column2.key, value },
        rollback
      });
    }
    applyLocally(task, column2, value) {
      if (column2.fieldId) {
        task.values[column2.fieldId] = value;
        return;
      }
      switch (column2.key) {
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
    async commitAndInsert(raw) {
      await this.commitEdit(raw, "stay");
      await this.insertRow();
    }
    async insertRow() {
      if (!this.data.can_edit) return;
      const after = this.selected?.id ?? null;
      const result = await this.send(
        `/api/projects/${encodeURIComponent(this.projectId)}/tasks`,
        { method: "POST", body: { after } }
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
    async moveRow(action) {
      const task = this.selected;
      if (!task || !this.data.can_edit) return;
      this.notice = null;
      const was = this.spotOf(task.id);
      const result = await this.send(
        `/api/projects/${encodeURIComponent(this.projectId)}/tasks/${task.id}/move`,
        { method: "POST", body: { action }, follow: task.id, was: was ?? void 0 }
      );
      if (result?.note) this.showNotice(result.note);
      else this.render();
    }
    async deleteRow() {
      const task = this.selected;
      if (!task || !this.data.can_edit) return;
      const label = task.name || t("\u7121\u984C\u306E\u30BF\u30B9\u30AF");
      const question = task.has_children ? `\u300C${label}\u300D\u3068\u3001\u305D\u306E\u5B50\u30BF\u30B9\u30AF\u3092\u3059\u3079\u3066\u524A\u9664\u3057\u307E\u3059\u3002\u3088\u308D\u3057\u3044\u3067\u3059\u304B\uFF1F` : `\u300C${label}\u300D\u3092\u524A\u9664\u3057\u307E\u3059\u3002\u3088\u308D\u3057\u3044\u3067\u3059\u304B\uFF1F`;
      if (!window.confirm(question)) return;
      await this.send(
        `/api/projects/${encodeURIComponent(this.projectId)}/tasks/${task.id}`,
        { method: "DELETE" }
      );
      this.select(this.row, this.column);
      this.render();
    }
    // --- server --------------------------------------------------------------
    async send(url, options) {
      const before = options.rollback ?? this.data;
      this.busy = true;
      let shape = null;
      let rows = [];
      let touched = null;
      try {
        const headers = { "x-fugantt-client": CLIENT_ID };
        if (options.body) headers["content-type"] = "application/json";
        const response = await fetch(url, {
          method: options.method,
          headers,
          body: options.body ? JSON.stringify(options.body) : void 0
        });
        if (!response.ok) {
          this.setData(before);
          this.fail(await this.reason(response));
          return null;
        }
        const result = await response.json();
        if (result.patch) {
          const drew = this.applyPatch(result.patch);
          if (drew === null) await this.refetch();
          else touched = drew;
        } else if (result.grid) {
          shape = this.shape();
          rows = this.tasks.map((task) => JSON.stringify(task));
          this.setData(result.grid);
        }
        this.error = null;
        this.remember(url, options, before, result);
        if (options.follow) this.reveal(options.follow);
        const moved = options.follow ? this.tasks.findIndex((task) => task.id === options.follow) : -1;
        this.select(moved >= 0 ? moved : this.row, this.column);
        return result;
      } catch {
        this.setData(before);
        this.fail(t("\u4FDD\u5B58\u3067\u304D\u307E\u305B\u3093\u3067\u3057\u305F\u3002\u63A5\u7D9A\u3092\u78BA\u8A8D\u3057\u3066\u304F\u3060\u3055\u3044\u3002"));
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
    applyPatch(patch) {
      const wasVisible = this.tasks.length;
      const structural = patch.range_start !== this.data.range_start || patch.range_end !== this.data.range_end || (patch.removed?.length ?? 0) > 0;
      if (patch.removed?.length) {
        const gone = new Set(patch.removed);
        this.data.tasks = this.data.tasks.filter((task) => !gone.has(task.id));
      }
      if (patch.moved && !this.carry(patch.moved)) return null;
      const at = new Map(this.data.tasks.map((task, index) => [task.id, index]));
      const fresh = [];
      for (const row of patch.rows) {
        const index = at.get(row.id);
        if (index === void 0) fresh.push(row);
        else this.data.tasks[index] = row;
      }
      if (fresh.length > 0) {
        const behind = patch.after ? this.data.tasks.findIndex((task) => task.id === patch.after) : -1;
        if (patch.after !== void 0 && behind < 0) return null;
        this.data.tasks.splice(behind + 1, 0, ...fresh);
      }
      this.data.revision = patch.revision;
      this.data.range_start = patch.range_start;
      this.data.range_end = patch.range_end;
      if (this.data.tasks.length !== patch.total) return null;
      this.computeVisible();
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
    carry(moved) {
      const at = this.data.tasks.findIndex((task) => task.id === moved.id);
      if (at < 0) return false;
      const row = this.data.tasks[at];
      if (!row) return false;
      let end = at + 1;
      while (end < this.data.tasks.length && (this.data.tasks[end]?.depth ?? 0) > row.depth) end++;
      const subtree = this.data.tasks.splice(at, end - at);
      const shift = moved.depth - row.depth;
      for (const task of subtree) task.depth += shift;
      const behind = moved.after ? this.data.tasks.findIndex((task) => task.id === moved.after) : -1;
      if (moved.after !== void 0 && behind < 0) return false;
      this.data.tasks.splice(behind + 1, 0, ...subtree);
      return true;
    }
    /** The plan as the server has it, when a patch could not be trusted. */
    async refetch() {
      const response = await fetch(
        `/api/projects/${encodeURIComponent(this.projectId)}/grid`,
        { headers: { accept: "application/json" } }
      );
      if (response.ok) this.setData(await response.json());
    }
    /** Draws the rows a patch touched, and leaves the rest of the table alone. */
    repaintRows(ids) {
      const showing = this.root.querySelector(".fg-error") !== null;
      if (ids.length === 0 || showing !== (this.error !== null)) {
        this.render();
        return;
      }
      for (const id of ids) {
        const index = this.tasks.findIndex((task) => task.id === id);
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
    remember(url, options, before, result) {
      if (this.replaying || options.method === "GET") return;
      const body = options.body;
      const taskId = decodeURIComponent(/\/tasks\/([^/?#]+)$/.exec(url)?.[1] ?? "");
      if (options.was && result.patch?.moved) {
        const to = this.spotOf(result.patch.moved.id);
        if (to) {
          this.done.push({ kind: "move", taskId: result.patch.moved.id, from: options.was, to });
          this.undone = [];
        }
        return;
      }
      if (url.endsWith("/tasks") && result.task_id) {
        const at = this.spotOf(result.task_id);
        if (at) {
          this.done.push({ kind: "add", taskId: result.task_id, at });
          this.undone = [];
        }
        return;
      }
      if (!body?.field || !taskId) {
        this.done.push({ kind: "barrier" });
        this.undone = [];
        return;
      }
      const was = before.tasks.find((task) => task.id === taskId);
      const now = (result.grid?.tasks ?? result.patch?.rows ?? []).find(
        (task) => task.id === taskId
      );
      if (!was || !now) return;
      const field = body.field;
      const fieldId = body.field_id;
      const step = {
        kind: "cell",
        taskId,
        field,
        fieldId,
        before: {
          send: this.sendableValue(was, field, fieldId),
          stored: this.storedValue(was, field, fieldId)
        },
        after: {
          send: this.sendableValue(now, field, fieldId),
          stored: this.storedValue(now, field, fieldId)
        }
      };
      if (step.before.stored === step.after.stored) return;
      this.done.push(step);
      this.undone = [];
    }
    /**
     * What one field of a task holds, as the server stores it.
     *
     * This is what `expect` is compared against, so it has to match the column
     * exactly — not what the cell shows a person.
     */
    storedValue(task, field, fieldId) {
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
          return task.waits.map((wait) => {
            const range = `${wait.start}/${wait.open ? "" : wait.end}`;
            return wait.reason ? `${range}:${wait.reason}` : range;
          }).join("\n");
        case "targets":
          return task.targets.map((target) => `${target.date}/${target.percent}`).join("\n");
        case "custom":
          return fieldId && task.values[fieldId] || "";
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
    sendableValue(task, field, fieldId) {
      if (field !== "waits") return this.storedValue(task, field, fieldId);
      return task.waits.map((wait) => {
        const range = `${wait.start}\u301C${wait.open ? "" : wait.end}`;
        return wait.reason ? `${range} ${wait.reason}` : range;
      }).join("\n");
    }
    /**
     * Puts back the last change this tab made.
     *
     * The value it is putting back travels with what it expects to find, so a
     * cell somebody else has since touched is refused rather than quietly
     * overwritten. Undo is for taking back your own work.
     */
    async replay(direction) {
      const from = direction === "undo" ? this.done : this.undone;
      const step = from.pop();
      if (!step) {
        this.fail(direction === "undo" ? t("\u53D6\u308A\u6D88\u305B\u308B\u64CD\u4F5C\u304C\u3042\u308A\u307E\u305B\u3093\u3002") : t("\u3084\u308A\u76F4\u305B\u308B\u64CD\u4F5C\u304C\u3042\u308A\u307E\u305B\u3093\u3002"));
        return;
      }
      const done = step.kind === "cell" ? await this.replayCell(step, direction) : step.kind === "move" ? await this.replayMove(step, direction) : step.kind === "add" ? await this.replayAdd(step, direction) : this.refuse(t("\u884C\u306E\u524A\u9664\u306F\u53D6\u308A\u6D88\u305B\u307E\u305B\u3093\u3002\u3082\u3046\u4E00\u5EA6\u62BC\u3059\u3068\u3001\u305D\u306E\u524D\u306E\u5909\u66F4\u3092\u53D6\u308A\u6D88\u3057\u307E\u3059\u3002"));
      if (!done) return;
      (direction === "undo" ? this.undone : this.done).push(step);
    }
    /** Says why nothing happened, and answers "not done" for the caller. */
    refuse(why) {
      this.fail(why);
      return false;
    }
    /** Puts one cell back to the value on the other side of the change. */
    async replayCell(step, direction) {
      if (!this.data.tasks.some((task) => task.id === step.taskId)) {
        this.fail(t("\u305D\u306E\u884C\u306F\u3082\u3046\u3042\u308A\u307E\u305B\u3093\u3002"));
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
          follow: step.taskId
        }
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
    async replayMove(step, direction) {
      if (!this.data.tasks.some((task) => task.id === step.taskId)) {
        this.fail(t("\u305D\u306E\u884C\u306F\u3082\u3046\u3042\u308A\u307E\u305B\u3093\u3002"));
        return false;
      }
      const now = this.spotOf(step.taskId);
      const held = direction === "undo" ? step.to : step.from;
      if (!now || now.parent !== held.parent || now.after !== held.after) {
        this.fail(t("\u305D\u306E\u884C\u306F\u8AB0\u304B\u304C\u52D5\u304B\u3057\u307E\u3057\u305F\u3002"));
        return false;
      }
      const target = direction === "undo" ? step.from : step.to;
      this.replaying = true;
      const result = await this.send(
        `/api/projects/${encodeURIComponent(this.projectId)}/tasks/${step.taskId}/place`,
        { method: "POST", body: target, follow: step.taskId, was: now }
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
    async replayAdd(step, direction) {
      if (direction === "redo") {
        this.replaying = true;
        const result2 = await this.send(
          `/api/projects/${encodeURIComponent(this.projectId)}/tasks`,
          { method: "POST", body: { after: step.at.after ?? step.at.parent } }
        );
        this.replaying = false;
        if (!result2?.task_id) return false;
        if (step.at.after === null && step.at.parent !== null) {
          this.replaying = true;
          await this.send(
            `/api/projects/${encodeURIComponent(this.projectId)}/tasks/${result2.task_id}/place`,
            { method: "POST", body: step.at, follow: result2.task_id }
          );
          this.replaying = false;
        }
        this.rename(step.taskId, result2.task_id);
        return true;
      }
      const row = this.data.tasks.find((task) => task.id === step.taskId);
      if (!row) {
        this.fail(t("\u305D\u306E\u884C\u306F\u3082\u3046\u3042\u308A\u307E\u305B\u3093\u3002"));
        return false;
      }
      if (row.has_children) {
        this.fail(t("\u305D\u306E\u884C\u306B\u306F\u5B50\u30BF\u30B9\u30AF\u304C\u3042\u308B\u306E\u3067\u3001\u53D6\u308A\u6D88\u3057\u3067\u306F\u6D88\u3057\u307E\u305B\u3093\u3002"));
        return false;
      }
      if (this.written(row)) {
        this.fail(t("\u66F8\u304D\u8FBC\u307F\u306E\u3042\u308B\u884C\u306F\u3001\u53D6\u308A\u6D88\u3057\u3067\u306F\u6D88\u3057\u307E\u305B\u3093\u3002"));
        return false;
      }
      this.replaying = true;
      const result = await this.send(
        `/api/projects/${encodeURIComponent(this.projectId)}/tasks/${step.taskId}`,
        { method: "DELETE" }
      );
      this.replaying = false;
      return result !== null;
    }
    /** Whether anything was ever put in this row. */
    written(task) {
      return task.name.trim() !== "" || task.start !== null || task.end !== null || task.actual_start !== null || task.actual_end !== null || task.progress !== 0 || task.assignee.trim() !== "" || task.note.trim() !== "" || Object.values(task.values).some((value) => value.trim() !== "");
    }
    /** Points every step at the id a row came back with. */
    rename(was, now) {
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
    spotOf(id) {
      const at = this.data.tasks.findIndex((task) => task.id === id);
      const row = this.data.tasks[at];
      if (!row) return null;
      let parent = null;
      let after = null;
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
    async reason(response) {
      if (response.status === 403) return t("\u96C6\u8A08\u884C\u306E\u65E5\u4ED8\u3068\u9032\u6357\u306F\u5B50\u30BF\u30B9\u30AF\u304B\u3089\u6C7A\u307E\u308A\u307E\u3059\u3002");
      const text = (await response.text()).replace(/^bad request:\s*/i, "").trim();
      return text || `\u4FDD\u5B58\u3067\u304D\u307E\u305B\u3093\u3067\u3057\u305F\uFF08${response.status}\uFF09\u3002`;
    }
    fail(message) {
      this.error = message;
      this.render();
    }
    /** A passing line that clears itself. Not an error, so not the error bar. */
    showNotice(message) {
      this.notice = message;
      this.paintNotice();
      window.clearTimeout(this.noticeTimer);
      this.noticeTimer = window.setTimeout(() => {
        this.notice = null;
        this.paintNotice();
      }, 4e3);
    }
    /**
     * Puts the passing line on screen, or takes it off.
     *
     * On its own, rather than by drawing the table again: the line says somebody
     * else changed something, and at two thousand rows redrawing everything to
     * say so costs a second — for a sentence that leaves again in four.
     */
    paintNotice() {
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
    onKeyDown(event) {
      if (event.isComposing || event.keyCode === 229) return;
      if (this.editing) {
        this.onEditKeyDown(event);
        return;
      }
      const meta = event.ctrlKey || event.metaKey;
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
          return;
      }
      event.preventDefault();
    }
    onEditKeyDown(event) {
      const input = event.target;
      switch (event.key) {
        case "Enter":
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
    render() {
      const chart = this.root.querySelector(".fg-pane-chart");
      if (chart) this.scrollLeft = chart.scrollLeft;
      const left = this.root.querySelector(".fg-pane-left");
      if (left) this.scrollTop = left.scrollTop;
      const typing = this.filterFocus;
      this.filterFocus = null;
      const parts = [];
      if (this.error) {
        const banner = element("div", "fg-error");
        banner.append(element("span", void 0, this.error));
        const dismiss = element("button", "fg-error-close", t("\u9589\u3058\u308B"));
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
      parts.push(this.data.tasks.length === 0 ? this.renderEmpty() : this.renderGrid());
      parts.push(this.renderToolbar());
      this.root.replaceChildren(...parts);
      const backLeft = this.root.querySelector(".fg-pane-left");
      const backChart = this.root.querySelector(".fg-pane-chart");
      if (backLeft) backLeft.scrollTop = this.scrollTop;
      if (backChart) {
        backChart.scrollTop = this.scrollTop;
        if (this.scrollLeft !== null) backChart.scrollLeft = this.scrollLeft;
      }
      this.updateFilterCount();
      if (typing) {
        const box = this.root.querySelector(
          `.fg-filter[data-column="${typing.key}"]`
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
    measureRows(pane) {
      const row = pane.querySelector(".fg-row.fg-data");
      const index = Number(row?.dataset["index"]);
      if (!row || !Number.isFinite(index)) return false;
      this.rowPixels = row.offsetHeight || this.rowPixels;
      this.rowsTop = row.offsetTop - index * this.rowPixels;
      return true;
    }
    /** Which rows are worth having in the document right now. */
    rowWindow() {
      const total = this.tasks.length;
      const pane = this.root.querySelector(".fg-pane-left");
      if (pane) this.measureRows(pane);
      const top = pane?.scrollTop ?? this.scrollTop;
      const height = pane?.clientHeight || 800;
      const from = Math.floor((top - this.rowsTop) / this.rowPixels);
      const to = Math.ceil((top + height - this.rowsTop) / this.rowPixels);
      const last = Math.min(total - 1, to + _Grid.OVERSCAN);
      return {
        first: Math.min(Math.max(0, from - _Grid.OVERSCAN), Math.max(0, last)),
        last
      };
    }
    /** A block of nothing, holding the place of the rows that are not drawn. */
    spacer(rows) {
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
    renderWindow(force = false) {
      const table = this.root.querySelector(".fg-table");
      const bars = this.root.querySelector(".fg-bars");
      const heading = this.root.querySelector(".fg-heading");
      if (!table || !bars || !heading) return;
      if (this.composing) return;
      const view = this.rowWindow();
      if (!force && view.first === this.first && view.last === this.last) return;
      const editor = this.root.querySelector(".fg-editor:not(.is-typist)");
      const caret = editor ? { from: editor.selectionStart ?? 0, to: editor.selectionEnd ?? 0 } : null;
      if (editor && (this.row < view.first || this.row > view.last)) {
        void this.commitEdit(editor.value, "stay");
        return;
      }
      if (editor) {
        this.moving = true;
        this.root.querySelector(".fg-grid")?.append(editor);
      }
      this.first = view.first;
      this.last = view.last;
      const typist = this.root.querySelector(".fg-editor.is-typist");
      if (typist) this.root.querySelector(".fg-grid")?.append(typist);
      const origin = parseDate(this.data.range_start);
      const tracks = heading.style.gridTemplateColumns;
      const rows = [];
      const drawnBars = [];
      const above = this.spacer(view.first);
      if (above) {
        rows.push(above);
        drawnBars.push(above.cloneNode());
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
        drawnBars.push(below.cloneNode());
      }
      const keptAbove = [...table.children].filter(
        (child) => !child.classList.contains("fg-data") && !child.classList.contains("fg-spacer")
      );
      const columns = bars.querySelector(".fg-columns");
      const today = bars.querySelector(".fg-today");
      table.replaceChildren(...keptAbove, ...rows);
      bars.replaceChildren(
        ...columns ? [columns] : [],
        ...drawnBars,
        ...today ? [today] : []
      );
      this.pinColumns();
      if (this.row >= view.first && this.row <= view.last) this.markSelection();
      if (editor) {
        const cell = this.root.querySelectorAll(".fg-pane-left .fg-row.fg-data")[this.row - view.first]?.children[this.column];
        if (cell) {
          cell.querySelector(".fg-editor.is-typist")?.remove();
          cell.append(editor);
          editor.focus({ preventScroll: true });
          if (caret) editor.setSelectionRange(caret.from, caret.to);
        }
        this.moving = false;
      }
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
    reachRow(index) {
      if (index >= this.first && index <= this.last) return;
      const pane = this.root.querySelector(".fg-pane-left");
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
    repaintRow(index) {
      const grid = this.root.querySelector(".fg-grid");
      const task = this.tasks[index];
      if (!grid || !task) return false;
      if (index < this.first || index > this.last) return true;
      const rows = grid.querySelectorAll(".fg-pane-left .fg-row.fg-data");
      const bars = grid.querySelectorAll(".fg-bar-row");
      const was = rows[index - this.first];
      const wasBar = bars[index - this.first];
      if (!was || !wasBar) return false;
      const row = this.renderRow(task, index);
      row.style.gridTemplateColumns = was.style.gridTemplateColumns;
      was.replaceWith(row);
      wasBar.replaceWith(this.renderBar(task, parseDate(this.data.range_start), index));
      this.pinRow(row);
      return true;
    }
    /** Parks one row's frozen cells where the columns before them end. */
    pinRow(row) {
      const left = this.root.querySelector(".fg-pane-left");
      if (!left) return;
      const heads = [...left.querySelectorAll(".fg-heading .fg-cell")];
      let offset = 0;
      for (let i = 0; i < this.data.frozen_columns && i < heads.length; i++) {
        const cell = row.children[i];
        if (cell instanceof HTMLElement) cell.style.left = `${offset}px`;
        offset += heads[i].getBoundingClientRect().width;
      }
    }
    /**
     * Everything the grid draws apart from the rows' own values.
     *
     * Two of these being equal means the table has the same shape it had a
     * moment ago — same columns, same rows in the same order, same dates across
     * the top — and only the contents of some rows can have moved.
     */
    shape() {
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
        this.tasks.map((task) => task.id).join(" ")
      ].join("|");
    }
    /**
     * Draws the difference between the table on screen and the one in hand.
     *
     * A cell edit comes back as the whole grid, because one value can move every
     * ancestor's dates — but almost always only a few rows actually changed, and
     * the rest of the table is already correct on screen.
     */
    repaintChanged(was, before) {
      const showing = this.root.querySelector(".fg-error") !== null;
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
      this.root.querySelector(".fg-toolbar")?.replaceWith(this.renderToolbar());
      this.updateFilterCount();
      this.restoreFocus();
    }
    renderEmpty() {
      const empty = element("div", "fg-empty");
      empty.append(element("p", void 0, t("\u30BF\u30B9\u30AF\u304C\u3042\u308A\u307E\u305B\u3093\u3002")));
      if (this.data.can_edit) {
        const add = element("button", "fg-button", t("\u6700\u521D\u306E\u30BF\u30B9\u30AF\u3092\u8FFD\u52A0"));
        add.type = "button";
        add.addEventListener("click", () => void this.insertRow());
        empty.append(add);
      }
      return empty;
    }
    renderToolbar() {
      const bar = element("div", "fg-toolbar");
      if (!this.data.can_edit) {
        bar.append(element("span", "fg-hint", t("\u95B2\u89A7\u306E\u307F")));
        return bar;
      }
      const add = element("button", "fg-button", t("\u884C\u3092\u8FFD\u52A0"));
      add.type = "button";
      add.disabled = this.busy;
      add.addEventListener("click", () => void this.insertRow());
      const remove = element("button", "fg-button fg-button-quiet", t("\u884C\u3092\u524A\u9664"));
      remove.type = "button";
      remove.disabled = this.busy || this.tasks.length === 0;
      remove.addEventListener("click", () => void this.deleteRow());
      const leaves = element("button", "fg-button", t("\u62C5\u5F53\u8005\u306E\u4F11\u6687/\u51FA\u793E"));
      leaves.type = "button";
      leaves.title = t("\u8AB0\u304C\u3044\u3064\u4F11\u307F\u3001\u3044\u3064\u51FA\u308B\u304B\u3002\u65E5\u6570\u306E\u6570\u3048\u65B9\u306B\u52B9\u304D\u307E\u3059");
      leaves.addEventListener("click", () => this.openLeaves());
      bar.append(add, remove, leaves);
      return bar;
    }
    renderGrid() {
      const origin = parseDate(this.data.range_start);
      const days = Math.max(1, dayIndex(this.data.range_end, origin) + 1);
      const left = element("div", "fg-pane-left");
      const table = element("div", "fg-table");
      left.append(table);
      const tracks = this.columns.map((column2) => {
        const width = this.data.column_widths[column2.key];
        return width ? `${width}px` : TRACKS[column2.kind];
      }).join(" ");
      const headings = element("div", "fg-row fg-heading");
      headings.style.gridTemplateColumns = tracks;
      this.columns.forEach((column2, index) => {
        const heading = element("div", `fg-cell fg-cell-${column2.key}`, t(column2.label));
        if (this.workdayBased && (column2.kind === "days" || column2.kind === "variance")) {
          heading.title = t("\u571F\u65E5\u30FB\u795D\u65E5\u3092\u9664\u3044\u305F\u55B6\u696D\u65E5\u3067\u6570\u3048\u3066\u3044\u307E\u3059");
        }
        if (index < this.data.frozen_columns) heading.classList.add("is-frozen");
        headings.append(heading);
      });
      table.append(this.renderFilterRow(tracks), headings);
      const chart = element("div", "fg-pane-chart");
      const canvas = element("div", "fg-canvas");
      canvas.style.width = `${days * this.dayWidth}px`;
      canvas.append(this.renderHeader(origin, days));
      const body = element("div", "fg-bars");
      const columns = element("div", "fg-columns");
      for (let i = 0; i < days; i++) {
        const date = new Date(origin + i * DAY_MS);
        const iso = date.toISOString().slice(0, 10);
        const holiday = this.holidayOn(iso);
        const column2 = element("div", "fg-column");
        const note = this.dayNote(iso);
        if (note) column2.title = note;
        if (holiday) {
          column2.classList.add("is-holiday");
        } else if (date.getUTCDay() === 6) {
          column2.classList.add("is-saturday");
        } else if (date.getUTCDay() === 0) {
          column2.classList.add("is-sunday");
        }
        columns.append(column2);
      }
      body.append(columns);
      if (this.tasks.length === 0) {
        table.append(element("div", "fg-nomatch", t("\u6761\u4EF6\u306B\u5408\u3046\u884C\u304C\u3042\u308A\u307E\u305B\u3093\u3002")));
      }
      const view = this.rowWindow();
      this.first = view.first;
      this.last = view.last;
      const above = this.spacer(view.first);
      if (above) {
        table.append(above);
        body.append(above.cloneNode());
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
        body.append(below.cloneNode());
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
        this.capPaneWidth = true;
      }
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
      const target = this.scrollLeft ?? (todayIndex >= 0 ? Math.max(0, (todayIndex - 5) * this.dayWidth) : 0);
      requestAnimationFrame(() => {
        chart.scrollLeft = target;
        left.scrollTop = this.scrollTop;
        chart.scrollTop = this.scrollTop;
        if (!left.isConnected) return;
        if (this.measureRows(left)) this.renderWindow();
        if (this.capPaneWidth) {
          const cap = Math.max(320, grid.clientWidth - 480);
          const dates = left.querySelector(".fg-heading .fg-cell-end");
          const wanted = dates ? dates.offsetLeft + dates.offsetWidth : left.scrollWidth;
          grid.style.setProperty(
            "--fg-pane-width",
            `${Math.max(320, Math.min(wanted, cap, left.scrollWidth))}px`
          );
        }
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
    pinColumns() {
      const left = this.root.querySelector(".fg-pane-left");
      if (!left) return;
      const heads = [...left.querySelectorAll(".fg-heading .fg-cell")];
      let offset = 0;
      for (let i = 0; i < this.data.frozen_columns && i < heads.length; i++) {
        for (const cell of left.querySelectorAll(`.fg-row > :nth-child(${i + 1})`)) {
          cell.style.left = `${offset}px`;
        }
        offset += heads[i].getBoundingClientRect().width;
      }
    }
    renderRow(task, index) {
      const row = element("div", "fg-row fg-data");
      row.dataset["index"] = String(index);
      if (this.behind(task)) row.classList.add("is-delayed");
      if (index === this.row) row.classList.add("is-current");
      if (task.background) row.style.setProperty("--fg-row-bg", task.background);
      if (task.color) row.style.setProperty("--fg-row-color", task.color);
      if (task.background || task.color) row.classList.add("is-painted");
      this.columns.forEach((column2, columnIndex) => {
        const cell = element("div", `fg-cell fg-cell-${column2.key}`);
        const isSelected = index === this.row && columnIndex === this.column;
        if (isSelected) cell.classList.add("is-selected");
        if (task.has_children && ROLLED_UP.includes(column2.key)) {
          cell.classList.add("is-derived");
        }
        if (column2.kind === "name") {
          cell.style.paddingLeft = `${12 + task.depth * 16}px`;
          if (task.has_children) cell.classList.add("is-summary");
        }
        if (column2.kind === "name") {
          if (this.data.can_edit) cell.append(this.renderHandle(task, index));
          cell.append(this.renderTwisty(task));
        }
        if (isSelected && this.editing) {
          cell.classList.add("is-editing");
          cell.append(this.renderEditor(task, column2));
          if (column2.kind === "date") cell.append(this.renderDatePicker());
        } else if (column2.key === "late") {
          if (this.behind(task)) {
            const mark = element("span", "fg-late-mark", t("\u9045\u5EF6"));
            mark.title = task.delayed ? t("\u4E88\u5B9A\u9032\u6357\u306B\u5C4A\u3044\u3066\u3044\u307E\u305B\u3093") : t("\u4E88\u5B9A\u7D42\u4E86\u3092\u904E\u304E\u3066\u3001\u5B9F\u65BD\u7D42\u4E86\u304C\u5165\u3063\u3066\u3044\u307E\u305B\u3093");
            cell.append(mark);
          }
        } else if (column2.kind === "name") {
          const text = element("span", "fg-name-text", task.name || t("\uFF08\u7121\u984C\uFF09"));
          if (!task.name) text.classList.add("is-placeholder");
          cell.append(text);
          if (task.has_children && this.collapsed.has(task.id)) {
            cell.append(element("span", "fg-folded", `+${this.hiddenCount(task)}`));
          }
          for (const tag of task.tags) cell.append(element("span", "fg-tag", tag));
        } else if (column2.kind === "status") {
          if (task.status) {
            const pill = element("span", "fg-status", task.status);
            const colour = this.data.statuses.find((status) => status.name === task.status)?.color;
            if (colour) pill.style.background = colour;
            cell.append(pill);
          }
        } else if (column2.kind === "select") {
          const value = this.cellText(task, column2);
          const option = column2.options?.find((entry) => entry.value === value);
          if (value && (option?.color || option?.background)) {
            const pill = element("span", "fg-status", value);
            if (option.color) pill.style.color = option.color;
            if (option.background) pill.style.background = option.background;
            cell.append(pill);
          } else if (value) {
            cell.append(element("span", void 0, value));
          }
        } else if (column2.key === "targets") {
          if (this.editable(task, column2)) {
            const open = element("button", "fg-wait-edit", task.targets.length === 0 ? "\uFF0B" : "\u270E");
            open.type = "button";
            open.title = t("\u4E88\u5B9A\u9032\u6357\u3092\u767B\u9332\u3059\u308B");
            open.addEventListener("mousedown", (event) => event.stopPropagation());
            open.addEventListener("click", () => {
              this.select(index, columnIndex);
              this.openTargets(task);
            });
            cell.append(open);
          }
          for (const target of task.targets) {
            const pill = element("span", "fg-target-pill", `${short(target.date)} ${target.percent}%`);
            if (target.missed) pill.classList.add("is-missed");
            else if (target.due) pill.classList.add("is-met");
            pill.title = target.missed ? t("\u3053\u306E\u65E5\u307E\u3067\u306B\u5C4A\u3044\u3066\u3044\u307E\u305B\u3093") : target.due ? t("\u9054\u6210") : t("\u3053\u308C\u304B\u3089");
            cell.append(pill);
          }
        } else if (column2.key === "waits") {
          if (this.editable(task, column2)) {
            const open = element("button", "fg-wait-edit", task.waits.length === 0 ? "\uFF0B" : "\u270E");
            open.type = "button";
            open.title = t("\u5F85\u3061\u306E\u671F\u9593\u3092\u767B\u9332\u3059\u308B");
            open.addEventListener("mousedown", (event) => event.stopPropagation());
            open.addEventListener("click", () => {
              this.select(index, columnIndex);
              this.openWaits(task);
            });
            cell.append(open);
          }
          for (const wait of task.waits) {
            const label = wait.open ? `${short(wait.start)}\u301C` : `${short(wait.start)}\u301C${short(wait.end)}`;
            const pill = element("span", "fg-wait-pill", label);
            if (wait.open) pill.classList.add("is-open");
            if (wait.days === 0 && task.start && task.end) pill.classList.add("is-idle");
            if (wait.reason) {
              pill.append(element("span", "fg-wait-why", wait.reason));
            }
            pill.title = [
              wait.reason || t("\u5F85\u3061"),
              wait.open ? t("\uFF08\u7D99\u7D9A\u4E2D\uFF09") : "",
              wait.days === 0 && task.start && task.end ? t("\u4E88\u5B9A\u306E\u671F\u9593\u306E\u5916\u306A\u306E\u3067\u65E5\u6570\u306B\u306F\u52B9\u304D\u307E\u305B\u3093") : ""
            ].filter(Boolean).join(" ");
            cell.append(pill);
          }
        } else if (column2.kind === "variance") {
          const span = element("span", void 0, this.cellDisplay(task, column2));
          if (task.has_children) {
            cell.title = t("\u5B50\u30BF\u30B9\u30AF\u306E\u305A\u308C\u3092\u8DB3\u3057\u305F\u3082\u306E\u3067\u3059\uFF08\u3053\u306E\u884C\u306E\u65E5\u4ED8\u306E\u5DEE\u3067\u306F\u3042\u308A\u307E\u305B\u3093\uFF09");
          }
          const days = column2.key === "start_variance" ? task.start_variance : task.end_variance;
          if (days !== null && days > 0) span.classList.add("is-late");
          if (days !== null && days < 0) span.classList.add("is-early");
          cell.append(span);
        } else {
          cell.append(element("span", void 0, this.cellDisplay(task, column2)));
        }
        if (isSelected && !this.editing) cell.append(this.renderTypist());
        if (columnIndex < this.data.frozen_columns) cell.classList.add("is-frozen");
        cell.addEventListener("mousedown", (event) => {
          if (this.editing) return;
          event.preventDefault();
          const now = Date.now();
          const again = this.lastPress?.row === index && this.lastPress.column === columnIndex && now - this.lastPress.at < 400;
          this.lastPress = { row: index, column: columnIndex, at: now };
          this.select(index, columnIndex);
          this.repaintSelection();
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
    renderHandle(task, index) {
      const handle = element("span", "fg-handle");
      handle.title = t("\u30C9\u30E9\u30C3\u30B0\u3067\u79FB\u52D5");
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
    beginRowDrag(event, task, index) {
      if (event.button !== 0 || !this.data.can_edit) return;
      event.preventDefault();
      event.stopPropagation();
      const grid = this.root.querySelector(".fg-grid");
      const rows = [
        ...this.root.querySelectorAll(".fg-pane-left .fg-row.fg-data")
      ];
      if (!grid) return;
      const subtree = this.subtreeLength(index);
      const excluded = /* @__PURE__ */ new Set();
      for (let i = index; i < index + subtree; i++) excluded.add(i);
      const indicator = element("div", "fg-drop");
      grid.append(indicator);
      rows[index - this.first]?.classList.add("is-dragging");
      const startX = event.clientX;
      let target = null;
      const preview = (move) => {
        let at = this.first + rows.length;
        for (const [i, row] of rows.entries()) {
          const box2 = row.getBoundingClientRect();
          if (move.clientY < box2.top + box2.height / 2) {
            at = this.first + i;
            break;
          }
        }
        while (excluded.has(at) && at < this.tasks.length) at++;
        const previous = this.tasks[at - 1];
        const next = this.tasks[at];
        const maxDepth = previous ? previous.depth + 1 : 0;
        const minDepth = next && !excluded.has(at) ? next.depth : 0;
        const wanted = task.depth + Math.round((move.clientX - startX) / 16);
        const depth = clamp(wanted, Math.min(minDepth, maxDepth), maxDepth);
        target = { at, depth };
        const edge = rows[at - this.first] ?? rows[rows.length - 1];
        if (!edge) return;
        const box = edge.getBoundingClientRect();
        const gridBox = grid.getBoundingClientRect();
        indicator.style.top = `${(at < this.first + rows.length ? box.top : box.bottom) - gridBox.top}px`;
        indicator.style.left = `${12 + depth * 16}px`;
      };
      const finish = async () => {
        detach();
        indicator.remove();
        rows[index - this.first]?.classList.remove("is-dragging");
        if (!target) return;
        const drop = this.dropTarget(target.at, target.depth, excluded);
        if (drop.parent === task.id) return;
        const was = this.spotOf(task.id);
        await this.send(
          `/api/projects/${encodeURIComponent(this.projectId)}/tasks/${task.id}/place`,
          { method: "POST", body: drop, follow: task.id, was: was ?? void 0 }
        );
      };
      const cancel = () => {
        detach();
        indicator.remove();
        rows[index - this.first]?.classList.remove("is-dragging");
      };
      function detach() {
        window.removeEventListener("pointermove", preview);
        window.removeEventListener("pointerup", finish);
        window.removeEventListener("pointercancel", cancel);
      }
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
    beginProgressDrag(event, task, bar, box, index) {
      event.preventDefault();
      event.stopPropagation();
      this.select(index, 0);
      this.repaintSelection();
      const fill = bar.querySelector(".fg-bar-fill");
      let progress = task.progress;
      bar.classList.add("is-dragging");
      const preview = (move) => {
        progress = clamp(Math.round((move.clientX - box.left) / box.width * 100), 0, 100);
        if (fill) fill.style.width = `${progress}%`;
        bar.title = `${task.name} \u2014 ${progress}%`;
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
          follow: task.id
        });
      };
      const cancel = () => {
        detach();
        bar.classList.remove("is-dragging");
        this.render();
      };
      function detach() {
        window.removeEventListener("pointermove", preview);
        window.removeEventListener("pointerup", finish);
        window.removeEventListener("pointercancel", cancel);
      }
      window.addEventListener("pointermove", preview);
      window.addEventListener("pointerup", finish);
      window.addEventListener("pointercancel", cancel);
    }
    /** How many visible rows the row at `index` owns, itself included. */
    subtreeLength(index) {
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
    dropTarget(at, depth, excluded) {
      let parent = null;
      let after = null;
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
    renderTwisty(task) {
      if (!task.has_children) return element("span", "fg-twisty is-leaf");
      const folded = this.collapsed.has(task.id);
      const button = element("button", "fg-twisty");
      button.type = "button";
      button.textContent = folded ? "\u25B6" : "\u25BC";
      button.title = folded ? t("\u5C55\u958B\u3059\u308B") : t("\u6298\u308A\u305F\u305F\u3080");
      button.setAttribute("aria-expanded", folded ? "false" : "true");
      button.tabIndex = -1;
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
    renderTypist() {
      const input = element("input", "fg-editor is-typist");
      input.type = "text";
      input.value = "";
      input.autocomplete = "off";
      input.setAttribute("aria-label", t("\u30BB\u30EB\u306E\u5165\u529B"));
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
    beginTyping(input) {
      const task = this.selected;
      const column2 = this.selectedColumn;
      if (!task || !this.editable(task, column2)) {
        input.value = "";
        this.startEdit(null);
        return;
      }
      if (column2.kind === "status" || column2.kind === "select") {
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
    renderDatePicker() {
      const picker = element("input", "fg-datepicker");
      picker.type = "date";
      picker.tabIndex = -1;
      picker.title = t("\u30AB\u30EC\u30F3\u30C0\u30FC\u304B\u3089\u9078\u3076");
      picker.addEventListener("mousedown", (event) => event.stopPropagation());
      picker.addEventListener("click", () => {
        try {
          picker.showPicker();
        } catch {
        }
      });
      picker.addEventListener("change", () => {
        const editor = picker.closest(".fg-cell")?.querySelector("input.fg-editor");
        if (editor) editor.value = picker.value;
        void this.commitEdit(picker.value, "stay");
      });
      return picker;
    }
    renderEditor(task, column2) {
      const choices = this.choicesFor(column2);
      if (choices) {
        const select = element("select", "fg-editor");
        if (column2.kind !== "status") select.append(element("option", void 0, ""));
        for (const choice of choices) {
          const option = element("option", void 0, choice);
          option.value = choice;
          select.append(option);
        }
        select.value = this.cellText(task, column2);
        requestAnimationFrame(() => {
          try {
            select.showPicker();
          } catch {
          }
        });
        select.addEventListener("change", () => void this.commitEdit(select.value, "stay"));
        select.addEventListener("blur", () => {
          if (this.editing) void this.commitEdit(select.value, "stay");
        });
        return select;
      }
      const input = element("input", "fg-editor");
      input.type = "text";
      const showing = this.cellText(task, column2);
      input.value = this.seed ?? (column2.kind === "date" && showing ? fullDate(showing) : showing);
      if (column2.kind === "suggest" && column2.options?.length) {
        const list = element("datalist");
        list.id = `fg-list-${column2.key}`;
        for (const choice of column2.options) {
          const option = element("option");
          option.value = choice.value;
          list.append(option);
        }
        input.setAttribute("list", list.id);
        this.root.querySelector(".fg-grid")?.append(list);
      }
      if (column2.key === "waits") {
        input.placeholder = t("8/17\u301C8/21 \u4ED6\u90E8\u7F72\uFF08\u7D42\u308F\u308A\u7701\u7565\u3067\u7D99\u7D9A\u4E2D\uFF09");
      }
      if (column2.kind === "date") {
        input.placeholder = "20260805 / 8-5";
        input.inputMode = "numeric";
        input.addEventListener("input", () => {
          const digits = normalizeWidth(input.value).trim();
          if (!/^\d{8}$/.test(digits)) return;
          const iso = flexibleDate(digits);
          if (iso) {
            input.value = iso;
            input.setSelectionRange(iso.length, iso.length);
          }
        });
        input.classList.add("has-picker");
      } else if (column2.kind === "progress" || column2.kind === "number") {
        input.inputMode = "numeric";
      }
      input.addEventListener("blur", (event) => {
        const next = event.relatedTarget;
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
    /**
     * Puts today back on screen.
     *
     * Reading last quarter means scrolling away from today, and scrolling back
     * is a long drag on a plan that runs for a year — the one place a person
     * always wants to get back to, and the only way there was by hand.
     */
    renderTodayButton() {
      const button = element("button", "fg-shows fg-today-button", t("\u4ECA\u65E5"));
      button.type = "button";
      button.tabIndex = -1;
      button.title = t("\u4ECA\u65E5\u306E\u3068\u3053\u308D\u3078\u623B\u308B");
      button.addEventListener("click", () => this.showToday());
      return button;
    }
    /** Scrolls the chart so today sits a few days in from the left. */
    showToday() {
      const chart = this.root.querySelector(".fg-pane-chart");
      const at = dayIndex(this.data.today, parseDate(this.data.range_start));
      if (!chart || at < 0) return;
      chart.scrollLeft = Math.max(0, (at - 5) * this.dayWidth);
      this.scrollLeft = chart.scrollLeft;
    }
    renderShowToggle() {
      const button = element("button", "fg-shows", t("\u8868\u793A"));
      button.type = "button";
      button.tabIndex = -1;
      button.title = t("\u30C1\u30E3\u30FC\u30C8\u306B\u51FA\u3059\u3082\u306E\u3092\u9078\u3073\u307E\u3059");
      const choices = [
        ["start", "\u958B\u59CB\u5DEE\u7570"],
        ["end", "\u7D42\u4E86\u5DEE\u7570"],
        ["worked", "\u5B9F\u4F5C\u696D\u65E5\u6570"],
        ["targets", "\u4E88\u5B9A\u9032\u6357"]
      ];
      if (choices.some(([key]) => !this.shows[key])) button.classList.add("is-on");
      button.addEventListener("mousedown", (event) => event.preventDefault());
      button.addEventListener("click", () => {
        const anchor = button.getBoundingClientRect();
        const menu = element("div", "fg-menu fg-shows-menu");
        menu.style.left = `${anchor.left}px`;
        menu.style.top = `${anchor.bottom + 2}px`;
        const close = (event) => {
          if (event && menu.contains(event.target)) return;
          menu.remove();
          document.removeEventListener("mousedown", close);
          document.removeEventListener("keydown", onEscape);
        };
        const onEscape = (event) => {
          if (event.key === "Escape") close();
        };
        for (const [key, label] of choices) {
          const item = element("label", "fg-menu-item fg-shows-item");
          const box2 = element("input");
          box2.type = "checkbox";
          box2.checked = this.shows[key];
          box2.dataset["shows"] = key;
          box2.addEventListener("change", () => {
            this.shows = { ...this.shows, [key]: box2.checked };
            window.localStorage.setItem(SHOWS_KEY, JSON.stringify(this.shows));
            this.render();
          });
          item.append(box2, element("span", void 0, t(label)));
          menu.append(item);
        }
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
    renderHeader(origin, days) {
      const header = element("div", "fg-chart-header");
      const spacer = element("div", "fg-filter-spacer");
      spacer.append(this.renderShowToggle(), this.renderTodayButton());
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
          if (width >= 56) {
            label.append(element("span", void 0, this.monthLabel(first)));
          }
          months.append(label);
          monthStart = i;
        }
        if (isBoundary && i > quarterStart) {
          const first = new Date(origin + quarterStart * DAY_MS);
          const q = this.quarterOf(first);
          const previous = quarters.lastElementChild;
          if (previous && previous.getAttribute("data-quarter") === q.key) {
            const width = Number(previous.getAttribute("data-days")) + (i - quarterStart);
            previous.setAttribute("data-days", String(width));
            previous.style.width = `${width * this.dayWidth}px`;
          } else {
            const label = element("div", "fg-quarter");
            label.setAttribute("data-quarter", q.key);
            label.setAttribute("data-days", String(i - quarterStart));
            label.style.width = `${(i - quarterStart) * this.dayWidth}px`;
            label.append(element("span", void 0, q.label));
            quarters.append(label);
          }
          quarterStart = i;
        }
        if (i === days) break;
        const day = date.getUTCDay();
        const iso = date.toISOString().slice(0, 10);
        const holiday = this.holidayOn(iso);
        const cell = element("div", "fg-day");
        cell.append(
          element("span", "fg-date", String(date.getUTCDate())),
          element("span", "fg-weekday", weekday(day))
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
    renderBar(task, origin, index) {
      const row = element("div", "fg-bar-row");
      row.addEventListener("mousedown", () => {
        if (this.editing) return;
        this.select(index, this.column);
        this.repaintSelection();
      });
      if (index === this.row) row.classList.add("is-current");
      const span = (from, to) => {
        if (!from || !to) return null;
        const start2 = dayIndex(from, origin);
        const length = Math.max(1, dayIndex(to, origin) - start2 + 1);
        return { start: start2, length };
      };
      const away = task.assignee.trim();
      for (const leave of away ? this.data.leaves : []) {
        if (leave.assignee.trim() !== away || leave.kind === "on") continue;
        const slice = span(leave.start, leave.end);
        if (!slice) continue;
        const cells = element("div", "fg-leave");
        cells.style.left = `${slice.start * this.dayWidth}px`;
        cells.style.width = `${slice.length * this.dayWidth}px`;
        cells.title = `${leave.assignee} \u4F11\u307F${leave.note ? `\uFF08${leave.note}\uFF09` : ""}`;
        row.append(cells);
      }
      for (const wait of task.waits) {
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
          `\u5F85\u3061 ${short(wait.start)}\u301C${wait.open ? "" : short(wait.end)}`,
          wait.reason,
          wait.open ? t("\uFF08\u7D99\u7D9A\u4E2D\uFF09") : ""
        ].filter(Boolean).join(" ");
        row.append(gap);
      }
      const planned = span(task.start, task.end);
      const actual = span(task.actual_start, task.actual_end ?? this.data.today);
      if (!planned && !actual) return row;
      if (planned) {
        const bar = element("div", "fg-bar");
        if (this.behind(task)) bar.classList.add("is-delayed");
        if (task.has_children) bar.classList.add("is-summary");
        bar.classList.add("is-plan");
        bar.dataset["task"] = task.id;
        bar.dataset["progress"] = String(task.progress);
        bar.style.left = `${planned.start * this.dayWidth}px`;
        bar.style.width = `${planned.length * this.dayWidth}px`;
        bar.title = `\u4E88\u5B9A ${task.start} \u301C ${task.end}\uFF08${task.progress}%\uFF09${this.extraTip(task)}`;
        const fill = element("div", "fg-bar-fill");
        fill.style.width = `${task.progress}%`;
        bar.append(fill);
        if (this.editable(task, column("start"))) {
          bar.classList.add("is-draggable");
          const knob = element("span", "fg-grip fg-grip-progress");
          const width = planned.length * this.dayWidth;
          const grip = this.dayWidth;
          knob.style.left = `${clamp(
            width * task.progress / 100 - grip / 2,
            GRIP_WIDTH,
            Math.max(GRIP_WIDTH, width - grip - GRIP_WIDTH)
          )}px`;
          knob.title = t("\u30C9\u30E9\u30C3\u30B0\u3067\u9032\u6357\u3092\u5909\u3048\u308B");
          bar.append(knob);
          bar.append(
            element("span", "fg-grip fg-grip-start"),
            element("span", "fg-grip fg-grip-end")
          );
          bar.addEventListener(
            "pointerdown",
            (event) => this.beginDrag(event, task, bar, planned.start, planned.length, index)
          );
        }
        row.append(bar);
        const due = this.shows.targets ? task.targets.filter((target) => target.due) : [];
        const promised = due.reduce(
          (worst, target) => worst === null || target.percent >= worst.percent ? target : worst,
          null
        );
        if (promised && promised.percent > task.progress) {
          const behind = element("div", "fg-bar-behind");
          behind.style.left = `${task.progress}%`;
          behind.style.width = `${promised.percent - task.progress}%`;
          behind.title = `${promised.date} \u307E\u3067\u306B ${promised.percent}%\uFF08\u3044\u307E ${task.progress}%\uFF09`;
          bar.append(behind);
          const label = element(
            "div",
            "fg-target-label is-missed",
            `${short(promised.date)} ${promised.percent}%`
          );
          label.style.left = `${planned.start * this.dayWidth + planned.length * this.dayWidth * promised.percent / 100 + 4}px`;
          label.title = behind.title;
          row.append(label);
        }
        for (const target of this.shows.targets ? task.targets : []) {
          if (target.due) continue;
          const left = planned.start * this.dayWidth + planned.length * this.dayWidth * target.percent / 100;
          const mark = element("div", "fg-target");
          mark.style.left = `${left}px`;
          mark.title = `${target.date} \u307E\u3067\u306B ${target.percent}%`;
          row.append(mark);
          const label = element(
            "div",
            "fg-target-label",
            `${short(target.date)} ${target.percent}%`
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
        bar.title = (task.actual_end ? `\u5B9F\u65BD ${task.actual_start} \u301C ${task.actual_end}` : `\u5B9F\u65BD ${task.actual_start} \u301C\uFF08\u9032\u884C\u4E2D\uFF09`) + this.extraTip(task);
        if (!task.actual_end) bar.classList.add("is-open");
        if (this.editable(task, column("actual_start"))) {
          bar.classList.add("is-draggable");
          bar.append(
            element("span", "fg-grip fg-grip-start"),
            element("span", "fg-grip fg-grip-end")
          );
          bar.addEventListener(
            "pointerdown",
            (event) => this.beginActualDrag(event, task, bar, actual.start, actual.length, index)
          );
        }
        row.append(bar);
      }
      if (this.shows.worked && actual && task.actual_days !== null && task.actual_days > 0) {
        const worked = element(
          "div",
          "fg-worked",
          LANG === "en" ? `${task.actual_days}d worked` : `\u5B9F\u4F5C\u696D ${task.actual_days}${this.workdayBased ? "\u55B6\u696D\u65E5" : "\u65E5"}`
        );
        worked.style.left = `${(actual.start + actual.length) * this.dayWidth + 6}px`;
        worked.title = t("\u5B9F\u969B\u306B\u52D5\u3044\u305F\u65E5\u6570\u3002\u7D42\u308F\u3063\u3066\u3044\u306A\u3051\u308C\u3070\u4ECA\u65E5\u307E\u3067\u6570\u3048\u307E\u3059");
        row.append(worked);
      }
      if (this.shows.start && task.start_variance !== null && task.start_variance !== 0 && planned && actual) {
        const label = element(
          "div",
          "fg-variance is-start",
          this.varianceLabel(task, task.start_variance)
        );
        label.classList.add(task.start_variance > 0 ? "is-late" : "is-early");
        label.style.right = `calc(100% - ${Math.min(planned.start, actual.start) * this.dayWidth - 6}px)`;
        row.append(label);
      }
      if (this.shows.end && task.end_variance !== null && task.end_variance !== 0 && planned) {
        const ends = [planned, actual].filter((span2) => span2 !== null).map((span2) => span2.start + span2.length);
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
    beginActualDrag(event, task, bar, from, span, index) {
      if (event.button !== 0 || !task.actual_start) return;
      event.preventDefault();
      event.stopPropagation();
      const box = bar.getBoundingClientRect();
      const offset = event.clientX - box.left;
      const open = !task.actual_end;
      const mode = offset >= box.width - GRIP_WIDTH ? "end" : offset <= GRIP_WIDTH ? "start" : "move";
      this.select(index, 0);
      this.repaintSelection();
      const startX = event.clientX;
      const origin = parseDate(this.data.range_start);
      let shift = 0;
      bar.classList.add("is-dragging");
      const preview = (moveEvent) => {
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
        const start2 = shiftDate(origin, from + (mode === "end" ? 0 : shift));
        const end = shiftDate(origin, from + span - 1 + (mode === "start" ? 0 : shift));
        const edit = open && mode !== "end" ? { field: "actual_start", value: start2 } : open ? { field: "actual_end", value: end } : { field: "actual_schedule", value: `${start2}/${end}` };
        await this.send(`/api/projects/${encodeURIComponent(this.projectId)}/tasks/${task.id}`, {
          method: "POST",
          body: edit,
          follow: task.id
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
    beginDrag(event, task, bar, from, span, index) {
      if (event.button !== 0 || !task.start || !task.end) return;
      event.preventDefault();
      event.stopPropagation();
      const box = bar.getBoundingClientRect();
      const offset = event.clientX - box.left;
      const fillEdge = box.width * task.progress / 100;
      const knob = bar.querySelector(".fg-grip-progress")?.getBoundingClientRect();
      const onKnob = !!knob && event.clientX >= knob.left && event.clientX <= knob.right;
      const mode = onKnob ? "progress" : offset <= GRIP_WIDTH ? "start" : offset >= box.width - GRIP_WIDTH ? "end" : Math.abs(offset - fillEdge) <= GRIP_WIDTH ? "progress" : "move";
      if (mode === "progress") {
        this.beginProgressDrag(event, task, bar, box, index);
        return;
      }
      this.select(index, 0);
      this.repaintSelection();
      const startX = event.clientX;
      const origin = parseDate(this.data.range_start);
      let shift = 0;
      bar.classList.add("is-dragging");
      const preview = (moveEvent) => {
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
        const start2 = shiftDate(origin, from + (mode === "end" ? 0 : shift));
        const end = shiftDate(origin, from + span - 1 + (mode === "start" ? 0 : shift));
        await this.send(`/api/projects/${encodeURIComponent(this.projectId)}/tasks/${task.id}`, {
          method: "POST",
          body: { field: "schedule", value: `${start2}/${end}` },
          follow: task.id
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
    syncPanes(left, chart) {
      let mirroring = false;
      const follow = (from, to) => () => {
        if (mirroring) return;
        mirroring = true;
        to.scrollTop = from.scrollTop;
        requestAnimationFrame(() => {
          mirroring = false;
        });
      };
      left.addEventListener("scroll", follow(left, chart));
      chart.addEventListener("scroll", follow(chart, left));
      const track = (pane) => () => {
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
    renderSplitter(grid) {
      const splitter = element("div", "fg-splitter");
      splitter.title = t("\u30C9\u30E9\u30C3\u30B0\u3067\u5E45\u3092\u5909\u3048\u308B");
      splitter.addEventListener("pointerdown", (event) => {
        event.preventDefault();
        const left = grid.querySelector(".fg-pane-left");
        if (!left) return;
        const startX = event.clientX;
        const startWidth = left.getBoundingClientRect().width;
        const drag = (move) => {
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
    openMenu(x, y) {
      const task = this.selected;
      if (!task) return;
      const menu = element("div", "fg-menu");
      menu.style.left = `${x}px`;
      menu.style.top = `${y}px`;
      const close = (event) => {
        if (event && menu.contains(event.target)) return;
        menu.remove();
        document.removeEventListener("mousedown", close);
        document.removeEventListener("keydown", onEscape);
      };
      const onEscape = (event) => {
        if (event.key === "Escape") close();
      };
      const item = (label, shortcut, run) => {
        const button = element("button", "fg-menu-item");
        button.type = "button";
        button.append(element("span", void 0, label), element("kbd", void 0, shortcut));
        button.addEventListener("click", () => {
          close();
          run();
        });
        button.addEventListener("mousedown", (event) => event.preventDefault());
        menu.append(button);
      };
      if (task.has_children) {
        const folded = this.collapsed.has(task.id);
        item(
          folded ? t("\u5C55\u958B\u3059\u308B") : t("\u6298\u308A\u305F\u305F\u3080"),
          folded ? `${MOD}\u2192` : `${MOD}\u2190`,
          () => this.toggleCollapse(task)
        );
        menu.append(element("div", "fg-menu-rule"));
      }
      item(t("\u5B50\u30BF\u30B9\u30AF\u306B\u3059\u308B"), `${ALT}\u2192`, () => void this.moveRow("indent"));
      item(t("\u968E\u5C64\u3092\u623B\u3059"), `${ALT}\u2190`, () => void this.moveRow("outdent"));
      item(t("\u4E0A\u3078\u79FB\u52D5"), `${ALT}\u2191`, () => void this.moveRow("up"));
      item(t("\u4E0B\u3078\u79FB\u52D5"), `${ALT}\u2193`, () => void this.moveRow("down"));
      menu.append(element("div", "fg-menu-rule"));
      item(t("\u4E0B\u306B\u884C\u3092\u8FFD\u52A0"), `${MOD}Enter`, () => void this.insertRow());
      item(t("\u884C\u3092\u524A\u9664"), `${MOD}Delete`, () => void this.deleteRow());
      if (this.editable(task, column("name"))) {
        menu.append(element("div", "fg-menu-rule"));
        menu.append(this.renderPalette(task, close));
      }
      this.root.append(menu);
      const box = menu.getBoundingClientRect();
      if (box.right > window.innerWidth) menu.style.left = `${x - box.width}px`;
      if (box.bottom > window.innerHeight) menu.style.top = `${y - box.height}px`;
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
    renderPalette(task, close) {
      const block = element("div", "fg-palette");
      const row = (label, field, choices, current) => {
        const line = element("div", "fg-palette-row");
        line.append(element("span", "fg-palette-label", label));
        for (const colour of choices) {
          const swatch = element("button", "fg-swatch");
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
      row(t("\u80CC\u666F"), "background", BACKGROUNDS, task.background);
      row(t("\u6587\u5B57"), "color", TEXT_COLOURS, task.color);
      const clear = element("button", "fg-menu-item", t("\u8272\u3092\u6D88\u3059"));
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
    async paint(task, which, colour) {
      const url = `/api/projects/${encodeURIComponent(this.projectId)}/tasks/${task.id}`;
      for (const field of which === "both" ? ["background", "color"] : [which]) {
        await this.send(url, {
          method: "POST",
          body: { field, value: colour },
          follow: task.id
        });
      }
    }
    /** Keeps the keyboard where the user left it across a full re-render. */
    restoreFocus() {
      if (this.editing) {
        const editor = this.root.querySelector(".fg-editor");
        if (editor instanceof HTMLInputElement) {
          editor.focus();
          if (this.seed === null) editor.select();
          else editor.setSelectionRange(editor.value.length, editor.value.length);
        } else {
          editor?.focus();
        }
        return;
      }
      const typist = this.root.querySelector(".fg-editor.is-typist");
      if (typist) typist.focus({ preventScroll: true });
      else this.root.querySelector(".fg-grid")?.focus({ preventScroll: true });
      this.root.querySelector(".fg-cell.is-selected")?.scrollIntoView({ block: "nearest", inline: "nearest" });
    }
  };
  /** Rows kept beyond each edge, so a small scroll redraws nothing. */
  _Grid.OVERSCAN = 8;
  var Grid = _Grid;
  async function start() {
    const root = document.getElementById("fugantt-grid");
    if (!root) return;
    const projectId = root.dataset["project"];
    if (!projectId) return;
    try {
      const response = await fetch(`/api/projects/${encodeURIComponent(projectId)}/grid`, {
        headers: { accept: "application/json" }
      });
      if (!response.ok) throw new Error(`HTTP ${response.status}`);
      new Grid(root, projectId, await response.json());
    } catch (error) {
      root.replaceChildren(
        element("p", "fg-empty", t("\u30B9\u30B1\u30B8\u30E5\u30FC\u30EB\u3092\u8AAD\u307F\u8FBC\u3081\u307E\u305B\u3093\u3067\u3057\u305F\u3002\u518D\u8AAD\u307F\u8FBC\u307F\u3057\u3066\u304F\u3060\u3055\u3044\u3002"))
      );
      console.error("fugantt: failed to load the grid", error);
    }
  }
  void start();
})();
