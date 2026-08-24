#!/usr/bin/env python3
"""公開デモ用のデータを作る。

    python3 web/test/demo.py <database>

情報システム部門が持っていそうな計画を8本、合わせて1万行ほど。日付・進捗・実績・
待ちは「今日」から逆算して辻褄を合わせる——固定の暦で作ると、置いた翌週には
「1年前に始まって誰も触っていない計画」になる。

先に管理者と demo のアカウントがある DB に対して走らせること（画面から作る。
パスワードの hash はここでは作れない）。
"""

import datetime
import random
import sqlite3
import sys
import uuid

DB = sys.argv[1]
TODAY = datetime.date.today()

con = sqlite3.connect(DB)
con.execute("PRAGMA foreign_keys = OFF")
cur = con.cursor()

ADMIN = cur.execute("SELECT id FROM users WHERE base_role = 'admin'").fetchone()[0]
DEMO = cur.execute("SELECT id FROM users WHERE email = 'demo'").fetchone()[0]
HOLIDAYS = {row[0] for row in cur.execute("SELECT date FROM app_holidays")}

random.seed(20260824)

now = int(datetime.datetime.now().timestamp())


def iso(d):
    return d.isoformat()


def workday(d):
    return d.weekday() < 5 and iso(d) not in HOLIDAYS


def next_workday(d):
    while not workday(d):
        d += datetime.timedelta(days=1)
    return d


def add_workdays(d, n):
    """n 営業日後（n=0 ならその日を含む最初の営業日）。"""
    d = next_workday(d)
    for _ in range(n):
        d = next_workday(d + datetime.timedelta(days=1))
    return d


def key(i, width=4):
    out = ""
    for _ in range(width):
        i, digit = divmod(i, 26)
        out = chr(ord("a") + digit) + out
    return out


def uid():
    return str(uuid.uuid4())


# --- 名簿 --------------------------------------------------------------------

PEOPLE = [
    ("佐藤 健一", "#1e3a8a", "#dbeafe"),
    ("鈴木 美咲", "#7c2d12", "#ffedd5"),
    ("高橋 亮", "#3f6212", "#ecfccb"),
    ("田中 由紀", "#831843", "#fce7f3"),
    ("伊藤 大輔", "#164e63", "#cffafe"),
    ("渡辺 彩", "#4c1d95", "#ede9fe"),
    ("山本 拓也", "#713f12", "#fef3c7"),
    ("中村 香織", "#134e4a", "#ccfbf1"),
    ("小林 誠", "#1e293b", "#e2e8f0"),
    ("加藤 直樹", "#7f1d1d", "#fee2e2"),
    ("吉田 麻衣", "#365314", "#f7fee7"),
    ("山口 優", "#0c4a6e", "#e0f2fe"),
    ("松本 和也", "#581c87", "#f3e8ff"),
    ("井上 陽子", "#9a3412", "#fed7aa"),
    ("木村 翔太", "#155e75", "#a5f3fc"),
    ("林 沙織", "#86198f", "#fae8ff"),
    ("清水 宏", "#1f2937", "#f3f4f6"),
    ("山崎 明日香", "#a16207", "#fef9c3"),
    ("森 健太郎", "#14532d", "#dcfce7"),
    ("池田 里奈", "#9f1239", "#ffe4e6"),
    ("橋本 亮介", "#0f766e", "#99f6e4"),
    ("石川 望", "#3730a3", "#e0e7ff"),
    ("前田 敦", "#78350f", "#fde68a"),
    ("外部 ベンダーA", "#334155", "#f1f5f9"),
    ("外部 ベンダーB", "#334155", "#f1f5f9"),
]

cur.execute("DELETE FROM assignees")
cur.executemany("INSERT INTO assignees (name, color, background) VALUES (?, ?, ?)", PEOPLE)

NAMES = [name for name, _, _ in PEOPLE]

STATUSES = [
    (0, "未着手", "#f1f5f9", 0),
    (1, "実施中", "#dbeafe", None),
    (2, "待ち", "#ede9fe", None),
    (3, "完了", "#dcfce7", 100),
    (4, "保留", "#fef3c7", None),
]

# --- 計画の骨格 --------------------------------------------------------------

MODULE_PHASES = [
    ("要件定義", ["業務ヒアリング", "As-Is 業務フロー作成", "To-Be 業務フロー作成",
                  "FIT&GAP 分析", "要件定義書 作成", "要件定義書 レビュー", "指摘対応"]),
    ("基本設計", ["画面一覧 作成", "画面レイアウト設計", "帳票レイアウト設計",
                  "テーブル定義", "外部インターフェース設計", "基本設計書 レビュー", "指摘対応"]),
    ("詳細設計", ["画面詳細設計", "バッチ詳細設計", "IF 詳細設計", "項目定義",
                  "詳細設計書 レビュー", "指摘対応"]),
    ("開発・単体テスト", ["画面 実装", "バッチ 実装", "IF 実装", "単体テスト仕様書 作成",
                          "単体テスト 実施", "単体テスト 指摘対応", "コードレビュー"]),
    ("結合テスト", ["結合テスト仕様書 作成", "テストデータ準備", "結合テスト 実施",
                    "障害対応", "再テスト"]),
    ("総合テスト", ["シナリオ作成", "総合テスト 実施", "性能確認", "障害対応"]),
]

CORE_MODULES = [
    "会計 / 総勘定元帳", "会計 / 買掛金", "会計 / 売掛金", "会計 / 固定資産",
    "会計 / 経費精算", "購買 / 発注", "購買 / 検収", "購買 / 支払",
    "在庫 / 入出庫", "在庫 / 棚卸", "在庫 / 倉庫間移動", "販売 / 受注",
    "販売 / 出荷", "販売 / 請求", "共通 / 認証・権限", "共通 / マスタ管理",
    "共通 / 帳票基盤", "共通 / バッチ基盤", "共通 / 外部連携",
]

EC_MODULES = [
    "商品 / カタログ", "商品 / 検索", "会員 / 登録・ログイン", "会員 / マイページ",
    "注文 / カート", "注文 / 決済", "注文 / 配送", "販促 / クーポン",
    "販促 / ポイント", "管理 / 受注管理", "管理 / 在庫連携", "基盤 / CDN・性能",
]

APP_MODULES = [
    "アプリ / ログイン", "アプリ / ホーム", "アプリ / 通知", "アプリ / 決済",
    "アプリ / 店舗検索", "アプリ / クーポン", "共通 / API", "共通 / 認証基盤",
]

DATA_MODULES = [
    "移行 / 会計データ", "移行 / 販売データ", "移行 / マスタ", "基盤 / DWH 構築",
    "基盤 / ETL", "基盤 / BI ダッシュボード", "運用 / 監視・ジョブ",
]

NET_SITES = ["本社", "大阪支社", "名古屋支社", "福岡支社", "仙台営業所", "関西物流センター",
             "関東物流センター", "研究所"]

NET_PHASES = [
    ("現地調査", ["現地調査 実施", "配線調査", "既設機器 棚卸", "調査報告書 作成"]),
    ("設計", ["ネットワーク構成設計", "IP アドレス設計", "機器選定", "設計レビュー"]),
    ("調達", ["見積取得", "発注", "納品確認"]),
    ("構築・切替", ["機器設定", "事前接続試験", "切替作業", "切替後 確認", "旧機器 撤去"]),
]

ISMS_PHASES = [
    ("準備", ["適用範囲の決定", "推進体制の整備", "キックオフ説明会", "現状分析"]),
    ("文書整備", ["情報セキュリティ方針 策定", "リスクアセスメント手順 策定",
                  "規程類 作成", "様式 整備", "文書レビュー"]),
    ("リスクアセスメント", ["資産台帳 作成", "リスク分析", "リスク対応計画 作成", "残留リスク承認"]),
    ("運用", ["従業員教育", "内部監査員 育成", "運用記録 収集", "是正処置"]),
    ("審査", ["第一段階審査 対応", "指摘事項 是正", "第二段階審査 対応", "認証取得"]),
]

PRODUCT_PHASES = [
    ("企画", ["市場調査", "競合分析", "コンセプト策定", "価格戦略 検討", "事業計画 レビュー"]),
    ("開発連携", ["仕様確定", "試作品 評価", "量産試作 確認", "品質基準 合意"]),
    ("販促準備", ["ネーミング決定", "パッケージデザイン", "販促物 制作", "Web サイト 制作",
                  "プレスリリース 作成", "広告出稿 手配"]),
    ("流通・営業", ["得意先 提案資料 作成", "商談", "初回受注 確定", "物流 手配",
                    "店頭什器 手配"]),
    ("発売", ["発売前 最終確認", "発売日 対応", "初動分析", "追加生産 判断"]),
]

MAINT_KINDS = [
    ("障害対応", ["障害受付", "一次調査", "原因調査", "暫定対応", "恒久対応", "再発防止策 報告"]),
    ("問合せ対応", ["問合せ受付", "内容確認", "回答作成", "回答"]),
    ("改善要望", ["要望 受付", "影響調査", "見積提示", "実装", "リリース"]),
    ("定期作業", ["月次バッチ 立会", "バックアップ 確認", "証跡 収集", "月次報告書 作成"]),
]


class Plan:
    """1つの計画。行を組み立ててから、まとめて書く。"""

    def __init__(self, pid, name, owner, team, memo, fields):
        self.pid = pid
        self.name = name
        self.owner = owner
        self.team = team
        self.memo = memo
        self.fields = fields
        self.rows = []          # (id, parent, sort, name, ...)
        self.counters = {}

    def add(self, parent, name, **kw):
        n = self.counters.get(parent, 0)
        self.counters[parent] = n + 1
        tid = uid()
        self.rows.append(dict(id=tid, parent=parent, sort=key(n), name=name, **kw))
        return tid


def realise(start, end, team, kind="task"):
    """予定に対する実績・進捗・ステータスを、今日から見て辻褄の合う形で返す。"""
    note = ""
    waits = ""
    targets = ""
    actual_start = None
    actual_end = None

    span = (end - start).days + 1

    if end < TODAY - datetime.timedelta(days=3):
        # 終わっているはずの仕事。1割ほどは遅れて終わった。
        status, progress = "完了", 100
        slip_start = random.choice([0, 0, 0, 1, -1, 2])
        slip_end = random.choice([0, 0, 0, 1, 2, 3, 5, 8])
        actual_start = iso(add_workdays(start, 0) + datetime.timedelta(days=slip_start))
        actual_end = iso(end + datetime.timedelta(days=slip_end))
        if slip_end >= 5:
            note = random.choice([
                "先方の確認が遅れたため後ろ倒し。",
                "仕様変更の反映で追加作業が発生。",
                "他案件と要員が重なり着手が遅れた。",
            ])
        # 終わっていない居残り。予定終了を過ぎたまま動いている行が無いと、
        # 遅れの色が一度も出ないデモになる——ただし直近の分だけ。1年前の予定が
        # 開いたままなら、遅れの日数がその1行で数百日になり、合計が嘘になる。
        recent = end > TODAY - datetime.timedelta(days=45)
        if recent and random.random() < 0.30:
            status, progress, actual_end = "実施中", random.randint(45, 95), None
            note = random.choice([
                "残作業あり。今週中に完了予定。",
                "指摘の修正が残っている。",
                "先行タスクの遅れを引きずっている。",
            ])
        # ごく一部は保留のまま止まっている。
        elif recent and random.random() < 0.10:
            status, progress, actual_end = "保留", random.randint(20, 70), None
            note = "上位の方針決定待ちで止めている。"
    elif start > TODAY:
        status, progress = "未着手", 0
        if random.random() < 0.05:
            note = random.choice([
                "着手前に体制を確認すること。",
                "前工程の完了判定後に開始。",
            ])
    else:
        # いま動いている仕事。素直に進んでいるものと、そうでないものを混ぜる。
        elapsed = (TODAY - start).days + 1
        ideal = max(5, min(95, round(elapsed / max(span, 1) * 100)))
        drift = random.choice([-35, -20, -12, -5, 0, 0, 5, 10])
        progress = max(0, min(95, ideal + drift))
        status = "実施中"
        actual_start = iso(add_workdays(start, 0) + datetime.timedelta(days=random.choice([0, 0, 1, 2])))

        if random.random() < 0.10:
            status = "待ち"
            from_day = TODAY - datetime.timedelta(days=random.randint(1, 10))
            to_day = TODAY + datetime.timedelta(days=random.randint(1, 12))
            waits = f"{iso(from_day)}/{iso(to_day)}: " + random.choice([
                "客先の確認待ち", "機器の納品待ち", "他チームの成果物待ち",
                "承認待ち", "先方環境の準備待ち",
            ])
        # 計画が「この日に何%」と言っていた行。半分ほどは割っている。
        if random.random() < 0.12:
            when = start + datetime.timedelta(days=int(span * 0.6))
            promised = min(100, max(20, ideal + random.choice([0, 10, 20])))
            targets = f"{iso(when)}/{promised}"

    return status, progress, actual_start, actual_end, note, waits, targets



SCREEN_WORDS = ["登録画面", "一覧画面", "照会画面", "承認画面", "検索画面", "取込画面",
                "設定画面", "明細画面", "修正画面", "取消画面"]
REPORT_WORDS = ["一覧表", "集計表", "明細表", "伝票", "通知書", "月次報告書"]
BATCH_WORDS = ["日次取込バッチ", "月次締めバッチ", "再計算バッチ", "連携送信バッチ",
               "連携受信バッチ", "データ整理バッチ"]


def features(module, count, seq):
    """モジュールが持つ画面・帳票・バッチ。名前は現場の呼び方に寄せる。"""
    short = module.split(" / ")[-1]
    out = []
    for _ in range(count):
        roll = random.random()
        if roll < 0.6:
            code, word = f"SC-{next(seq):04d}", random.choice(SCREEN_WORDS)
        elif roll < 0.8:
            code, word = f"RP-{next(seq):04d}", random.choice(REPORT_WORDS)
        else:
            code, word = f"BT-{next(seq):04d}", random.choice(BATCH_WORDS)
        out.append(f"{code} {short}{word}")
    return out


MODULE_LEVEL = {
    "要件定義": ["業務ヒアリング", "As-Is 業務フロー作成", "To-Be 業務フロー作成",
                 "FIT&GAP 分析", "要件定義書 作成", "要件定義書 レビュー", "指摘対応"],
    "基本設計": ["画面一覧 作成", "テーブル定義", "外部インターフェース設計",
                 "基本設計書 レビュー", "指摘対応"],
    "詳細設計": ["項目定義", "詳細設計書 レビュー", "指摘対応"],
    "開発・単体テスト": ["開発環境 準備", "コードレビュー", "単体テスト 集計"],
    "結合テスト": ["結合テスト仕様書 作成", "テストデータ準備", "障害対応", "再テスト"],
    "総合テスト": ["シナリオ作成", "総合テスト 実施", "性能確認", "障害対応"],
}

PER_FEATURE = {
    "基本設計": ["設計"],
    "詳細設計": ["詳細設計"],
    "開発・単体テスト": ["実装", "単体テスト"],
    "結合テスト": ["結合テスト"],
}

SOFTWARE_PHASES = ["要件定義", "基本設計", "詳細設計", "開発・単体テスト",
                   "結合テスト", "総合テスト"]


def build_software_plan(plan, modules, first_day, last_day, per_module=(14, 24)):
    """工程 → モジュール → 機能ごとの作業。大きい計画はこの形をしている。"""
    seq = iter(range(1001, 99999))
    catalogue = {m: features(m, random.randint(*per_module), seq) for m in modules}

    total_days = (last_day - first_day).days
    slots = len(SOFTWARE_PHASES)

    for pi, phase_name in enumerate(SOFTWARE_PHASES):
        phase_id = plan.add(None, phase_name)
        p_start = first_day + datetime.timedelta(days=int(total_days * pi / slots * 0.92))
        p_end = min(first_day + datetime.timedelta(days=int(total_days * (pi + 1.15) / slots)), last_day)
        p_span = max((p_end - p_start).days, 20)

        for mi, module in enumerate(modules):
            group_id = plan.add(phase_id, module)
            m_start = p_start + datetime.timedelta(days=int(p_span * mi / max(len(modules), 1) * 0.5))
            room = max(int(p_span * 0.6), 15)

            names = list(MODULE_LEVEL[phase_name])
            for feature in catalogue[module]:
                for verb in PER_FEATURE.get(phase_name, []):
                    names.append(f"{feature} {verb}")

            cursor = m_start
            for name in names:
                length = random.randint(1, 8)
                s = next_workday(cursor + datetime.timedelta(days=random.choice([0, 0, 1, 2])))
                if (s - m_start).days > room:
                    s = next_workday(m_start + datetime.timedelta(days=random.randint(0, room)))
                e = add_workdays(s, length - 1)
                status, progress, a_start, a_end, note, waits, targets = realise(s, e, plan.team)
                plan.add(group_id, name, start=iso(s), end=iso(e), actual_start=a_start,
                         actual_end=a_end, status=status, progress=progress, note=note,
                         waits=waits, targets=targets, assignee=random.choice(plan.team))
                cursor = s + datetime.timedelta(days=random.randint(0, 3))

        m_day = next_workday(p_end)
        done = m_day < TODAY
        plan.add(phase_id, f"◆ {phase_name} 完了判定", start=iso(m_day), end=iso(m_day),
                 actual_start=iso(m_day) if done else None,
                 actual_end=iso(m_day) if done else None,
                 status="完了" if done else "未着手", progress=100 if done else 0,
                 assignee=plan.team[0], color="#7f1d1d", background="#fee2e2")


def build_phase_plan(plan, phases, modules, first_day, last_day, per_leaf=(3, 10)):
    """工程 × モジュールの2段構え。工程は順に、モジュールは工程の中に散らす。"""
    total_days = (last_day - first_day).days
    slots = len(phases)
    for pi, (phase_name, templates) in enumerate(phases):
        phase_id = plan.add(None, phase_name)
        # 工程は少し重なりながら進む。
        p_start = first_day + datetime.timedelta(days=int(total_days * pi / slots * 0.92))
        p_end = first_day + datetime.timedelta(days=int(total_days * (pi + 1.15) / slots))
        p_end = min(p_end, last_day)
        p_span = max((p_end - p_start).days, 20)

        for mi, module in enumerate(modules):
            group_id = plan.add(phase_id, module)
            m_start = p_start + datetime.timedelta(days=int(p_span * mi / max(len(modules), 1) * 0.5))
            m_room = max(int(p_span * 0.6), 15)

            count = random.randint(*per_leaf)
            chosen = [templates[i % len(templates)] for i in range(count)]
            cursor = m_start
            for name in chosen:
                length = random.randint(2, 9)
                s = next_workday(cursor + datetime.timedelta(days=random.choice([0, 0, 1, 2, 3])))
                e = add_workdays(s, length - 1)
                if (e - m_start).days > m_room:
                    s = next_workday(m_start + datetime.timedelta(days=random.randint(0, m_room // 2)))
                    e = add_workdays(s, length - 1)
                status, progress, a_start, a_end, note, waits, targets = realise(s, e, plan.team)
                plan.add(
                    group_id,
                    f"{module.split(' / ')[-1]} {name}",
                    start=iso(s), end=iso(e), actual_start=a_start, actual_end=a_end,
                    status=status, progress=progress, note=note, waits=waits,
                    targets=targets, assignee=random.choice(plan.team),
                )
                cursor = s + datetime.timedelta(days=random.randint(1, 4))

        # 工程の締めはマイルストーン。1日だけの行にして、色を付ける。
        m_day = next_workday(p_end)
        done = m_day < TODAY
        plan.add(
            phase_id,
            f"◆ {phase_name} 完了判定",
            start=iso(m_day), end=iso(m_day),
            actual_start=iso(m_day) if done else None,
            actual_end=iso(m_day) if done else None,
            status="完了" if done else "未着手",
            progress=100 if done else 0,
            assignee=plan.team[0],
            color="#7f1d1d", background="#fee2e2",
        )


# --- 計画をひとつずつ --------------------------------------------------------

plans = []

core = Plan(
    "kikan-sasshin", "基幹システム刷新",
    ADMIN,
    ["佐藤 健一", "鈴木 美咲", "高橋 亮", "田中 由紀", "伊藤 大輔", "渡辺 彩",
     "山本 拓也", "中村 香織", "小林 誠", "外部 ベンダーA"],
    "会計・購買・在庫・販売を新基幹へ。2027年4月 本稼働。月次の進捗報告はこの計画の数字を使う。",
    [("工数（人日）", "number", None), ("優先度", "select", ["高", "中", "低"]),
     ("課題番号", "text", None), ("ベンダー", "suggest", None)],
)
build_software_plan(core, CORE_MODULES,
                    datetime.date(2025, 4, 1), datetime.date(2027, 3, 31), per_module=(18, 30))
# 後工程は工程 × モジュールの形をしていないので、別に足す。
for phase, tasks in [
    ("データ移行", ["移行方針 策定", "移行対象 洗い出し", "移行ツール 作成", "移行リハーサル（1回目）",
                    "移行リハーサル（2回目）", "移行判定会", "本番移行 手順書 作成"]),
    ("受入・研修", ["受入テスト計画 作成", "受入テスト 実施", "操作マニュアル 作成",
                    "管理者研修", "利用者研修（本社）", "利用者研修（支社）", "問合せ窓口 準備"]),
    ("本番移行・稼働判定", ["移行作業（金曜夜）", "移行後 確認", "稼働判定会", "旧システム 停止"]),
    ("安定化支援", ["初月 立会", "問合せ対応", "残課題 対応", "プロジェクト完了報告"]),
]:
    pid = core.add(None, phase)
    base = datetime.date(2026, 10, 1) + datetime.timedelta(days=random.randint(0, 60))
    for i, name in enumerate(tasks):
        s = next_workday(base + datetime.timedelta(days=i * random.randint(4, 12)))
        e = add_workdays(s, random.randint(3, 12))
        status, progress, a_start, a_end, note, waits, targets = realise(s, e, core.team)
        core.add(pid, name, start=iso(s), end=iso(e), actual_start=a_start, actual_end=a_end,
                 status=status, progress=progress, note=note, waits=waits, targets=targets,
                 assignee=random.choice(core.team))
plans.append(core)

ec = Plan(
    "ec-renewal", "EC サイトリニューアル",
    ADMIN,
    ["田中 由紀", "渡辺 彩", "山口 優", "松本 和也", "井上 陽子", "木村 翔太", "外部 ベンダーB"],
    "既存 EC を新カートへ。売上ピークの11月を避け、10月中旬 リリース。",
    [("画面ID", "text", None), ("優先度", "select", ["高", "中", "低"]),
     ("工数（人日）", "number", None)],
)
build_software_plan(ec, EC_MODULES,
                    datetime.date(2026, 1, 5), datetime.date(2026, 12, 25), per_module=(14, 26))
plans.append(ec)

app = Plan(
    "mobile-app-2", "モバイルアプリ 2.0",
    DEMO,
    ["木村 翔太", "林 沙織", "清水 宏", "山崎 明日香", "石川 望", "前田 敦"],
    "会員アプリの作り直し。iOS / Android 同時公開。ストア審査の戻りを2週間見込む。",
    [("対象OS", "select", ["iOS", "Android", "共通"]), ("工数（人日）", "number", None)],
)
build_software_plan(app, APP_MODULES,
                    datetime.date(2026, 4, 1), datetime.date(2027, 3, 20), per_module=(16, 28))
plans.append(app)

data = Plan(
    "data-platform", "データ基盤移行",
    ADMIN,
    ["伊藤 大輔", "中村 香織", "森 健太郎", "池田 里奈", "橋本 亮介"],
    "オンプレ DWH をクラウドへ。基幹刷新のデータ構造に合わせる。",
    [("移行対象", "text", None), ("優先度", "select", ["高", "中", "低"])],
)
build_software_plan(data, DATA_MODULES,
                    datetime.date(2025, 10, 1), datetime.date(2026, 9, 30), per_module=(14, 24))
plans.append(data)

net = Plan(
    "network-koukai", "全社ネットワーク更改",
    ADMIN,
    ["小林 誠", "加藤 直樹", "清水 宏", "外部 ベンダーA"],
    "拠点のルータ・スイッチを更改。土日の切替作業が中心。",
    [("拠点", "select", NET_SITES), ("課題番号", "text", None)],
)
build_phase_plan(net, NET_PHASES, NET_SITES,
                 datetime.date(2026, 6, 1), datetime.date(2027, 2, 28), per_leaf=(8, 18))
plans.append(net)

isms = Plan(
    "isms-2026", "ISMS 認証取得",
    ADMIN,
    ["吉田 麻衣", "山本 拓也", "池田 里奈", "石川 望"],
    "情報システム部と管理本部を適用範囲に ISO/IEC 27001 を取得する。審査は2027年1月。",
    [("管理策番号", "text", None), ("部門", "select", ["情報システム部", "管理本部", "全社"])],
)
build_phase_plan(isms, ISMS_PHASES, ["情報システム部", "管理本部", "全社共通"],
                 datetime.date(2026, 4, 1), datetime.date(2027, 3, 31), per_leaf=(10, 20))
plans.append(isms)

product = Plan(
    "shinseihin-2026", "新製品 発売準備",
    DEMO,
    ["鈴木 美咲", "高橋 亮", "山崎 明日香", "林 沙織", "前田 敦"],
    "秋の新製品を12月1日に発売する。開発・生産・販促を1枚で見る。",
    [("部門", "select", ["営業", "マーケティング", "生産", "品質保証"]),
     ("予算（千円）", "number", None)],
)
build_phase_plan(product, PRODUCT_PHASES, ["主力モデル", "廉価モデル", "法人向け"],
                 datetime.date(2026, 3, 2), datetime.date(2026, 12, 25), per_leaf=(10, 20))
plans.append(product)

maint = Plan(
    "hoshu-2026", "保守・運用（2026年度）",
    ADMIN,
    ["小林 誠", "山口 優", "橋本 亮介", "石川 望", "外部 ベンダーA"],
    "受け付けた案件をそのまま並べている。月ごとに束ねて、担当と期日だけ管理する。",
    [("種別", "select", ["障害", "問合せ", "改善"]), ("受付番号", "text", None)],
)
months = [datetime.date(2026, 4, 1) + datetime.timedelta(days=31 * i) for i in range(12)]
for m in months:
    month_id = maint.add(None, f"{m.year}年{m.month}月")
    for kind, steps in MAINT_KINDS:
        kind_id = maint.add(month_id, kind)
        for n in range(random.randint(6, 14)):
            case = f"{kind}#{m.month:02d}{n + 1:02d}"
            base = next_workday(m + datetime.timedelta(days=random.randint(0, 24)))
            for si, step in enumerate(steps):
                s = next_workday(base + datetime.timedelta(days=si * random.randint(1, 3)))
                e = add_workdays(s, random.randint(0, 3))
                status, progress, a_start, a_end, note, waits, targets = realise(s, e, maint.team)
                maint.add(kind_id, f"{case} {step}", start=iso(s), end=iso(e),
                          actual_start=a_start, actual_end=a_end, status=status,
                          progress=progress, note=note, waits=waits, targets=targets,
                          assignee=random.choice(maint.team))
plans.append(maint)

# --- 書き込み ----------------------------------------------------------------

ids = [p.pid for p in plans]
marks = ",".join("?" * len(ids))
for table in ("tasks", "project_members", "project_settings", "project_statuses",
              "project_fields", "project_assignees", "project_holidays", "changes",
              "filter_sets"):
    cur.execute(f"DELETE FROM {table} WHERE project_id IN ({marks})", ids)
cur.execute(f"DELETE FROM projects WHERE id IN ({marks})", ids)
cur.execute("DELETE FROM leaves")

total = 0
for plan in plans:
    cur.execute(
        "INSERT INTO projects (id, name, owner_id, revision, created_at, updated_at)"
        " VALUES (?, ?, ?, 0, ?, ?)",
        (plan.pid, plan.name, plan.owner, now - 86400 * 200, now),
    )
    for user, role in ((ADMIN, "owner" if plan.owner == ADMIN else "editor"),
                       (DEMO, "owner" if plan.owner == DEMO else "editor")):
        cur.execute(
            "INSERT OR REPLACE INTO project_members (project_id, user_id, role) VALUES (?, ?, ?)",
            (plan.pid, user, role),
        )
    cur.executemany(
        "INSERT INTO project_statuses (project_id, position, name, color, percent)"
        " VALUES (?, ?, ?, ?, ?)",
        [(plan.pid, *s) for s in STATUSES],
    )
    cur.executemany(
        "INSERT INTO project_assignees (project_id, name) VALUES (?, ?)",
        [(plan.pid, name) for name in plan.team],
    )
    for k, v in (
        ("counting", "1"), ("skip_saturday", "1"), ("skip_sunday", "1"),
        ("skip_holidays", "1"), ("skip_leave", "1"), ("quarters", "1"),
        ("fiscal_year_start", "4"), ("day_width", "22"), ("frozen_columns", "1"),
        ("memo", plan.memo),
    ):
        cur.execute(
            "INSERT INTO project_settings (project_id, key, value) VALUES (?, ?, ?)",
            (plan.pid, k, v),
        )

    # 独自の項目。値は葉の行にだけ入れる。
    field_ids = []
    for fi, (label, kind, options) in enumerate(plan.fields):
        fid = uid()
        field_ids.append((fid, kind, options))
        cur.execute(
            "INSERT INTO project_fields (id, project_id, label, kind, sort_key) VALUES (?, ?, ?, ?, ?)",
            (fid, plan.pid, label, kind, key(fi)),
        )
        if kind == "select":
            cur.executemany(
                "INSERT INTO project_field_options (field_id, value, sort_key) VALUES (?, ?, ?)",
                [(fid, value, key(oi)) for oi, value in enumerate(options)],
            )

    parents = {row["parent"] for row in plan.rows}
    for row in plan.rows:
        cur.execute(
            "INSERT INTO tasks (id, project_id, parent_id, sort_key, name, start_date, end_date,"
            " actual_start, actual_end, progress, status, assignee, note, waits, targets,"
            " color, background, updated_at, updated_by)"
            " VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            (
                row["id"], plan.pid, row["parent"], row["sort"], row["name"],
                row.get("start"), row.get("end"), row.get("actual_start"), row.get("actual_end"),
                row.get("progress", 0), row.get("status", "未着手"), row.get("assignee", ""),
                row.get("note", ""), row.get("waits", ""), row.get("targets", ""),
                row.get("color", ""), row.get("background", ""),
                now - random.randint(0, 86400 * 30),
                random.choice([ADMIN, DEMO]),
            ),
        )
        total += 1

        if row["id"] in parents:
            continue
        for fid, kind, options in field_ids:
            if random.random() < 0.55:
                if kind == "number":
                    value = str(random.choice([0.5, 1, 2, 3, 5, 8, 13, 20]))
                elif kind == "select":
                    value = random.choice(options)
                elif kind == "suggest":
                    value = random.choice(["株式会社アオゾラ", "みどりソフト", "自社"])
                else:
                    value = f"{random.choice(['ISS', 'REQ', 'SC', 'INC'])}-{random.randint(100, 999)}"
                cur.execute(
                    "INSERT INTO task_field_values (task_id, field_id, value) VALUES (?, ?, ?)",
                    (row["id"], fid, value),
                )

# 集計行のステータス。日付と進捗は下から決まるが、ステータスは決まらないので、
# 誰かが手で合わせている計画に見えるよう、下から埋めておく。
for plan in plans:
    children = {}
    for row in plan.rows:
        children.setdefault(row["parent"], []).append(row)
    by_id = {row["id"]: row for row in plan.rows}

    def settle(row):
        kids = children.get(row["id"])
        if not kids:
            return row.get("status", "未着手")
        states = {settle(kid) for kid in kids}
        row["status"] = ("完了" if states == {"完了"}
                         else "未着手" if states == {"未着手"}
                         else "実施中")
        cur.execute("UPDATE tasks SET status = ? WHERE id = ?", (row["status"], row["id"]))
        return row["status"]

    for top in children.get(None, []):
        settle(top)

# --- 休暇 --------------------------------------------------------------------

REASONS = [("有給休暇", "off"), ("夏季休暇", "off"), ("リフレッシュ休暇", "off"),
           ("社外研修", "off"), ("育児休業", "off")]
for name in NAMES[:20]:
    for _ in range(random.randint(1, 4)):
        s = TODAY + datetime.timedelta(days=random.randint(-120, 150))
        s = next_workday(s)
        e = s + datetime.timedelta(days=random.choice([0, 0, 1, 2, 4]))
        note, kind = random.choice(REASONS)
        cur.execute(
            "INSERT INTO leaves (id, assignee, start_date, end_date, note, kind, created_at)"
            " VALUES (?, ?, ?, ?, ?, ?, ?)",
            (uid(), name, iso(s), iso(e), note, kind, now),
        )

# --- 変更履歴 ----------------------------------------------------------------

FIELDS = [("progress", "40", "60"), ("status", "未着手", "実施中"),
          ("end_date", "2026-09-11", "2026-09-18"), ("assignee", "", "佐藤 健一"),
          ("note", "", "先方確認待ち")]
for plan in plans:
    leaves = [row for row in plan.rows if row.get("start")]
    for _ in range(min(120, len(leaves))):
        row = random.choice(leaves)
        field, before, after = random.choice(FIELDS)
        cur.execute(
            "INSERT INTO changes (project_id, task_id, task_name, action, field, before, after, actor, at)"
            " VALUES (?, ?, ?, 'edit', ?, ?, ?, ?, ?)",
            (plan.pid, row["id"], row["name"], field, before, after,
             random.choice(["fufu", "デモ利用者"]), now - random.randint(0, 86400 * 90)),
        )

con.commit()

print(f"タスク {total} 行 / 計画 {len(plans)} 本")
for pid, name, count in cur.execute(
    "SELECT p.id, p.name, count(t.id) FROM projects p LEFT JOIN tasks t ON t.project_id = p.id"
    " GROUP BY p.id ORDER BY count(t.id) DESC"
):
    print(f"  {count:6,} {name} ({pid})")
