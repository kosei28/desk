//! テストで使う架空のツリー。中身は架空のもので、実際のツリーの中身を入れない。

// テストのファイルごとに使うものが違う
#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};

use tempfile::TempDir;

/// 一時ディレクトリに作ったツリー。消えるまで持っておく。
pub struct Fixture {
    _dir: TempDir,
    pub root: PathBuf,
}

impl Fixture {
    /// `.desk` と README だけのツリー（desk init で作るものと同じ置き方）。
    pub fn new() -> Fixture {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().canonicalize().unwrap().join("desk");
        let f = Fixture { _dir: dir, root };
        f.write(".desk", "");
        f.write(".gitignore", ".DS_Store\n");
        f.write("README.md", "# desk\n\n架空のツリー。\n");
        f.write(
            "projects/misc/README.md",
            "# misc\n\nどのプロジェクトにも属さないもの。\n",
        );
        f
    }

    /// 手元のツリーを一時ディレクトリに複製する（.git は複製しない）。
    pub fn copy_of(src: &Path) -> Fixture {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().canonicalize().unwrap().join("desk");
        copy_dir(src, &root);
        Fixture { _dir: dir, root }
    }

    /// ルートからのパスにファイルを書く。
    pub fn write(&self, rel: &str, content: &str) {
        let p = self.root.join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, content).unwrap();
    }

    /// ルートからのパスにディレクトリを作る。
    pub fn mkdir(&self, rel: &str) {
        fs::create_dir_all(self.root.join(rel)).unwrap();
    }

    pub fn path(&self, rel: &str) -> PathBuf {
        self.root.join(rel)
    }
}

fn copy_dir(src: &Path, dst: &Path) {
    fs::create_dir_all(dst).unwrap();
    for e in fs::read_dir(src).unwrap() {
        let e = e.unwrap();
        if e.file_name() == ".git" {
            continue;
        }
        let (from, to) = (e.path(), dst.join(e.file_name()));
        let t = e.file_type().unwrap();
        if t.is_dir() {
            copy_dir(&from, &to);
        } else if t.is_symlink() {
            #[cfg(unix)]
            std::os::unix::fs::symlink(fs::read_link(&from).unwrap(), &to).unwrap();
        } else {
            fs::copy(&from, &to).unwrap();
        }
    }
}

/// タスクのファイルの中身（Go のテストの taskDoc）。
pub fn task_doc(status: &str, extra_fm: &str, title: &str) -> String {
    let mut fm = format!("status: {status}\n");
    if !extra_fm.contains("needs_human:") {
        fm.push_str("needs_human: false\n");
    }
    if !extra_fm.is_empty() {
        fm.push_str(extra_fm);
        fm.push('\n');
    }
    format!(
        "---\n{fm}---\n\n# {title}\n\n## 目的\n目的。\n\n## いま\nいま。\n\n## 成果物\n- なし\n\n## 現在地\n次の一手。\n\n## 決まり\nなし。\n\n## 手順\n- [ ] 手順\n"
    )
}

/// 架空のレシピアプリのプロジェクトを持つツリー（Go のテストの recipeTree）。
pub fn recipe_tree() -> Fixture {
    let f = Fixture::new();
    f.write(
        "principles/prefer-structure-over-patches.md",
        "---\ndescription: 不具合は起きない構造にして直す\n---\n\n# 構造で直す\n",
    );
    f.write(
        "projects/recipe-app/README.md",
        "# recipe-app\n\n架空のレシピアプリ。\n",
    );
    f.write(
        "projects/recipe-app/tasks/2026-09-07-meal-planner.md",
        &task_doc("inprogress", "", "献立プランナー"),
    );
    f.write(
        "projects/recipe-app/tasks/2026-11-09-printable-menu.md",
        &(task_doc(
            "inprogress",
            "parent: 2026-09-07-meal-planner.md",
            "献立を印刷できるようにする",
        ) + "- [印刷のレイアウト](../artifacts/print-layout.md)\n\n## ログ\n### 2026-11-10\n- 決定: 印刷用のページは画面と別に作る\n"),
    );
    f.write(
        "projects/recipe-app/artifacts/print-layout.md",
        "---\ntasks: [../tasks/2026-11-09-printable-menu.md]\n---\n\n# 印刷のレイアウト\n\n![](assets/print-sample.png)\n\n親: [献立プランナー](../tasks/2026-09-07-meal-planner.md)\n",
    );
    f.write(
        "projects/recipe-app/artifacts/assets/print-sample.png",
        "png",
    );
    f.write(
        "projects/recipe-app/wiki/meal-planner/overview.md",
        "---\ndescription: 献立プランナーの仕組み全体\n---\n\n# 献立プランナー\n\n印刷: [レイアウト](../../artifacts/print-layout.md)\n",
    );
    f.write(
        "projects/recipe-app/knowledge/api-error-format.md",
        "---\ndescription: API のエラーを返すときに読む\nprinciple: ../../../principles/prefer-structure-over-patches.md\nsources: []\n---\n\n# エラーの形\n",
    );
    f
}

/// 読み方がずれやすいところを集めたツリー。
pub fn edge_tree() -> Fixture {
    let f = Fixture::new();
    // 形の外のもの、. で始まるもの
    f.write("notes.md", "# ルートに置いたメモ\n");
    f.write(".hidden/x.md", "# 見ない\n");
    f.write("projects/.hidden-project/README.md", "# 見ない\n");
    f.mkdir("projects/empty");

    f.write(
        "principles/structure.md",
        "---\ndescription: >\n  不具合は\n  構造で直す\n---\n\n# 構造で直す\n",
    );
    f.write(
        "knowledge/review.md",
        "---\ndescription: \"レビューの  指摘を\\t直すときに読む\"\nprinciple: [../principles/structure.md, '../principles/none.md']\nsources:\n  - ../projects/alpha/materials/2026-11-07-redo.md\nverified: 2026-11-12\n---\n\n# レビューの指摘\n\n詳細: [詳細](review-detail.md)\n",
    );
    f.write(
        "knowledge/review-detail.md",
        "---\ndescription: 詳細\n---\n\n# 詳細\n",
    );

    f.write(
        "projects/alpha/README.md",
        "---\nrepo: ~/src/alpha\n---\n\n# alpha <b>&</b>\n\n架空のプロジェクト。[タスク](tasks/2026-11-01-main.md)\n",
    );
    // CRLF、リストの印付きの「いま」、見出しの末尾の #、いろいろな形のリンク
    let main = [
        "---",
        "status: inprogress",
        "needs_human: false",
        "parent: '2026-10-31-missing.md'",
        "depends_on:",
        "  - 2026-11-02-human.md",
        "  - \"2026-11-03-broken.md\"",
        "repo: ~/src/alpha",
        "---",
        "",
        "# 本筋のタスク\ta ##",
        "",
        "## 目的 ##",
        "[空白](<../artifacts/with space.md>) と [エスケープ](../artifacts/with%20space.md \"題\") と [節](./2026-11-02-human.md#目的)",
        "[入れ子 [角] 括弧](../artifacts/design.md)、![図](../artifacts/assets/figure.png)、[[guide]]",
        "`[コードの中](../artifacts/design.md)` と [自分](2026-11-01-main.md) と [外](https://example.com/a_(b))",
        "[絶対](/tmp/x.md) [家](~/x.md) [メール](mailto:a@example.com) [アンカー](#いま) [ファイル](file:///tmp/y.md)",
        "[全角の空白](../artifacts/全角　スペース.md) [ツリーの外](../../../../outside.md)",
        "",
        "[参照]: ../wiki/guide.md",
        "   [参照2]: <../wiki/deep/nested.md>",
        "",
        "~~~",
        "[フェンスの中](../artifacts/design.md)",
        "~~~",
        "",
        "## いま",
        "",
        "- [ ] 次はこれ  ",
        "",
        "## 成果物",
        "- [設計](../artifacts/design.md)",
        "",
    ]
    .join("\r\n");
    f.write("projects/alpha/tasks/2026-11-01-main.md", &main);
    f.write(
        "projects/alpha/tasks/2026-11-02-human.md",
        "---\nzeta: 最後\nstatus: todo\nneeds_human: true\nparent: 2026-11-01-main.md\nalpha: 最初\nestimate: 1.50\nflag: true\nempty:\ntilde: ~\nquoted_null: \"null\"\ntags: []\npr: https://example.com/pr/1\npr: https://example.com/pr/2\n---\n\n## 目的\n\n1. 決めてほしい\n",
    );
    f.write(
        "projects/alpha/tasks/2026-11-03-broken.md",
        "---\nstatus: [todo\n---\n\n# 壊れた frontmatter\n\n[設計](../artifacts/design.md)\n",
    );
    f.write(
        "projects/alpha/tasks/2026-11-04-unclosed.md",
        "---\nstatus: todo\n\n# 閉じていない\n",
    );
    f.write(
        "projects/alpha/tasks/2026-11-05-scalar.md",
        "---\njust a scalar\n---\n\n# frontmatter が対応でない\n",
    );
    f.write(
        "projects/alpha/tasks/2026-11-06-dots.md",
        "---\nstatus: done\nneeds_human: false\n...\n\n# ... で閉じる\n",
    );
    f.write(
        "projects/alpha/tasks/no-date.md",
        &task_doc("todo", "", "日付の無い名前"),
    );
    f.write("projects/alpha/tasks/sub/nested.md", "# 入れ子\n");
    f.write("projects/alpha/tasks/image.png", "png");
    f.write("projects/alpha/drafts/x.md", "# 置けないディレクトリ\n");
    f.write("projects/alpha/notes.md", "# 直下のメモ\n");

    f.write(
        "projects/alpha/artifacts/design.md",
        "---\ntasks: [../tasks/2026-11-01-main.md]\n---\n\n# 設計\n\n![](assets/figure.png)\n",
    );
    f.write(
        "projects/alpha/artifacts/with space.md",
        "---\ntasks: []\n---\n\n# 空白の入った名前\n",
    );
    f.write(
        "projects/alpha/artifacts/全角　スペース.md",
        "# 全角の空白\n",
    );
    f.write("projects/alpha/artifacts/assets/figure.png", "png");
    f.write(
        "projects/alpha/artifacts/assets/notes.md",
        "# 静的ファイルの md\n[x](../design.md)\n",
    );

    f.write(
        "projects/alpha/wiki/guide.md",
        "---\ndescription: 使い方\n---\n\n# 使い方\n",
    );
    f.write(
        "projects/alpha/wiki/deep/nested.md",
        "---\ndescription: 入れ子の wiki\n---\n\n# 入れ子\n\n[本筋](../../tasks/2026-11-01-main.md)\n",
    );
    f.write("projects/alpha/wiki/assets/w.png", "png");
    f.write(
        "projects/alpha/inbox/2026-11-05-memo.md",
        "---\nabout: ../tasks/2026-11-01-main.md\n---\n\n# メモ\n",
    );
    f.write(
        "projects/alpha/inbox/2026-11-06-bare.md",
        "見出しの無いメモ\n",
    );
    f.write(
        "projects/alpha/library/2026-11-06-ref.md",
        "---\nurl: https://example.com/ref\nsource: 勉強会で聞いた話 ../tasks/2026-11-01-main.md\ntags: [rust, yaml]\nabout: ../tasks/2026-11-01-main.md\n---\n\n# 控え\n\n## 要約\n要約。\n",
    );
    f.write(
        "projects/alpha/materials/2026-11-07-redo.md",
        "---\nsource: ../tasks/2026-11-01-main.md\nknowledge: [../../../knowledge/review.md]\n---\n\n# やり直した\n",
    );
    f.write(
        "projects/alpha/archive/tasks/2026-10-01-old.md",
        &task_doc(
            "done",
            "depends_on: [../../tasks/2026-11-01-main.md]",
            "終わったタスク",
        ),
    );
    f.write(
        "projects/alpha/archive/wiki/x.md",
        "# archive に置けないもの\n",
    );

    f.write("projects/beta/README.md", "# beta\n");
    f.write(
        "projects/beta/tasks/2026-11-08-cross.md",
        &(task_doc(
            "backlog",
            "depends_on: [../../alpha/tasks/2026-11-01-main.md]",
            "ほかのプロジェクトを待つ",
        ) + "\n[ガイド](../../alpha/wiki/guide.md)\n"),
    );

    f.write("archive/projects/old/README.md", "# 終わったプロジェクト\n");
    f.write(
        "archive/projects/old/tasks/2026-01-01-z.md",
        &task_doc("done", "", "古いタスク"),
    );
    f.write(
        "archive/projects/old/archive/tasks/2026-01-02-y.md",
        &task_doc(
            "dropped",
            "parent: ../../tasks/2026-01-01-z.md",
            "やめたタスク",
        ),
    );
    f.write("archive/notes.md", "# 置けない\n");

    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        symlink(
            f.path("projects/alpha/wiki/guide.md"),
            f.path("projects/alpha/wiki/linked.md"),
        )
        .unwrap();
        symlink(
            f.path("projects/alpha/wiki/deep"),
            f.path("projects/alpha/wiki/linked-dir"),
        )
        .unwrap();
    }
    f
}

/// Go と読み方を変えたところ（差分テストの例外）を集めたツリー。
pub fn exception_tree() -> Fixture {
    let f = Fixture::new();
    f.write(
        "projects/alpha/library/2026-11-01-typed.md",
        "---\ntags: [2026, true, null, 1.5, rust]\nmeta: {a: b}\nnested: [[a], b]\n---\n\n# 型の付いた値\n",
    );
    f
}
