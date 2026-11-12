package desk

import (
	"bytes"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// run は CLI を引数どおりに動かし、標準出力・標準エラー・終了コードを返す。
func run(t *testing.T, args ...string) (string, string, int) {
	t.Helper()
	var out, errb bytes.Buffer
	code := Run(args, strings.NewReader(""), &out, &errb)
	return out.String(), errb.String(), code
}

// mustRun は終了コード 0 を期待して動かす。
func mustRun(t *testing.T, args ...string) string {
	t.Helper()
	out, errOut, code := run(t, args...)
	if code != 0 {
		t.Fatalf("desk %s: 終了コード %d\nstdout:\n%s\nstderr:\n%s", strings.Join(args, " "), code, out, errOut)
	}
	return out
}

// isolateGit は git がユーザーの設定を読まないようにし、コミットの作者を決める。
func isolateGit(t *testing.T) {
	t.Helper()
	t.Setenv("GIT_CONFIG_GLOBAL", os.DevNull)
	t.Setenv("GIT_CONFIG_NOSYSTEM", "1")
	t.Setenv("GIT_AUTHOR_NAME", "test")
	t.Setenv("GIT_AUTHOR_EMAIL", "test@example.com")
	t.Setenv("GIT_COMMITTER_NAME", "test")
	t.Setenv("GIT_COMMITTER_EMAIL", "test@example.com")
}

// tempDir はシンボリックリンクを解いた一時ディレクトリ。
func tempDir(t *testing.T) string {
	t.Helper()
	d, err := filepath.EvalSymlinks(t.TempDir())
	if err != nil {
		t.Fatal(err)
	}
	return d
}

// newTree は desk init でツリーを作り、作業ディレクトリをそのルートにする。
func newTree(t *testing.T) string {
	t.Helper()
	isolateGit(t)
	root := filepath.Join(tempDir(t), "desk")
	mustRun(t, "init", root)
	t.Chdir(root)
	return root
}

// write はルートからのパスにファイルを書く。
func write(t *testing.T, root, rel, content string) {
	t.Helper()
	p := filepath.Join(root, filepath.FromSlash(rel))
	if err := os.MkdirAll(filepath.Dir(p), 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(p, []byte(content), 0o644); err != nil {
		t.Fatal(err)
	}
}

// read はルートからのパスのファイルを読む。
func read(t *testing.T, root, rel string) string {
	t.Helper()
	b, err := os.ReadFile(filepath.Join(root, filepath.FromSlash(rel)))
	if err != nil {
		t.Fatal(err)
	}
	return string(b)
}

func exists(root, rel string) bool {
	_, err := os.Stat(filepath.Join(root, filepath.FromSlash(rel)))
	return err == nil
}

func taskDoc(status, extraFM, title string) string {
	fm := "status: " + status + "\n"
	if !strings.Contains(extraFM, "owner:") {
		fm += "owner: ai\n"
	}
	if extraFM != "" {
		fm += extraFM + "\n"
	}
	return "---\n" + fm + "---\n\n# " + title + "\n\n## 目的\n目的。\n\n## いま\nいま。\n\n## 成果物\n- なし\n\n## 現在地\n次の一手。\n\n## 決まり\nなし。\n\n## 手順\n- [ ] 手順\n"
}

// recipeTree は架空のレシピアプリのプロジェクトを持つツリーを作る。
func recipeTree(t *testing.T) string {
	t.Helper()
	root := newTree(t)
	write(t, root, "principles/prefer-structure-over-patches.md", "---\ndescription: 不具合は起きない構造にして直す\n---\n\n# 構造で直す\n")
	write(t, root, "projects/recipe-app/README.md", "# recipe-app\n\n架空のレシピアプリ。\n")
	write(t, root, "projects/recipe-app/tasks/2026-09-07-meal-planner.md", taskDoc("inprogress", "", "献立プランナー"))
	write(t, root, "projects/recipe-app/tasks/2026-11-09-printable-menu.md",
		taskDoc("inprogress", "parent: 2026-09-07-meal-planner.md", "献立を印刷できるようにする")+
			"- [印刷のレイアウト](../artifacts/print-layout.md)\n\n## ログ\n### 2026-11-10\n- 決定: 印刷用のページは画面と別に作る\n")
	write(t, root, "projects/recipe-app/artifacts/print-layout.md",
		"---\ntasks: [../tasks/2026-11-09-printable-menu.md]\n---\n\n# 印刷のレイアウト\n\n![](assets/print-sample.png)\n\n親: [献立プランナー](../tasks/2026-09-07-meal-planner.md)\n")
	write(t, root, "projects/recipe-app/artifacts/assets/print-sample.png", "png")
	write(t, root, "projects/recipe-app/wiki/meal-planner/overview.md",
		"---\ndescription: 献立プランナーの仕組み全体\n---\n\n# 献立プランナー\n\n印刷: [レイアウト](../../artifacts/print-layout.md)\n")
	write(t, root, "projects/recipe-app/knowledge/api-error-format.md",
		"---\ndescription: API のエラーを返すときに読む\nprinciple: ../../../principles/prefer-structure-over-patches.md\nsources: []\n---\n\n# エラーの形\n")
	return root
}

// outsideTreeFails は、ツリーの外で実行したコマンドが何も書かずにエラーで止まることを確かめる。
func outsideTreeFails(t *testing.T, cmds ...[]string) {
	t.Helper()
	isolateGit(t)
	outside := tempDir(t)
	t.Chdir(outside)
	for _, args := range cmds {
		_, errOut, code := run(t, args...)
		if code == 0 || !strings.Contains(errOut, "ツリーの中ではない") {
			t.Errorf("desk %v がツリーの外で止まらない: code=%d %s", args, code, errOut)
		}
	}
	if entries, err := os.ReadDir(outside); err != nil || len(entries) != 0 {
		t.Errorf("ツリーの外に書いている: %v %v", entries, err)
	}
}
