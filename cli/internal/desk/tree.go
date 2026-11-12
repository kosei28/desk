package desk

import (
	"errors"
	"fmt"
	"io/fs"
	"os"
	"os/exec"
	"path"
	"path/filepath"
	"sort"
	"strings"
	"time"
)

// Tree はツリー全体。読むたびに走査し、保存しない。
type Tree struct {
	Root   string // 絶対パス
	Docs   []*Doc // パス順
	ByPath map[string]*Doc
	dates  map[string]string
}

// FindRoot はルートを決める。作業ディレクトリから上にたどって .desk のあるディレクトリ。
// 見つからなければエラー。
func FindRoot() (string, error) {
	cwd, err := os.Getwd()
	if err != nil {
		return "", err
	}
	if r := findMarker(cwd); r != "" {
		return r, nil
	}
	return "", fmt.Errorf("%s はツリーの中ではない（上にたどっても %s が無い）。desk はツリーの中で実行する（新しく作るなら desk init）", cwd, MarkerFile)
}

// findMarker は dir から上にたどって .desk のあるディレクトリを返す。無ければ空。
func findMarker(dir string) string {
	for {
		if st, err := os.Stat(filepath.Join(dir, MarkerFile)); err == nil && !st.IsDir() {
			return dir
		}
		up := filepath.Dir(dir)
		if up == dir {
			return ""
		}
		dir = up
	}
}

func expandHome(p string) string {
	if p == "~" || strings.HasPrefix(p, "~/") {
		if home, err := os.UserHomeDir(); err == nil {
			return filepath.Join(home, strings.TrimPrefix(p, "~"))
		}
	}
	return p
}

// LoadTree はルートの下を全部読む。. で始まる名前は見ない。
func LoadTree(root string) (*Tree, error) {
	st, err := os.Stat(root)
	if err != nil {
		return nil, fmt.Errorf("ルート %s が無い（desk init で作る）", root)
	}
	if !st.IsDir() {
		return nil, fmt.Errorf("ルート %s がディレクトリでない", root)
	}
	t := &Tree{Root: root, ByPath: map[string]*Doc{}}
	err = filepath.WalkDir(root, func(p string, e fs.DirEntry, err error) error {
		if err != nil {
			return err
		}
		if p == root {
			return nil
		}
		if strings.HasPrefix(e.Name(), ".") {
			if e.IsDir() {
				return filepath.SkipDir
			}
			return nil
		}
		if e.IsDir() || e.Type()&fs.ModeSymlink != 0 && !isFile(p) {
			return nil
		}
		rel, _ := filepath.Rel(root, p)
		rel = filepath.ToSlash(rel)
		info := Classify(rel)
		var d *Doc
		if strings.HasSuffix(rel, ".md") && !info.Asset {
			b, err := os.ReadFile(p)
			if err != nil {
				return err
			}
			d = ParseDoc(info, string(b))
		} else {
			d = &Doc{Info: info}
		}
		t.Docs = append(t.Docs, d)
		t.ByPath[rel] = d
		return nil
	})
	if err != nil {
		return nil, err
	}
	sort.Slice(t.Docs, func(i, j int) bool { return t.Docs[i].Path < t.Docs[j].Path })
	return t, nil
}

func isFile(p string) bool {
	st, err := os.Stat(p)
	return err == nil && st.Mode().IsRegular()
}

// Abs はルートからのパスを絶対パスにする。
func (t *Tree) Abs(rel string) string { return filepath.Join(t.Root, filepath.FromSlash(rel)) }

// Exists はルートからのパス（ファイルかディレクトリ）があるか。
func (t *Tree) Exists(rel string) bool {
	if _, ok := t.ByPath[rel]; ok {
		return true
	}
	if rel == "." {
		return true
	}
	_, err := os.Stat(t.Abs(rel))
	return err == nil
}

// Rel は絶対パスをルートからのパスにする。ルートの外なら ok=false。
func (t *Tree) Rel(abs string) (string, bool) {
	rel, err := filepath.Rel(t.Root, abs)
	if err != nil {
		return "", false
	}
	rel = filepath.ToSlash(rel)
	if rel == ".." || strings.HasPrefix(rel, "../") {
		return "", false
	}
	return rel, true
}

// Projects はプロジェクト（archived なら archive/ に移ったものも）。
func (t *Tree) Projects(archived bool) []Info {
	var out []Info
	add := func(arch bool) {
		parent := ProjectsDir
		if arch {
			parent = ArchiveDir + "/" + ProjectsDir
		}
		entries, err := os.ReadDir(t.Abs(parent))
		if err != nil {
			return
		}
		for _, e := range entries {
			if !e.IsDir() || strings.HasPrefix(e.Name(), ".") {
				continue
			}
			out = append(out, Info{Kind: KindProject, Project: e.Name(), Path: projectDir(e.Name(), arch), Archived: arch, ProjectArchived: arch})
		}
	}
	add(false)
	if archived {
		add(true)
	}
	return out
}

// FindByName はファイル名（.md は省略可）で引く。
func (t *Tree) FindByName(name string) []*Doc {
	var out []*Doc
	for _, d := range t.Docs {
		b := path.Base(d.Path)
		if b == name || b == name+".md" {
			out = append(out, d)
		}
	}
	return out
}

// ErrAmbiguous は名前が複数に当たったとき。
type ErrAmbiguous struct {
	Name       string
	Candidates []string
}

func (e *ErrAmbiguous) Error() string {
	return fmt.Sprintf("%s は複数ある。パスで指定する:\n  %s", e.Name, strings.Join(e.Candidates, "\n  "))
}

// Resolve は引数（作業ディレクトリからのパス、ルートからのパス、名前）をルートからのパスにする。
func (t *Tree) Resolve(arg string) (string, error) {
	arg = expandHome(arg)
	if filepath.IsAbs(arg) {
		rel, ok := t.Rel(filepath.Clean(arg))
		if !ok {
			return "", fmt.Errorf("%s はツリー（%s）の外", arg, t.Root)
		}
		if !t.Exists(rel) {
			return "", fmt.Errorf("%s が無い", arg)
		}
		return rel, nil
	}
	if cwd, err := os.Getwd(); err == nil {
		abs := filepath.Join(cwd, arg)
		if rel, ok := t.Rel(abs); ok && t.Exists(rel) {
			if _, err := os.Stat(abs); err == nil {
				return rel, nil
			}
		}
	}
	clean := path.Clean(filepath.ToSlash(arg))
	if t.Exists(clean) && !strings.HasPrefix(clean, "../") {
		return clean, nil
	}
	if strings.Contains(arg, "/") {
		return "", fmt.Errorf("%s が無い", arg)
	}
	found := t.FindByName(arg)
	if len(found) == 1 {
		return found[0].Path, nil
	}
	if len(found) == 0 {
		return "", fmt.Errorf("%s が無い", arg)
	}
	// 作業ディレクトリがプロジェクトの中なら、そのプロジェクトで絞る
	if cwd, err := os.Getwd(); err == nil {
		if rel, ok := t.Rel(cwd); ok && rel != "." {
			info := Classify(rel + "/x")
			var in []*Doc
			for _, d := range found {
				if d.Project != "" && d.Project == info.Project && d.ProjectArchived == info.ProjectArchived {
					in = append(in, d)
				}
			}
			if len(in) == 1 {
				return in[0].Path, nil
			}
		}
	}
	var c []string
	for _, d := range found {
		c = append(c, d.Path)
	}
	return "", &ErrAmbiguous{Name: arg, Candidates: c}
}

// ResolveDest はまだ無いかもしれない先のパスをルートからのパスにする。
// 作業ディレクトリがツリーの中ならそこから、外ならルートから。
func (t *Tree) ResolveDest(arg string) (string, error) {
	arg = expandHome(arg)
	var abs string
	switch {
	case filepath.IsAbs(arg):
		abs = filepath.Clean(arg)
	default:
		cwd, _ := os.Getwd()
		if _, ok := t.Rel(cwd); ok && cwd != "" {
			abs = filepath.Join(cwd, arg)
		} else {
			abs = filepath.Join(t.Root, arg)
		}
	}
	rel, ok := t.Rel(abs)
	if !ok {
		return "", fmt.Errorf("%s はツリー（%s）の外", arg, t.Root)
	}
	return rel, nil
}

// Modified は最終コミットの日付（YYYY-MM-DD）。コミットされていなければ空。
func (t *Tree) Modified(rel string) string {
	if t.dates == nil {
		t.dates = gitDates(t.Root)
	}
	return t.dates[rel]
}

func gitDates(root string) map[string]string {
	out := map[string]string{}
	cmd := exec.Command("git", "-C", root, "-c", "core.quotepath=false", "log", "--format=%x01%cs", "--name-only", "--no-renames")
	b, err := cmd.Output()
	if err != nil {
		return out
	}
	date := ""
	for _, line := range strings.Split(string(b), "\n") {
		line = strings.TrimSpace(line)
		if line == "" {
			continue
		}
		if strings.HasPrefix(line, "\x01") {
			date = strings.TrimPrefix(line, "\x01")
			continue
		}
		if _, ok := out[line]; !ok {
			out[line] = date
		}
	}
	return out
}

// RelPath は from のディレクトリから to への相対パス（どちらもルートからのパス）。
func RelPath(fromDir, to string) string {
	r, err := filepath.Rel(filepath.FromSlash(fromDir), filepath.FromSlash(to))
	if err != nil {
		return to
	}
	return filepath.ToSlash(r)
}

// LinkText はリンクに書く文字（空白はエスケープ）。
func LinkText(rel, anchor string) string {
	s := strings.ReplaceAll(rel, " ", "%20")
	if anchor != "" {
		s += "#" + anchor
	}
	return s
}

func today() string { return time.Now().Format("2006-01-02") }

// isGitRepo はルートが git のリポジトリか。
func isGitRepo(root string) bool {
	_, err := os.Stat(filepath.Join(root, ".git"))
	return err == nil
}

// gitTracked はファイルが git で管理されているか。
func gitTracked(root, rel string) bool {
	if !isGitRepo(root) {
		return false
	}
	cmd := exec.Command("git", "-C", root, "ls-files", "--error-unmatch", "--", rel)
	cmd.Stdout, cmd.Stderr = nil, nil
	return cmd.Run() == nil
}

func gitRun(root string, args ...string) error {
	cmd := exec.Command("git", append([]string{"-C", root}, args...)...)
	out, err := cmd.CombinedOutput()
	if err != nil {
		return errors.New(strings.TrimSpace(string(out)))
	}
	return nil
}
