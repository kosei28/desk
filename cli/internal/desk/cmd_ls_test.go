package desk

import (
	"encoding/json"
	"strings"
	"testing"
)

func TestLsFilters(t *testing.T) {
	root := recipeTree(t)
	write(t, root, "projects/recipe-app/tasks/2026-11-10-decide-paper-size.md",
		"---\nstatus: todo\nneeds_human: true\nparent: 2026-11-09-printable-menu.md\n---\n\n# 用紙の大きさを決める\n\n## 目的\n選ぶ。\n\n## 成果物\n- なし\n")
	write(t, root, "projects/recipe-app/archive/tasks/2026-09-07-favorites.md", taskDoc("done", "", "お気に入り"))

	out := mustRun(t, "ls", "task", "--where", "needs_human=true", "--paths")
	if strings.TrimSpace(out) != "projects/recipe-app/tasks/2026-11-10-decide-paper-size.md" {
		t.Errorf("人宛て:\n%s", out)
	}

	out = mustRun(t, "ls", "task", "--paths")
	if strings.Contains(out, "archive/") {
		t.Errorf("--archived なしで archive/ が入る:\n%s", out)
	}
	if out := mustRun(t, "ls", "task", "--archived", "--paths"); !strings.Contains(out, "archive/tasks/2026-09-07-favorites.md") {
		t.Errorf("--archived で archive/ が入らない:\n%s", out)
	}

	out = mustRun(t, "ls", "task", "--where", "parent=2026-09-07-meal-planner", "--paths")
	if strings.TrimSpace(out) != "projects/recipe-app/tasks/2026-11-09-printable-menu.md" {
		t.Errorf("parent の名前で引けない:\n%s", out)
	}

	out = mustRun(t, "ls", "artifact", "--where", "tasks~printable-menu", "--paths")
	if strings.TrimSpace(out) != "projects/recipe-app/artifacts/print-layout.md" {
		t.Errorf("tasks の部分一致:\n%s", out)
	}

	out = mustRun(t, "ls", "knowledge", "--where", "sources=[]", "--paths")
	if strings.TrimSpace(out) != "projects/recipe-app/knowledge/api-error-format.md" {
		t.Errorf("空の配列で引けない:\n%s", out)
	}

	out = mustRun(t, "ls", "principle", "--project", "recipe-app", "--paths")
	if strings.TrimSpace(out) != "principles/prefer-structure-over-patches.md" {
		t.Errorf("--project でルートの原則が入らない:\n%s", out)
	}

	out = mustRun(t, "ls", "project", "--paths")
	if !strings.Contains(out, "projects/recipe-app") || !strings.Contains(out, "projects/misc") {
		t.Errorf("プロジェクトの一覧:\n%s", out)
	}

	var rows []map[string]any
	if err := json.Unmarshal([]byte(mustRun(t, "ls", "wiki", "--json")), &rows); err != nil || len(rows) != 1 || rows[0]["path"] != "projects/recipe-app/wiki/meal-planner/overview.md" {
		t.Errorf("--json: %v %v", rows, err)
	}

	if _, _, code := run(t, "ls", "nothing"); code == 0 {
		t.Error("知らない種類で通る")
	}
}

func TestParseWhere(t *testing.T) {
	c, err := parseWhere("status!=done")
	if err != nil || c != (cond{key: "status", op: "!=", val: "done"}) {
		t.Errorf("%+v %v", c, err)
	}
	if _, err := parseWhere("=x"); err == nil {
		t.Error("キーが無いのに通る")
	}
}

func TestTableAlignsWideCharacters(t *testing.T) {
	var b strings.Builder
	table(&b, []string{"title", "status"}, [][]string{{"献立プランナー", "active"}, {"ab", ""}})
	lines := strings.Split(strings.TrimRight(b.String(), "\n"), "\n")
	want := []string{
		"title           status",
		"献立プランナー  active",
		"ab              -",
	}
	if strings.Join(lines, "\n") != strings.Join(want, "\n") {
		t.Errorf("列がそろっていない:\n%s", b.String())
	}
	if got := displayWidth("献立a"); got != 5 {
		t.Errorf("displayWidth = %d", got)
	}
}

func TestMoveAndListOutsideTreeFail(t *testing.T) {
	outsideTreeFails(t,
		[]string{"ls"},
		[]string{"graph", "--all"},
		[]string{"archive", "x.md"},
		[]string{"mv", "a.md", "b.md"},
	)
}
