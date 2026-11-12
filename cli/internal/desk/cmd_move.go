package desk

import (
	"fmt"
	"io"
	"os"
	"path"
	"path/filepath"
	"sort"
	"strings"
)

type move struct{ from, to string }

// fileEdit はファイル 1 つの書き換え（行の書き換えと、frontmatter に足す行）。
type fileEdit struct {
	edits   []Edit
	inserts []string // frontmatter の最初に足す行
	notes   []string
}

type moveResult struct {
	moved     []move
	rewritten []string // 書き換えたファイル（動かした後のパス）
}

// planLinkEdits は moves の後もリンクが同じ先を指すように、書き換えを作る。
func planLinkEdits(t *Tree, moves []move) map[string]*fileEdit {
	to := map[string]string{}
	for _, m := range moves {
		to[m.from] = m.to
	}
	out := map[string]*fileEdit{}
	for _, d := range t.Docs {
		if !d.IsMD {
			continue
		}
		newPath, moved := to[d.Path]
		if !moved {
			newPath = d.Path
		}
		for _, l := range d.Links {
			if l.Form != FormRelative || l.Start < 0 {
				continue
			}
			p, _, _ := strings.Cut(l.Raw, "#")
			if p == "" {
				continue
			}
			newTarget, targetMoved := to[l.Target]
			if !targetMoved {
				newTarget = l.Target
			}
			if !moved && !targetMoved {
				continue
			}
			newRaw := LinkText(RelPath(path.Dir(newPath), newTarget), l.Anchor)
			if newRaw == l.Raw {
				continue
			}
			fe := out[d.Path]
			if fe == nil {
				fe = &fileEdit{}
				out[d.Path] = fe
			}
			fe.edits = append(fe.edits, Edit{Line: l.Line, Start: l.Start, Old: l.Raw, New: newRaw})
		}
	}
	return out
}

// statusEdit は task の status を書き換える（無ければ足す）。
func statusEdit(d *Doc, status string, fe *fileEdit) {
	n := d.fmValue("status")
	if n != nil && n.Value == status {
		return
	}
	if n != nil && n.Line > 0 && n.Value != "" {
		if start := d.locate(n.Line, n); start >= 0 {
			fe.edits = append(fe.edits, Edit{Line: n.Line, Start: start, Old: n.Value, New: status})
			return
		}
	}
	if n != nil {
		// 空の status: 行ごと置き換える
		for i := 1; i < d.FMEnd; i++ {
			if strings.HasPrefix(strings.TrimSpace(d.Lines[i]), "status:") {
				fe.edits = append(fe.edits, Edit{Line: i, Start: 0, Old: d.Lines[i], New: "status: " + status})
				return
			}
		}
	}
	fe.inserts = append(fe.inserts, "status: "+status)
}

// writeEdits はファイルに書き換えを当てて保存する。
func writeEdits(t *Tree, d *Doc, fe *fileEdit) error {
	lines, err := ApplyEdits(d.Lines, fe.edits)
	if err != nil {
		return fmt.Errorf("%s: %v", d.Path, err)
	}
	if len(fe.inserts) > 0 {
		if d.HasFM {
			lines = append(lines[:1], append(append([]string{}, fe.inserts...), lines[1:]...)...)
		} else {
			head := append([]string{"---"}, fe.inserts...)
			head = append(head, "---", "")
			lines = append(head, lines...)
		}
	}
	return os.WriteFile(t.Abs(d.Path), []byte(strings.Join(lines, "\n")), 0o644)
}

// performMoves はファイルを動かし、リンクを付け替え、task の status を書く。
func performMoves(w io.Writer, t *Tree, moves []move, status map[string]string, dry bool) (*moveResult, error) {
	// 確かめる
	dest := map[string]bool{}
	src := map[string]bool{}
	for _, m := range moves {
		src[m.from] = true
	}
	for _, m := range moves {
		if _, ok := t.ByPath[m.from]; !ok {
			return nil, fmt.Errorf("%s はファイルでない", m.from)
		}
		if dest[m.to] || (t.Exists(m.to) && !src[m.to]) {
			return nil, fmt.Errorf("%s は既にある", m.to)
		}
		dest[m.to] = true
		if info := Classify(m.to); info.Problem != "" {
			return nil, fmt.Errorf("%s は決まりの外の場所: %s", m.to, info.Problem)
		}
	}

	edits := planLinkEdits(t, moves)
	for p, s := range status {
		fe := edits[p]
		if fe == nil {
			fe = &fileEdit{}
			edits[p] = fe
		}
		statusEdit(t.ByPath[p], s, fe)
	}
	newPath := map[string]string{}
	for _, m := range moves {
		newPath[m.from] = m.to
	}
	var files []string
	for p, fe := range edits {
		if len(fe.edits) == 0 && len(fe.inserts) == 0 {
			continue
		}
		files = append(files, p)
	}
	sort.Strings(files)

	res := &moveResult{moved: moves}
	if dry {
		fmt.Fprintln(w, "動かす:")
		for _, m := range moves {
			fmt.Fprintf(w, "  %s → %s\n", m.from, m.to)
		}
		if len(files) > 0 {
			fmt.Fprintln(w, "書き換える:")
		}
		for _, p := range files {
			fe := edits[p]
			sort.Slice(fe.edits, func(i, j int) bool { return fe.edits[i].Line < fe.edits[j].Line })
			for _, ed := range fe.edits {
				fmt.Fprintf(w, "  %s:%d  %s → %s\n", p, ed.Line+1, ed.Old, ed.New)
			}
			for _, ins := range fe.inserts {
				fmt.Fprintf(w, "  %s  （frontmatter に足す）%s\n", p, ins)
			}
		}
		return res, nil
	}

	for _, p := range files {
		if err := writeEdits(t, t.ByPath[p], edits[p]); err != nil {
			return nil, err
		}
		if np, ok := newPath[p]; ok {
			res.rewritten = append(res.rewritten, np)
		} else {
			res.rewritten = append(res.rewritten, p)
		}
	}
	// 同じパスの入れ替え（a→b、b→a）は無い前提で、順に動かす
	for _, m := range moves {
		if err := os.MkdirAll(filepath.Dir(t.Abs(m.to)), 0o755); err != nil {
			return nil, err
		}
		if gitTracked(t.Root, m.from) {
			if err := gitRun(t.Root, "mv", "--", m.from, m.to); err != nil {
				return nil, fmt.Errorf("git mv %s %s: %v", m.from, m.to, err)
			}
		} else if err := os.Rename(t.Abs(m.from), t.Abs(m.to)); err != nil {
			return nil, err
		}
	}
	for _, m := range moves {
		removeEmptyParents(t, path.Dir(m.from))
	}
	fmt.Fprintln(w, "動かした:")
	for _, m := range moves {
		fmt.Fprintf(w, "  %s → %s\n", m.from, m.to)
	}
	if len(res.rewritten) > 0 {
		fmt.Fprintln(w, "書き換えた（リンク・status）:")
		for _, p := range res.rewritten {
			fmt.Fprintf(w, "  %s\n", p)
		}
	}
	var add []string
	seen := map[string]bool{}
	for _, m := range moves {
		if !seen[m.to] {
			seen[m.to] = true
			add = append(add, m.to)
		}
	}
	for _, p := range res.rewritten {
		if !seen[p] {
			seen[p] = true
			add = append(add, p)
		}
	}
	fmt.Fprintf(w, "コミットするとき: git -C %s add %s\n", displayPath(t.Root), strings.Join(add, " "))
	return res, nil
}

// removeEmptyParents は空になったディレクトリを消す（プロジェクトのディレクトリと、ルートの予約名のディレクトリは残す）。
func removeEmptyParents(t *Tree, dir string) {
	for dir != "." && dir != "" {
		parts := strings.Split(dir, "/")
		switch {
		case len(parts) < 2:
			return
		case parts[0] == ProjectsDir && len(parts) <= 2:
			return
		case parts[0] == ArchiveDir && len(parts) <= 3:
			return
		}
		if !removeIfEmpty(t.Abs(dir)) {
			return
		}
		dir = path.Dir(dir)
	}
}

// removeIfEmpty は .DS_Store しか無いディレクトリを消す。
func removeIfEmpty(abs string) bool {
	entries, err := os.ReadDir(abs)
	if err != nil {
		return false
	}
	for _, e := range entries {
		if e.Name() != ".DS_Store" {
			return false
		}
	}
	os.Remove(filepath.Join(abs, ".DS_Store"))
	return os.Remove(abs) == nil
}

// removeEmptyTree は中のディレクトリが全部空なら消す。
func removeEmptyTree(abs string) bool {
	entries, err := os.ReadDir(abs)
	if err != nil {
		return false
	}
	empty := true
	for _, e := range entries {
		if e.IsDir() {
			if !removeEmptyTree(filepath.Join(abs, e.Name())) {
				empty = false
			}
		} else if e.Name() != ".DS_Store" {
			empty = false
		}
	}
	if !empty {
		return false
	}
	return removeIfEmpty(abs)
}

func isArchivable(d *Doc) bool {
	if d.Kind == KindTask || d.Kind == KindArtifact {
		return true
	}
	return d.Kind == KindAsset && strings.Contains("/"+d.Path, "/artifacts/assets/")
}

func cmdArchive(e *env, args []string) error {
	fs := newFlags("archive", e)
	status := fs.String("status", "", "タスクに書く status（done か dropped。省略は done、done・dropped ならそのまま）")
	project := fs.String("project", "", "プロジェクトごとルートの archive/ へ")
	dry := fs.Bool("dry-run", false, "何を動かし、どこを書き換えるかだけ出す")
	fs.Usage = func() {
		fmt.Fprintln(e.stderr, "使い方: desk archive <パス>... [--status done|dropped] [--dry-run]\n       desk archive --project <名前> [--dry-run]")
		fs.PrintDefaults()
	}
	if err := fs.Parse(args); err != nil {
		return err
	}
	if *status != "" && *status != "done" && *status != "dropped" {
		return usageErr("--status は done か dropped")
	}
	t, err := loadTree()
	if err != nil {
		return err
	}

	if *project != "" {
		if fs.NArg() != 0 || *status != "" {
			return usageErr("--project にはパスも --status も付けない")
		}
		name := *project
		p := projectDir(name, false)
		if st, err := os.Stat(t.Abs(p)); strings.Contains(name, "/") || err != nil || !st.IsDir() {
			return usageErr("プロジェクト %s が無い", name)
		}
		to := projectDir(name, true)
		if t.Exists(to) {
			return fmt.Errorf("%s は既にある", to)
		}
		var moves []move
		for _, d := range t.Docs {
			if strings.HasPrefix(d.Path, p+"/") {
				moves = append(moves, move{from: d.Path, to: to + "/" + strings.TrimPrefix(d.Path, p+"/")})
			}
		}
		if len(moves) == 0 {
			return fmt.Errorf("%s に動かすファイルが無い", p)
		}
		if _, err := performMoves(e.stdout, t, moves, nil, *dry); err != nil {
			return err
		}
		if !*dry {
			removeEmptyTree(t.Abs(p))
		}
		return nil
	}

	if fs.NArg() == 0 {
		fs.Usage()
		return usageErr("動かすファイルを指定する")
	}
	var moves []move
	st := map[string]string{}
	seen := map[string]bool{}
	for _, a := range fs.Args() {
		p, err := t.Resolve(a)
		if err != nil {
			return err
		}
		d := t.ByPath[p]
		if d == nil {
			return fmt.Errorf("%s はファイルでない（プロジェクトごとなら --project）", p)
		}
		if seen[p] {
			continue
		}
		seen[p] = true
		if d.Archived {
			fmt.Fprintf(e.stdout, "既に archive/ にある: %s\n", p)
			if d.Kind == KindTask && *status != "" && d.Status() != *status {
				st[p] = *status
			}
			continue
		}
		if !isArchivable(d) {
			return fmt.Errorf("%s は archive/ に移すものでない（移すのはタスクと artifacts/。ほかは desk mv）", p)
		}
		moves = append(moves, move{from: p, to: ArchivePath(d.Info)})
		if d.Kind == KindTask {
			switch {
			case *status != "":
				st[p] = *status
			case d.Status() != "done" && d.Status() != "dropped":
				st[p] = "done"
			}
		}
	}
	if len(moves) == 0 && len(st) == 0 {
		return nil
	}
	_, err = performMoves(e.stdout, t, moves, st, *dry)
	return err
}

func cmdMv(e *env, args []string) error {
	fs := newFlags("mv", e)
	dry := fs.Bool("dry-run", false, "何を動かし、どこを書き換えるかだけ出す")
	fs.Usage = func() {
		fmt.Fprintln(e.stderr, "使い方: desk mv <元>... <先のディレクトリ> [--dry-run]\n       desk mv <元のファイル> <新しいパス.md> [--dry-run]   # 名前の変更")
		fs.PrintDefaults()
	}
	if err := fs.Parse(args); err != nil {
		return err
	}
	if fs.NArg() < 2 {
		fs.Usage()
		return usageErr("元と先を指定する")
	}
	t, err := loadTree()
	if err != nil {
		return err
	}
	srcArgs := fs.Args()[:fs.NArg()-1]
	destArg := fs.Arg(fs.NArg() - 1)
	dest, err := t.ResolveDest(destArg)
	if err != nil {
		return err
	}
	var srcs []string
	for _, a := range srcArgs {
		p, err := t.Resolve(a)
		if err != nil {
			return err
		}
		srcs = append(srcs, p)
	}
	destIsDir := strings.HasSuffix(destArg, "/") || dest == "."
	if st, err := os.Stat(t.Abs(dest)); err == nil && st.IsDir() {
		destIsDir = true
	}
	if !destIsDir && path.Ext(dest) == "" {
		destIsDir = true
	}
	var moves []move
	if !destIsDir {
		if len(srcs) != 1 || t.ByPath[srcs[0]] == nil {
			return usageErr("名前を変えるときは元のファイルを 1 つ")
		}
		moves = append(moves, move{from: srcs[0], to: dest})
	} else {
		for _, s := range srcs {
			if _, ok := t.ByPath[s]; ok {
				moves = append(moves, move{from: s, to: path.Join(dest, path.Base(s))})
				continue
			}
			// ディレクトリは中のファイルを全部
			base := path.Base(s)
			n := 0
			for _, d := range t.Docs {
				if strings.HasPrefix(d.Path, s+"/") {
					moves = append(moves, move{from: d.Path, to: path.Join(dest, base, strings.TrimPrefix(d.Path, s+"/"))})
					n++
				}
			}
			if n == 0 {
				return fmt.Errorf("%s に動かすファイルが無い", s)
			}
		}
	}
	for _, m := range moves {
		if m.from == m.to {
			return fmt.Errorf("%s は動かない（先が同じ）", m.from)
		}
	}
	if _, err := performMoves(e.stdout, t, moves, nil, *dry); err != nil {
		return err
	}
	if !*dry {
		for _, s := range srcs {
			if _, ok := t.ByPath[s]; !ok {
				removeEmptyTree(t.Abs(s))
				removeEmptyParents(t, path.Dir(s))
			}
		}
	}
	return nil
}
