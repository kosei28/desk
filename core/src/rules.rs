//! 形の定義。正本はスキル（skill/SKILL.md）で、ここは同じ形を実装したもの。
//! スキルと食い違えばスキルが正しく、ここを直す。

use serde::Serialize;

/// ファイルの種類。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Task,
    Artifact,
    Inbox,
    Library,
    Principle,
    Knowledge,
    Material,
    Wiki,
    Project,
    Readme,
    Asset,
}

impl Kind {
    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Task => "task",
            Kind::Artifact => "artifact",
            Kind::Inbox => "inbox",
            Kind::Library => "library",
            Kind::Principle => "principle",
            Kind::Knowledge => "knowledge",
            Kind::Material => "material",
            Kind::Wiki => "wiki",
            Kind::Project => "project",
            Kind::Readme => "readme",
            Kind::Asset => "asset",
        }
    }

    /// 名前から種類を引く。
    pub fn from_name(s: &str) -> Option<Kind> {
        ALL_KINDS
            .iter()
            .chain([Kind::Readme, Kind::Asset].iter())
            .copied()
            .find(|k| k.as_str() == s)
    }
}

impl std::fmt::Display for Kind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// frontmatter のキー 1 つの決まり。
#[derive(Debug, Clone, Copy, Default)]
pub struct KeyRule {
    pub name: &'static str,
    /// 配列
    pub list: bool,
    /// 相対パスのリンク
    pub link: bool,
    /// キーが無ければ挙げる（--fix は空で足す）
    pub required: bool,
    /// 値が空なら挙げる
    pub non_empty: bool,
    /// 取れる値
    pub values: &'static [&'static str],
    /// YYYY-MM-DD
    pub date: bool,
    /// `desk rules` に出す説明
    pub note: &'static str,
}

/// 種類 1 つの形。
#[derive(Debug, Clone, Copy)]
pub struct KindRule {
    pub kind: Kind,
    /// 置くディレクトリの名前
    pub dir: &'static str,
    /// ルートに置く
    pub in_root: bool,
    /// プロジェクトに置く
    pub in_project: bool,
    /// 名前が YYYY-MM-DD-<slug>.md
    pub dated: bool,
    /// 中を入れ子にしてよい（wiki だけ）
    pub nested: bool,
    /// 中に assets/ を置ける
    pub assets: bool,
    /// desk archive で archive/ に移す
    pub archivable: bool,
    /// frontmatter が要る
    pub need_fm: bool,
    pub keys: &'static [KeyRule],
    /// desk new が書く節
    pub sections: &'static [&'static str],
    pub desc: &'static str,
}

pub const STATUSES: &[&str] = &["backlog", "todo", "inprogress", "done", "dropped"];
pub const NEEDS_HUMAN: &[&str] = &["false", "true"];

/// タスクに要る節（needs_human: false）。
pub const TASK_SECTIONS: &[&str] = &["目的", "いま", "成果物", "現在地", "決まり", "手順"];
/// needs_human: true のタスクに要る節。
pub const HUMAN_TASK_SECTIONS: &[&str] = &["目的", "成果物"];

pub const LOG_SECTION: &str = "ログ";
pub const NOW_SECTION: &str = "いま";

/// ルートに置いてよい名前。プロジェクトは projects/ の下に置く。
pub const ROOT_RESERVED: &[&str] = &[
    "README.md",
    "principles",
    "knowledge",
    "archive",
    "assets",
    "projects",
];

/// プロジェクトを置くディレクトリ
pub const PROJECTS_DIR: &str = "projects";
/// 終わったものを置くディレクトリ（プロジェクトの中と、ルート）
pub const ARCHIVE_DIR: &str = "archive";
/// どのプロジェクトにも属さないものを置くプロジェクト
pub const MISC_PROJECT: &str = "misc";
/// ルートの印
pub const MARKER_FILE: &str = ".desk";

const fn key(name: &'static str) -> KeyRule {
    KeyRule {
        name,
        list: false,
        link: false,
        required: false,
        non_empty: false,
        values: &[],
        date: false,
        note: "",
    }
}

pub static RULES: &[KindRule] = &[
    KindRule {
        kind: Kind::Task,
        dir: "tasks",
        in_root: false,
        in_project: true,
        dated: true,
        nested: false,
        assets: false,
        archivable: true,
        need_fm: true,
        desc: "タスク。1 タスク 1 ファイル。節は目的・いま・成果物・現在地・決まり・手順で、最後のログの節は追記だけ",
        keys: &[
            KeyRule {
                required: true,
                non_empty: true,
                values: STATUSES,
                note: "backlog はやるか決めていない・いつか、todo はやると決めた・未着手、inprogress は着手している",
                ..key("status")
            },
            KeyRule {
                required: true,
                non_empty: true,
                values: NEEDS_HUMAN,
                note: "true は人の判断か作業が要る（確認・人がやる作業）",
                ..key("needs_human")
            },
            KeyRule {
                link: true,
                note: "親タスク",
                ..key("parent")
            },
            KeyRule {
                list: true,
                link: true,
                note: "待っているタスク",
                ..key("depends_on")
            },
            KeyRule {
                note: "~ からのパス",
                ..key("repo")
            },
            key("branch"),
            KeyRule {
                note: "~ からのパス",
                ..key("worktree")
            },
            KeyRule {
                note: "URL",
                ..key("pr")
            },
        ],
        sections: TASK_SECTIONS,
    },
    KindRule {
        kind: Kind::Artifact,
        dir: "artifacts",
        in_root: false,
        in_project: true,
        dated: false,
        nested: false,
        assets: true,
        archivable: true,
        need_fm: true,
        desc: "タスクの文書。タスクと一緒に役目を終える",
        keys: &[KeyRule {
            list: true,
            link: true,
            required: true,
            note: "どのタスクのための文書か",
            ..key("tasks")
        }],
        sections: &[],
    },
    KindRule {
        kind: Kind::Wiki,
        dir: "wiki",
        in_root: false,
        in_project: true,
        dated: false,
        nested: true,
        assets: true,
        archivable: false,
        need_fm: true,
        desc: "プロジェクトの成果物としてメンテする文書。構成はプロジェクトで設計する（入れ子にしてよい）",
        keys: &[
            KeyRule {
                required: true,
                non_empty: true,
                note: "何が書いてあるか、いつ読むか",
                ..key("description")
            },
            KeyRule {
                date: true,
                note: "確かめたが変えなかった日（任意）",
                ..key("verified")
            },
        ],
        sections: &[],
    },
    KindRule {
        kind: Kind::Inbox,
        dir: "inbox",
        in_root: false,
        in_project: true,
        dated: true,
        nested: false,
        assets: false,
        archivable: false,
        need_fm: false,
        desc: "まだどこにも属さない未整理のものの受け箱。棚卸しして空にする。frontmatter は任意",
        keys: &[KeyRule {
            link: true,
            note: "関係するタスクや文書（任意）",
            ..key("about")
        }],
        sections: &[],
    },
    KindRule {
        kind: Kind::Library,
        dir: "library",
        in_root: false,
        in_project: true,
        dated: true,
        nested: false,
        assets: false,
        archivable: false,
        need_fm: false,
        desc: "あとで参照したい情報の控え。書いた時点のまま残し、メンテしない。frontmatter は全部任意",
        keys: &[
            key("url"),
            KeyRule {
                note: "URL がないときの出どころ。自由な文（リンクではない）",
                ..key("source")
            },
            KeyRule {
                list: true,
                ..key("tags")
            },
            KeyRule {
                link: true,
                note: "きっかけになったタスク",
                ..key("about")
            },
        ],
        sections: &["要約"],
    },
    KindRule {
        kind: Kind::Principle,
        dir: "principles",
        in_root: true,
        in_project: true,
        dated: false,
        nested: false,
        assets: false,
        archivable: false,
        need_fm: true,
        desc: "抽象的で根源的な考え方。ルートは全プロジェクト、プロジェクトのものはそこだけ",
        keys: &[
            KeyRule {
                required: true,
                non_empty: true,
                ..key("description")
            },
            KeyRule {
                link: true,
                note: "プロジェクトの原則が具体化しているルートの原則",
                ..key("principle")
            },
            KeyRule {
                date: true,
                ..key("verified")
            },
        ],
        sections: &[],
    },
    KindRule {
        kind: Kind::Knowledge,
        dir: "knowledge",
        in_root: true,
        in_project: true,
        dated: false,
        nested: false,
        assets: false,
        archivable: false,
        need_fm: true,
        desc: "AI が作業で使う決まり・手順・事実。段 0 は description、段 1 は本文、段 2 は本文が指す詳細",
        keys: &[
            KeyRule {
                required: true,
                non_empty: true,
                note: "いつ読むかと何が書いてあるか",
                ..key("description")
            },
            KeyRule {
                required: true,
                non_empty: true,
                link: true,
                note: "どの原則の当てはめか（1 つ以上。1 つなら文字列、複数ならリスト）。詳細（ほかのナレッジの本文から指されるもの）には要らない",
                ..key("principle")
            },
            KeyRule {
                list: true,
                link: true,
                note: "根拠（素材、決定）",
                ..key("sources")
            },
            KeyRule {
                date: true,
                ..key("verified")
            },
        ],
        sections: &[],
    },
    KindRule {
        kind: Kind::Material,
        dir: "materials",
        in_root: false,
        in_project: true,
        dated: true,
        nested: false,
        assets: false,
        archivable: false,
        need_fm: true,
        desc: "ナレッジの素材（指摘、やり直し、思っていたのと違ったこと）。1 件 1 ファイル",
        keys: &[
            KeyRule {
                link: true,
                note: "起きた場所。ツリーにあるときだけ書く",
                ..key("source")
            },
            KeyRule {
                list: true,
                link: true,
                required: true,
                note: "蒸留してできたナレッジ。空なら蒸留待ち",
                ..key("knowledge")
            },
        ],
        sections: &["何が起きたか", "一般化の候補"],
    },
];

/// ls で選べる種類。
pub const ALL_KINDS: &[Kind] = &[
    Kind::Task,
    Kind::Artifact,
    Kind::Inbox,
    Kind::Library,
    Kind::Principle,
    Kind::Knowledge,
    Kind::Material,
    Kind::Wiki,
    Kind::Project,
];

/// 種類の形を返す。
pub fn rule_of(kind: Kind) -> Option<&'static KindRule> {
    RULES.iter().find(|r| r.kind == kind)
}

/// ディレクトリの名前から種類の形を返す。
pub fn rule_by_dir(dir: &str) -> Option<&'static KindRule> {
    RULES.iter().find(|r| r.dir == dir)
}

/// その種類の frontmatter のキーの決まりを返す。
pub fn key_rule_of(kind: Kind, key: &str) -> Option<&'static KeyRule> {
    rule_of(kind)?.keys.iter().find(|k| k.name == key)
}

/// 種類が分からないファイルでリンクとして読むキー。
const GENERIC_LINK_KEYS: &[&str] = &[
    "parent",
    "depends_on",
    "tasks",
    "about",
    "principle",
    "sources",
    "source",
    "knowledge",
];

/// その種類で相対パスのリンクとして読む frontmatter のキー。
pub fn link_keys(kind: Option<Kind>) -> Vec<&'static str> {
    match kind.and_then(rule_of) {
        None => GENERIC_LINK_KEYS.to_vec(),
        Some(r) => r.keys.iter().filter(|k| k.link).map(|k| k.name).collect(),
    }
}

/// プロジェクトの直下に置いてよい名前。
pub fn project_entries() -> Vec<&'static str> {
    let mut out = vec!["README.md", "assets", ARCHIVE_DIR];
    out.extend(RULES.iter().filter(|r| r.in_project).map(|r| r.dir));
    out
}

/// 実装している形の一覧（`desk rules` の出力そのもの）。
pub fn describe() -> String {
    let mut w = String::new();
    let mut line = |s: &str| {
        w.push_str(s);
        w.push('\n');
    };
    line("CLI が実装している形（正本はスキル desk。食い違えばスキルが正しい）");
    line("");
    line(&format!(
        "ルートに置く名前: {MARKER_FILE}、{}",
        ROOT_RESERVED.join("、")
    ));
    line(&format!(
        "プロジェクト: {PROJECTS_DIR}/<名前>/（どこにも属さないものは {PROJECTS_DIR}/{MISC_PROJECT}/）。直下に置く名前: {}",
        project_entries().join("、")
    ));
    line(&format!(
        "archive: プロジェクトの {ARCHIVE_DIR}/ は {ARCHIVE_DIR}/tasks/・{ARCHIVE_DIR}/artifacts/（元と同じ形）。プロジェクトごとは {ARCHIVE_DIR}/{PROJECTS_DIR}/<名前>/"
    ));
    line(
        "リンク: 本文も frontmatter も、そのファイルからの相対パス。絶対パス・~ 始まり・名前だけは書かない",
    );
    line(
        "名前: <slug>.md（slug は英小文字・数字・ハイフン）。同じ種類の中（archive/ も含めて）で一意",
    );
    line("");
    for r in RULES {
        let mut place = Vec::new();
        if r.in_root {
            place.push(format!("{}/", r.dir));
        }
        if r.in_project {
            place.push(format!("{PROJECTS_DIR}/<p>/{}/", r.dir));
        }
        let mut notes = vec![if r.nested {
            "中は入れ子にしてよい".to_string()
        } else {
            "中はフラット".to_string()
        }];
        if r.assets {
            notes.push(format!("静的ファイルは {}/assets/", r.dir));
        }
        if r.archivable {
            notes.push(format!("終わったら {ARCHIVE_DIR}/{}/", r.dir));
        }
        line(&format!("{} — {}", r.kind, r.desc));
        line(&format!(
            "  置き場所: {}（{}）",
            place.join("、"),
            notes.join("。")
        ));
        line(if r.dated {
            "  名前: YYYY-MM-DD-<slug>.md（作った日）"
        } else {
            "  名前: <slug>.md"
        });
        line(&format!(
            "  frontmatter: {}",
            if r.need_fm { "要る" } else { "任意" }
        ));
        for k in r.keys {
            let mut a: Vec<String> = Vec::new();
            for (on, s) in [
                (k.required, "必須"),
                (k.non_empty, "空にしない"),
                (k.list, "配列"),
                (k.link, "相対パス"),
            ] {
                if on {
                    a.push(s.to_string());
                }
            }
            if !k.values.is_empty() {
                a.push(k.values.join(" | "));
            }
            if k.date {
                a.push("YYYY-MM-DD".to_string());
            }
            if !k.note.is_empty() && k.note != "必須" {
                a.push(k.note.to_string());
            }
            line(&format!("    {}: {}", k.name, a.join("。")));
        }
        if r.kind == Kind::Task {
            line(&format!(
                "  必須の節: {}（needs_human: true は {}）。「{LOG_SECTION}」の節は最後で追記だけ",
                TASK_SECTIONS.join("・"),
                HUMAN_TASK_SECTIONS.join("・")
            ));
        } else if !r.sections.is_empty() {
            line(&format!("  desk new が書く節: {}", r.sections.join("・")));
        }
    }
    w
}
