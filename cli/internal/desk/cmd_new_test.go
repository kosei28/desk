package desk

import (
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"testing"
)

func TestInitCreatesTree(t *testing.T) {
	isolateGit(t)
	root := filepath.Join(tempDir(t), "notes")
	mustRun(t, "init", root)
	for _, rel := range []string{".desk", "README.md", ".gitignore", "projects/misc/README.md"} {
		if !exists(root, rel) {
			t.Errorf("%s が無い", rel)
		}
	}
	for _, rel := range []string{"principles", "knowledge", "archive"} {
		if exists(root, rel) {
			t.Errorf("空のディレクトリ %s を作っている", rel)
		}
	}
	if !strings.Contains(read(t, root, "README.md"), "projects/misc/inbox/") {
		t.Error("README に人向けの受け箱の場所が無い")
	}
	log, err := exec.Command("git", "-C", root, "log", "--format=%s").Output()
	if err != nil || strings.TrimSpace(string(log)) != "docs: desk を作る" {
		t.Errorf("最初のコミットが無い: %q %v", log, err)
	}

	out := mustRun(t, "init", root)
	if !strings.Contains(out, "もうツリーのルート") {
		t.Errorf("2 回目は何も作らないはず:\n%s", out)
	}
}

func TestInitDefaultsToWorkingDirectory(t *testing.T) {
	isolateGit(t)
	dir := tempDir(t)
	t.Chdir(dir)
	mustRun(t, "init")
	if !exists(dir, ".desk") || !exists(dir, "projects/misc/README.md") {
		t.Error("作業ディレクトリに作られていない")
	}
}

func TestInitInExistingRepository(t *testing.T) {
	isolateGit(t)
	dir := tempDir(t)
	write(t, dir, "README.md", "# 前からある\n")
	for _, args := range [][]string{{"init", "-q", "-b", "main"}, {"add", "README.md"}, {"commit", "-q", "-m", "first"}} {
		if out, err := exec.Command("git", append([]string{"-C", dir}, args...)...).CombinedOutput(); err != nil {
			t.Fatalf("git %v: %v %s", args, err, out)
		}
	}
	out := mustRun(t, "init", dir)
	if read(t, dir, "README.md") != "# 前からある\n" {
		t.Error("あるファイルを書き換えている")
	}
	if !exists(dir, ".desk") || !strings.Contains(out, "コミットはしていない") {
		t.Errorf("印を置いてコミットせずに案内するはず:\n%s", out)
	}
	if log, _ := exec.Command("git", "-C", dir, "log", "--format=%s").Output(); strings.TrimSpace(string(log)) != "first" {
		t.Errorf("コミットを足している: %q", log)
	}
}

func TestInitInsideTreeFails(t *testing.T) {
	root := newTree(t)
	inner := filepath.Join(root, "projects", "misc", "nested")
	_, errOut, code := run(t, "init", inner)
	if code == 0 || !strings.Contains(errOut, "ツリーの中にツリーは作らない") {
		t.Errorf("ツリーの中に作れてしまう: code=%d %s", code, errOut)
	}
	if exists(root, "projects/misc/nested") {
		t.Error("止まったのにディレクトリを作っている")
	}
}

func TestRootDiscovery(t *testing.T) {
	root := newTree(t)
	sub := filepath.Join(root, "projects", "misc")
	t.Chdir(sub)
	if got := strings.TrimSpace(mustRun(t, "root")); got != root {
		t.Errorf("ツリーの中: root = %q, want %q", got, root)
	}
	if got := mustRun(t, "read", "projects/misc/README.md"); !strings.Contains(got, "# misc") {
		t.Errorf("サブディレクトリから読めない: %q", got)
	}
}

func TestOutsideTreeFails(t *testing.T) {
	outsideTreeFails(t,
		[]string{"root"},
		[]string{"read", "x.md"},
		[]string{"new", "project", "recipe-app"},
		[]string{"new", "task", "x", "--project", "misc"},
	)
}

func TestNewProjectAndTasks(t *testing.T) {
	root := newTree(t)
	mustRun(t, "new", "project", "recipe-app")
	if !exists(root, "projects/recipe-app/README.md") {
		t.Fatal("プロジェクトの README が無い")
	}
	if _, _, code := run(t, "new", "project", "recipe-app"); code == 0 {
		t.Error("同じプロジェクトを 2 回作れてしまう")
	}

	out := mustRun(t, "new", "task", "2026-09-07-meal-planner", "--project", "recipe-app", "--title", "献立プランナー")
	parent := "projects/recipe-app/tasks/2026-09-07-meal-planner.md"
	if strings.TrimSpace(out) != filepath.Join(root, parent) {
		t.Errorf("作ったパスを出さない: %q", out)
	}
	c := read(t, root, parent)
	for _, want := range []string{"status: todo", "needs_human: false", "# 献立プランナー", "## 目的", "## いま", "## 成果物", "## 現在地", "## 決まり", "## 手順"} {
		if !strings.Contains(c, want) {
			t.Errorf("タスクに %q が無い:\n%s", want, c)
		}
	}

	mustRun(t, "new", "task", "2026-11-10-decide-paper-size", "--parent", "2026-09-07-meal-planner", "--needs-human")
	child := read(t, root, "projects/recipe-app/tasks/2026-11-10-decide-paper-size.md")
	if !strings.Contains(child, "parent: 2026-09-07-meal-planner.md") || !strings.Contains(child, "needs_human: true") || strings.Contains(child, "## 現在地") {
		t.Errorf("人の子タスクの形が違う:\n%s", child)
	}

	mustRun(t, "new", "artifact", "print-layout", "--parent", "2026-09-07-meal-planner")
	if !strings.Contains(read(t, root, "projects/recipe-app/artifacts/print-layout.md"), "tasks: [../tasks/2026-09-07-meal-planner.md]") {
		t.Error("artifact の tasks が親を指さない")
	}

	mustRun(t, "new", "knowledge", "reproduce-before-fixing")
	if !exists(root, "knowledge/reproduce-before-fixing.md") {
		t.Error("プロジェクトを付けないナレッジはルートに置く")
	}

	if _, errOut, code := run(t, "new", "task", "x", "--project", "nothing"); code == 0 || !strings.Contains(errOut, "プロジェクト nothing が無い") {
		t.Errorf("無いプロジェクトに作れてしまう: %s", errOut)
	}
	if _, _, code := run(t, "new", "task", "Bad_Slug", "--project", "recipe-app"); code == 0 {
		t.Error("決まりの外の slug で作れてしまう")
	}
}

func TestNewRefusesNameUsedInArchive(t *testing.T) {
	root := newTree(t)
	write(t, root, "projects/recipe-app/archive/tasks/2026-09-07-favorites.md", taskDoc("done", "", "お気に入り"))
	if _, errOut, code := run(t, "new", "task", "2026-09-07-favorites", "--project", "recipe-app"); code == 0 || !strings.Contains(errOut, "既にある") {
		t.Errorf("archive/ にある名前で作れてしまう: %s", errOut)
	}
	if _, err := os.Stat(filepath.Join(root, "projects/recipe-app/tasks/2026-09-07-favorites.md")); err == nil {
		t.Error("ファイルが作られている")
	}
}
