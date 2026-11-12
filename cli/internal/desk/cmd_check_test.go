package desk

import (
	"encoding/json"
	"strings"
	"testing"
)

func checkJSON(t *testing.T, args ...string) ([]Finding, int) {
	t.Helper()
	out, errOut, code := run(t, append([]string{"check", "--json"}, args...)...)
	var fs []Finding
	if err := json.Unmarshal([]byte(out), &fs); err != nil {
		t.Fatalf("JSON でない: %v\n%s\n%s", err, out, errOut)
	}
	return fs, code
}

func hasFinding(fs []Finding, path, problem string) bool {
	for _, f := range fs {
		if f.Path == path && strings.Contains(f.Problem, problem) {
			return true
		}
	}
	return false
}

func TestCheckCleanTree(t *testing.T) {
	recipeTree(t)
	fs, code := checkJSON(t)
	if code != 0 || len(fs) != 0 {
		t.Errorf("形に合っているのに挙がる（code=%d）: %+v", code, fs)
	}
}

func TestCheckFindings(t *testing.T) {
	root := recipeTree(t)
	p := "projects/recipe-app/"
	write(t, root, "notes/todo.md", "# メモ\n")
	write(t, root, p+"drafts/x.md", "# 下書き\n")
	write(t, root, p+"tasks/no-date.md", taskDoc("todo", "", "日付なし"))
	write(t, root, p+"tasks/2026-11-01-bad-status.md", taskDoc("doing", "", "決まりの外の status"))
	write(t, root, p+"tasks/2026-11-02-no-fm.md", "# frontmatter なし\n")
	write(t, root, p+"tasks/2026-11-03-short.md", "---\nstatus: todo\nowner: ai\n---\n\n# 節が足りない\n\n## ログ\n- a\n\n## 目的\nx\n")
	write(t, root, p+"tasks/2026-11-04-closed.md", taskDoc("done", "", "閉じたのに外"))
	write(t, root, p+"archive/tasks/2026-11-05-reopened.md", taskDoc("todo", "", "動いているのに中"))
	write(t, root, p+"tasks/2026-11-06-started.md", taskDoc("inprogress", "depends_on: [2026-11-07-dep.md]", "依存が終わる前に着手"))
	write(t, root, p+"tasks/2026-11-07-dep.md", taskDoc("todo", "", "終わっていない依存"))
	write(t, root, p+"tasks/2026-11-08-parent.md", taskDoc("inprogress", "", "親"))
	write(t, root, p+"archive/tasks/2026-11-08-child.md", taskDoc("done", "parent: ../../tasks/2026-11-08-parent.md", "子"))
	write(t, root, p+"artifacts/links.md", "---\ntasks: [../tasks/2026-11-08-parent.md]\n---\n\n[消えた](gone.md) [絶対](/tmp/x.md) [[名前]]\n")
	write(t, root, p+"artifacts/assets/unused.png", "png")
	write(t, root, p+"knowledge/no-principle.md", "---\ndescription: x\n---\n# x\n")
	write(t, root, p+"archive/artifacts/links.md", "---\ntasks: []\n---\n# 同じ名前\n")
	write(t, root, p+"tasks/2026-11-10-planned.md", taskDoc("todo", "depends_on: [2026-11-10-someday.md]", "いつかに依存"))
	write(t, root, p+"tasks/2026-11-10-someday.md", taskDoc("backlog", "", "いつか"))
	write(t, root, p+"tasks/2026-11-11-on-dropped.md", taskDoc("todo", "depends_on: [../archive/tasks/2026-11-11-dropped.md]", "やめたものに依存"))
	write(t, root, p+"archive/tasks/2026-11-11-dropped.md", taskDoc("dropped", "", "やめた"))
	write(t, root, p+"tasks/2026-11-12-todo-parent.md", taskDoc("todo", "", "未着手の親"))
	write(t, root, p+"tasks/2026-11-12-started-child.md", taskDoc("inprogress", "parent: 2026-11-12-todo-parent.md", "着手した子"))
	write(t, root, p+"tasks/2026-11-13-backlog-parent.md", taskDoc("backlog", "", "いつかの親"))
	write(t, root, p+"tasks/2026-11-13-todo-child.md", taskDoc("todo", "parent: 2026-11-13-backlog-parent.md", "やると決めた子"))
	write(t, root, p+"archive/tasks/2026-11-14-closed-parent.md", taskDoc("done", "", "閉じた親"))
	write(t, root, p+"tasks/2026-11-14-left-child.md", taskDoc("todo", "parent: ../archive/tasks/2026-11-14-closed-parent.md", "残った子"))
	write(t, root, p+"tasks/2026-11-15-self.md", taskDoc("todo", "depends_on: [2026-11-15-self.md]", "自分に依存"))
	write(t, root, p+"tasks/2026-11-16-a.md", taskDoc("todo", "depends_on: [2026-11-16-b.md]", "循環 a"))
	write(t, root, p+"tasks/2026-11-16-b.md", taskDoc("todo", "depends_on: [2026-11-16-a.md]", "循環 b"))
	write(t, root, p+"tasks/2026-11-17-a.md", taskDoc("todo", "parent: 2026-11-17-b.md", "親の循環 a"))
	write(t, root, p+"tasks/2026-11-17-b.md", taskDoc("todo", "parent: 2026-11-17-a.md", "親の循環 b"))
	write(t, root, p+"tasks/2026-11-18-no-owner.md", "---\nstatus: todo\n---\n\n# owner が無い\n\n## 目的\nx\n\n## いま\nx\n\n## 成果物\nx\n\n## 現在地\nx\n\n## 決まり\nx\n\n## 手順\nx\n")
	write(t, root, p+"tasks/2026-11-19-old-status.md", taskDoc("active", "", "前の語彙"))

	fs, code := checkJSON(t)
	if code != 1 {
		t.Errorf("見つかったのに終了コードが %d", code)
	}
	for _, w := range []struct{ path, problem string }{
		{"notes/", "決まった名前だけ"},
		{p + "drafts/", "プロジェクトに置けないディレクトリ"},
		{p + "tasks/no-date.md", "日付が無い"},
		{p + "tasks/2026-11-01-bad-status.md", `status の値 "doing"`},
		{p + "tasks/2026-11-02-no-fm.md", "frontmatter が無い"},
		{p + "tasks/2026-11-03-short.md", "必須の節「いま」が無い"},
		{p + "tasks/2026-11-03-short.md", "ログの節が最後にない"},
		{p + "tasks/2026-11-04-closed.md", "archive/ の外"},
		{p + "archive/tasks/2026-11-05-reopened.md", "archive/ の中"},
		{p + "tasks/2026-11-06-started.md", "depends_on の projects/recipe-app/tasks/2026-11-07-dep.md が todo なのに inprogress"},
		{p + "tasks/2026-11-08-parent.md", "子タスクが全部終わっているのに inprogress"},
		{p + "tasks/2026-11-10-planned.md", "todo なのに depends_on の projects/recipe-app/tasks/2026-11-10-someday.md が backlog"},
		{p + "tasks/2026-11-11-on-dropped.md", "が dropped"},
		{p + "tasks/2026-11-12-started-child.md", "が todo なのに inprogress"},
		{p + "tasks/2026-11-13-todo-child.md", "が backlog なのに todo"},
		{p + "tasks/2026-11-14-left-child.md", "が done なのに todo"},
		{p + "tasks/2026-11-15-self.md", "depends_on が自分自身を指している"},
		{p + "tasks/2026-11-16-a.md", "depends_on が循環している"},
		{p + "tasks/2026-11-16-b.md", "depends_on が循環している"},
		{p + "tasks/2026-11-17-a.md", "parent が循環している"},
		{p + "tasks/2026-11-17-b.md", "parent が循環している"},
		{p + "tasks/2026-11-18-no-owner.md", "owner が無い"},
		{p + "tasks/2026-11-19-old-status.md", `status の値 "active"`},
		{p + "artifacts/links.md", "gone.md の先が無い"},
		{p + "artifacts/links.md", "/tmp/x.md が相対パスでない"},
		{p + "artifacts/links.md", "[[名前]] が名前だけ"},
		{p + "artifacts/links.md", "名前が重複"},
		{p + "artifacts/assets/unused.png", "どこからも指されていない"},
		{p + "knowledge/no-principle.md", "principle が無い"},
	} {
		if !hasFinding(fs, w.path, w.problem) {
			t.Errorf("挙がらない: %s %s", w.path, w.problem)
		}
	}
	for _, f := range fs {
		if strings.HasPrefix(f.Path, "notes/todo.md") || strings.HasPrefix(f.Path, p+"drafts/x.md") {
			t.Errorf("決まりの外のディレクトリの中を 1 つずつ挙げている: %+v", f)
		}
	}

	// --name は関係するファイルの分も出す
	fs, _ = checkJSON(t, "--name", "2026-11-08-child")
	if !hasFinding(fs, p+"tasks/2026-11-08-parent.md", "子タスクが全部終わっている") {
		t.Errorf("--name で親の分が出ない: %+v", fs)
	}
	for _, f := range fs {
		if f.Path != p+"tasks/2026-11-08-parent.md" && f.Path != p+"archive/tasks/2026-11-08-child.md" {
			t.Errorf("--name で関係ないものが出る: %+v", f)
		}
	}

	// --project はそのプロジェクトだけ
	fs, _ = checkJSON(t, "--project", "misc")
	if len(fs) != 0 {
		t.Errorf("--project misc で他が出る: %+v", fs)
	}
}

func TestCheckFix(t *testing.T) {
	root := recipeTree(t)
	gitAddAll(t, root)
	p := "projects/recipe-app/"
	write(t, root, p+"tasks/2026-11-02-no-fm.md", "# 人が書いたタスク\n\n## 目的\nx\n\n## いま\nx\n\n## 成果物\nx\n\n## 現在地\nx\n\n## 決まり\nx\n\n## 手順\nx\n")
	write(t, root, p+"tasks/2026-11-04-closed.md", taskDoc("done", "", "閉じた")+"\n[設計](../artifacts/print-layout.md)\n")
	write(t, root, p+"artifacts/moved-by-hand.md", "---\ntasks: []\n---\n\n[概要](overview.md)\n")
	write(t, root, p+"artifacts/no-tasks.md", "---\ntitle: x\n---\n# tasks が無い\n")

	out, _, _ := run(t, "check", "--fix")
	if c := read(t, root, p+"tasks/2026-11-02-no-fm.md"); !strings.HasPrefix(c, "---\nstatus:\nowner:\n---\n\n# 人が書いたタスク") {
		t.Errorf("frontmatter を足していない（値は埋めない）:\n%s", c)
	}
	// status と owner の値は手で書くので残る
	fs, code := checkJSON(t)
	if code != 1 || len(fs) != 2 || !hasFinding(fs, p+"tasks/2026-11-02-no-fm.md", "status が空") || !hasFinding(fs, p+"tasks/2026-11-02-no-fm.md", "owner が空") {
		t.Errorf("--fix の後に残るのが status・owner の空だけでない（code=%d）: %+v", code, fs)
	}
	moved := p + "archive/tasks/2026-11-04-closed.md"
	if !exists(root, moved) || exists(root, p+"tasks/2026-11-04-closed.md") {
		t.Error("done のタスクを archive に移していない")
	} else if c := read(t, root, moved); !strings.Contains(c, "(../../artifacts/print-layout.md)") {
		t.Errorf("移したタスクのリンク:\n%s", c)
	}
	if c := read(t, root, p+"artifacts/moved-by-hand.md"); !strings.Contains(c, "(../wiki/meal-planner/overview.md)") {
		t.Errorf("切れたリンクを付け替えていない:\n%s", c)
	}
	if c := read(t, root, p+"artifacts/no-tasks.md"); !strings.Contains(c, "tasks: []") {
		t.Errorf("必須キーを足していない:\n%s", c)
	}
	for _, want := range []string{"直した:", moved, "git -C"} {
		if !strings.Contains(out, want) {
			t.Errorf("出力に %q が無い:\n%s", want, out)
		}
	}
}

func TestCheckFixLeavesManualFindings(t *testing.T) {
	root := recipeTree(t)
	write(t, root, "projects/recipe-app/knowledge/empty.md", "# 中身だけ\n")
	_, _, code := run(t, "check", "--fix")
	if code != 1 {
		t.Errorf("手で直すもの（description・principle が空）が残るのに終了コードが %d", code)
	}
	if c := read(t, root, "projects/recipe-app/knowledge/empty.md"); !strings.HasPrefix(c, "---\ndescription:\nprinciple:\n---\n") {
		t.Errorf("空のキーを足していない:\n%s", c)
	}
}

func TestCheckRules(t *testing.T) {
	out := mustRun(t, "check", "--rules")
	for _, want := range []string{"正本はスキル desk", "task —", "status: 必須", "backlog | todo | inprogress | done | dropped", "YYYY-MM-DD-<slug>.md", "projects/<p>/tasks/"} {
		if !strings.Contains(out, want) {
			t.Errorf("--rules に %q が無い:\n%s", want, out)
		}
	}
}

func TestCheckOutsideTreeFails(t *testing.T) {
	outsideTreeFails(t, []string{"check"}, []string{"check", "--fix"})
}

func TestKnowledgeWithSeveralPrinciples(t *testing.T) {
	root := recipeTree(t)
	write(t, root, "principles/single-source-of-truth.md", "---\ndescription: 1 つの事実は 1 か所に置く\n---\n\n# 事実は 1 か所\n")
	write(t, root, "knowledge/derive-state-from-input.md",
		"---\ndescription: 画面の状態を直すときに読む。状態を入力から導く\nprinciple: [../principles/prefer-structure-over-patches.md, ../principles/single-source-of-truth.md]\nsources: []\n---\n\n# 状態は入力から導く\n")

	if fs, code := checkJSON(t); code != 0 || len(fs) != 0 {
		t.Errorf("principle がリストのナレッジで挙がる（code=%d）: %+v", code, fs)
	}
	for _, p := range []string{"principles/prefer-structure-over-patches.md", "principles/single-source-of-truth.md"} {
		out := mustRun(t, "graph", p, "--in", "--via", "principle", "--format", "paths")
		if !strings.Contains(out, "knowledge/derive-state-from-input.md") {
			t.Errorf("%s から指しているナレッジが出ない:\n%s", p, out)
		}
	}
	out := mustRun(t, "ls", "knowledge", "--where", "principle=single-source-of-truth", "--paths")
	if strings.TrimSpace(out) != "knowledge/derive-state-from-input.md" {
		t.Errorf("--where principle= で 2 つ目の原則から引けない:\n%s", out)
	}
}

func TestKnowledgeDetailNeedsNoPrinciple(t *testing.T) {
	root := recipeTree(t)
	write(t, root, "knowledge/derive-state-from-input.md",
		"---\ndescription: 画面の状態を直すときに読む。状態を入力から導く\nprinciple: ../principles/prefer-structure-over-patches.md\nsources: []\n---\n\n# 状態は入力から導く\n\n手順は [詳細](derive-state-from-input-steps.md)。\n")
	write(t, root, "knowledge/derive-state-from-input-steps.md",
		"---\ndescription: derive-state-from-input.md から指されたときだけ読む。導き方の手順\n---\n\n# 導き方の手順\n")
	write(t, root, "knowledge/orphan.md", "---\ndescription: どこからも指されていない\n---\n\n# 指されていない\n")
	write(t, root, "projects/recipe-app/materials/2026-11-10-no-source.md",
		"---\nknowledge: []\n---\n\n# ツリーに場所が無い出来事\n\n## 何が起きたか\nx\n\n## 一般化の候補\n- x\n")

	fs, code := checkJSON(t)
	if code != 1 {
		t.Errorf("principle の無いナレッジがあるのに終了コードが %d", code)
	}
	for _, f := range fs {
		if f.Path != "knowledge/orphan.md" {
			t.Errorf("詳細や source の無い素材で挙がる: %+v", f)
		}
	}
	if !hasFinding(fs, "knowledge/orphan.md", "principle が無い") {
		t.Errorf("詳細でないナレッジの principle が無いのに挙がらない: %+v", fs)
	}
}
