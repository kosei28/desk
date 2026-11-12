package desk

import (
	"os/exec"
	"strings"
	"testing"
)

func gitAddAll(t *testing.T, root string) {
	t.Helper()
	for _, args := range [][]string{{"add", "-A"}, {"commit", "-q", "-m", "fixture"}} {
		if out, err := exec.Command("git", append([]string{"-C", root}, args...)...).CombinedOutput(); err != nil {
			t.Fatalf("git %v: %v\n%s", args, err, out)
		}
	}
}

func TestArchiveMovesAndRewritesLinks(t *testing.T) {
	root := recipeTree(t)
	gitAddAll(t, root)
	task := "projects/recipe-app/tasks/2026-11-09-printable-menu.md"

	out := mustRun(t, "archive", task, "projects/recipe-app/artifacts/print-layout.md", "projects/recipe-app/artifacts/assets/print-sample.png", "--dry-run")
	if !strings.Contains(out, "動かす:") || !exists(root, task) {
		t.Fatalf("--dry-run で動いた、または計画を出さない:\n%s", out)
	}

	mustRun(t, "archive", task, "projects/recipe-app/artifacts/print-layout.md", "projects/recipe-app/artifacts/assets/print-sample.png")
	moved := "projects/recipe-app/archive/tasks/2026-11-09-printable-menu.md"
	if exists(root, task) || !exists(root, moved) || !exists(root, "projects/recipe-app/archive/artifacts/assets/print-sample.png") {
		t.Fatal("動いていない")
	}
	c := read(t, root, moved)
	if !strings.Contains(c, "status: done") {
		t.Errorf("status が done にならない:\n%s", c)
	}
	if !strings.Contains(c, "parent: ../../tasks/2026-09-07-meal-planner.md") {
		t.Errorf("動かしたファイルの中のリンクを付け替えていない:\n%s", c)
	}
	a := read(t, root, "projects/recipe-app/archive/artifacts/print-layout.md")
	if !strings.Contains(a, "tasks: [../tasks/2026-11-09-printable-menu.md]") || !strings.Contains(a, "![](assets/print-sample.png)") || !strings.Contains(a, "(../../tasks/2026-09-07-meal-planner.md)") {
		t.Errorf("一緒に動いたもの同士・外へのリンク:\n%s", a)
	}
	w := read(t, root, "projects/recipe-app/wiki/meal-planner/overview.md")
	if !strings.Contains(w, "(../../archive/artifacts/print-layout.md)") {
		t.Errorf("外から指すリンクを付け替えていない:\n%s", w)
	}
	if exists(root, "projects/recipe-app/artifacts") {
		t.Error("空になったディレクトリが残る")
	}
	if status, _ := exec.Command("git", "-C", root, "status", "--porcelain").Output(); !strings.Contains(string(status), task+" -> "+moved) {
		t.Errorf("git mv で動かしていない:\n%s", status)
	}

	if out := mustRun(t, "archive", moved); !strings.Contains(out, "既に archive/ にある") {
		t.Errorf("2 回目:\n%s", out)
	}
	if _, _, code := run(t, "archive", "projects/recipe-app/wiki/meal-planner/overview.md"); code == 0 {
		t.Error("wiki を archive に移せてしまう")
	}
}

func TestArchiveProject(t *testing.T) {
	root := recipeTree(t)
	write(t, root, "projects/misc/artifacts/notes.md", "---\ntasks: []\n---\n\n[概要](../../recipe-app/wiki/meal-planner/overview.md)\n")
	mustRun(t, "archive", "--project", "recipe-app")
	if exists(root, "projects/recipe-app") || !exists(root, "archive/projects/recipe-app/wiki/meal-planner/overview.md") {
		t.Fatal("プロジェクトが動いていない")
	}
	if c := read(t, root, "archive/projects/recipe-app/knowledge/api-error-format.md"); !strings.Contains(c, "principle: ../../../../principles/prefer-structure-over-patches.md") {
		t.Errorf("中から外へのリンク:\n%s", c)
	}
	if c := read(t, root, "archive/projects/recipe-app/artifacts/print-layout.md"); !strings.Contains(c, "tasks: [../tasks/2026-11-09-printable-menu.md]") {
		t.Errorf("中同士のリンクが変わった:\n%s", c)
	}
	if c := read(t, root, "projects/misc/artifacts/notes.md"); !strings.Contains(c, "(../../../archive/projects/recipe-app/wiki/meal-planner/overview.md)") {
		t.Errorf("外から指すリンク:\n%s", c)
	}
}

func TestMvRenameAndPromote(t *testing.T) {
	root := recipeTree(t)
	mustRun(t, "mv", "projects/recipe-app/artifacts/print-layout.md", "projects/recipe-app/artifacts/print-columns.md")
	if c := read(t, root, "projects/recipe-app/wiki/meal-planner/overview.md"); !strings.Contains(c, "(../../artifacts/print-columns.md)") {
		t.Errorf("名前の変更でリンクを付け替えていない:\n%s", c)
	}

	mustRun(t, "mv", "projects/recipe-app/knowledge/api-error-format.md", "knowledge/")
	if c := read(t, root, "knowledge/api-error-format.md"); !strings.Contains(c, "principle: ../principles/prefer-structure-over-patches.md") {
		t.Errorf("ルートに移したナレッジのリンク:\n%s", c)
	}

	if _, errOut, code := run(t, "mv", "projects/recipe-app/artifacts/print-columns.md", "projects/recipe-app/drafts/"); code == 0 || !strings.Contains(errOut, "決まりの外") {
		t.Errorf("決まりの外へ動かせてしまう: %s", errOut)
	}
}
