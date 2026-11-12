package desk

import (
	"strings"
	"testing"
)

func TestGraph(t *testing.T) {
	root := recipeTree(t)
	write(t, root, "projects/recipe-app/tasks/2026-11-11-print-preview.md",
		taskDoc("todo", "parent: 2026-11-09-printable-menu.md", "印刷のプレビューを出す"))

	out := mustRun(t, "graph", "2026-09-07-meal-planner", "--in", "--via", "parent", "--depth", "all", "--format", "paths")
	want := "projects/recipe-app/tasks/2026-11-09-printable-menu.md\nprojects/recipe-app/tasks/2026-11-11-print-preview.md\n"
	if out != want {
		t.Errorf("子孫:\n%s\nwant\n%s", out, want)
	}

	out = mustRun(t, "graph", "2026-11-09-printable-menu", "--in", "--format", "paths")
	if !strings.Contains(out, "artifacts/print-layout.md") || !strings.Contains(out, "2026-11-11-print-preview.md") {
		t.Errorf("指しているもの:\n%s", out)
	}

	out = mustRun(t, "graph", "projects/recipe-app/artifacts/print-layout.md", "--out")
	if !strings.Contains(out, "→ tasks  projects/recipe-app/tasks/2026-11-09-printable-menu.md") || !strings.Contains(out, "→ body  projects/recipe-app/artifacts/assets/print-sample.png") {
		t.Errorf("指す先:\n%s", out)
	}

	out = mustRun(t, "graph", "--all", "--format", "dot")
	if !strings.HasPrefix(out, "digraph desk {") || !strings.Contains(out, `[label="principle"]`) {
		t.Errorf("dot:\n%s", out)
	}

	if _, _, code := run(t, "graph", "x", "--in", "--out"); code == 0 {
		t.Error("--in と --out を両方付けて通る")
	}
}
