package desk

import (
	"strings"
	"testing"
)

func TestReadLevels(t *testing.T) {
	recipeTree(t)
	task := "projects/recipe-app/tasks/2026-11-09-printable-menu.md"

	out := mustRun(t, "read", task, "--level", "0")
	if !strings.Contains(out, "[task · recipe-app · inprogress · ai]") || !strings.Contains(out, "いま: いま。") {
		t.Errorf("段 0:\n%s", out)
	}

	out = mustRun(t, "read", "2026-11-09-printable-menu", "--section", "目的,いま")
	if !strings.Contains(out, "## 目的") || !strings.Contains(out, "## いま") || strings.Contains(out, "## ログ") {
		t.Errorf("節で切れていない:\n%s", out)
	}

	out = mustRun(t, "read", task, "--outline")
	if !strings.Contains(out, "ログ") || strings.Contains(out, "印刷用のページ") {
		t.Errorf("見出しだけになっていない:\n%s", out)
	}

	out = mustRun(t, "read", "projects/recipe-app/wiki/meal-planner/overview.md", "--level", "2")
	if !strings.Contains(out, "--- 指す先 ---") || !strings.Contains(out, "==> projects/recipe-app/artifacts/print-layout.md <==") {
		t.Errorf("段 2 で指す先を読まない:\n%s", out)
	}
}

func TestReadAmbiguousName(t *testing.T) {
	root := recipeTree(t)
	write(t, root, "projects/misc/artifacts/print-layout.md", "---\ntasks: []\n---\n# 別の\n")
	_, errOut, code := run(t, "read", "print-layout")
	if code == 0 || !strings.Contains(errOut, "複数ある") {
		t.Errorf("名前が複数に当たるのに止まらない: %s", errOut)
	}
	t.Chdir(root + "/projects/recipe-app")
	if out := mustRun(t, "read", "print-layout", "--level", "0"); !strings.Contains(out, "projects/recipe-app/artifacts/print-layout.md") {
		t.Errorf("作業ディレクトリのプロジェクトで絞れていない:\n%s", out)
	}
}
