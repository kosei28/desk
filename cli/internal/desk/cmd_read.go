package desk

import (
	"bufio"
	"fmt"
	"io"
	"strings"
)

func cmdRead(e *env, args []string) error {
	fs := newFlags("read", e)
	level := fs.Int("level", 1, "0: 概要だけ / 1: 全体 / 2: 本文が指す .md まで")
	outline := fs.Bool("outline", false, "見出しと各節の行数だけ")
	section := fs.String("section", "", "その節だけ（見出しをカンマ区切り）")
	stdin := fs.Bool("stdin", false, "パスを標準入力から読む（desk ls --paths などの出力）")
	fs.Usage = func() {
		fmt.Fprintln(e.stderr, "使い方: desk read <パス>... [--level 0|1|2] [--outline] [--section <見出し>,...]\n       desk read --stdin [...]")
		fs.PrintDefaults()
	}
	if err := fs.Parse(args); err != nil {
		return err
	}
	if *level < 0 || *level > 2 {
		return usageErr("--level は 0・1・2")
	}
	t, err := loadTree()
	if err != nil {
		return err
	}
	names := fs.Args()
	if *stdin {
		sc := bufio.NewScanner(e.stdin)
		for sc.Scan() {
			if l := strings.TrimSpace(sc.Text()); l != "" {
				names = append(names, l)
			}
		}
	}
	if len(names) == 0 {
		fs.Usage()
		return usageErr("読むファイルを指定する")
	}
	var docs []*Doc
	for _, n := range names {
		p, err := t.Resolve(n)
		if err != nil {
			return err
		}
		d := t.ByPath[p]
		if d == nil {
			return fmt.Errorf("%s はファイルでない", p)
		}
		docs = append(docs, d)
	}
	opt := readOpt{level: *level, outline: *outline, sections: splitComma(*section)}
	multi := len(docs) > 1
	for i, d := range docs {
		if i > 0 && (opt.level != 0 || opt.outline) {
			fmt.Fprintln(e.stdout)
		}
		readDoc(e, t, d, opt, multi)
	}
	return nil
}

type readOpt struct {
	level    int
	outline  bool
	sections []string
}

func header(w io.Writer, p string) { fmt.Fprintf(w, "==> %s <==\n", p) }

func readDoc(e *env, t *Tree, d *Doc, opt readOpt, multi bool) {
	w := e.stdout
	switch {
	case opt.outline:
		if multi {
			header(w, d.Path)
		}
		printOutline(w, d)
	case len(opt.sections) > 0:
		if multi {
			header(w, d.Path)
		}
		printSections(e, d, opt.sections)
	case opt.level == 0:
		for _, l := range Summary(d) {
			fmt.Fprintln(w, l)
		}
	case opt.level == 1:
		if multi {
			header(w, d.Path)
		}
		printContent(w, d)
	default:
		header(w, d.Path)
		printContent(w, d)
		linked := linkedDocs(t, d)
		if len(linked) == 0 {
			return
		}
		fmt.Fprintln(w)
		fmt.Fprintln(w, "--- 指す先 ---")
		for _, ld := range linked {
			for _, l := range Summary(ld) {
				fmt.Fprintln(w, l)
			}
		}
		for _, ld := range linked {
			fmt.Fprintln(w)
			header(w, ld.Path)
			printContent(w, ld)
		}
	}
}

func printContent(w io.Writer, d *Doc) {
	if !d.IsMD {
		fmt.Fprintf(w, "（静的ファイル: %s）\n", d.Path)
		return
	}
	c := d.Content()
	fmt.Fprint(w, c)
	if !strings.HasSuffix(c, "\n") {
		fmt.Fprintln(w)
	}
}

// linkedDocs は本文がリンクで指す .md（1 段、指された順、重複なし）。
func linkedDocs(t *Tree, d *Doc) []*Doc {
	var out []*Doc
	seen := map[string]bool{d.Path: true}
	for _, l := range d.Links {
		if l.Key != "body" || l.Form != FormRelative {
			continue
		}
		ld := t.ByPath[l.Target]
		if ld == nil || !ld.IsMD || seen[ld.Path] {
			continue
		}
		seen[ld.Path] = true
		out = append(out, ld)
	}
	return out
}

func printOutline(w io.Writer, d *Doc) {
	if !d.IsMD {
		fmt.Fprintf(w, "（静的ファイル: %s）\n", d.Path)
		return
	}
	if d.HasFM {
		fmt.Fprintf(w, "frontmatter（%d 行）\n", d.FMEnd+1)
	}
	for _, h := range d.Headings {
		_, end, _ := d.sectionAt(h)
		fmt.Fprintf(w, "%s %s（%d 行）\n", strings.Repeat("#", h.Level), h.Text, end-h.Line)
	}
}

// sectionAt は見出し h の節の範囲。
func (d *Doc) sectionAt(h Heading) (int, int, bool) {
	end := len(d.Lines)
	for _, h2 := range d.Headings {
		if h2.Line > h.Line && h2.Level <= h.Level {
			end = h2.Line
			break
		}
	}
	for end > h.Line+1 && strings.TrimSpace(d.Lines[end-1]) == "" {
		end--
	}
	return h.Line, end, true
}

func printSections(e *env, d *Doc, names []string) {
	first := true
	for _, n := range names {
		start, end, ok := d.SectionRange(n)
		if !ok {
			fmt.Fprintf(e.stderr, "（%s に節「%s」は無い）\n", d.Path, n)
			continue
		}
		if !first {
			fmt.Fprintln(e.stdout)
		}
		first = false
		fmt.Fprintln(e.stdout, strings.Join(d.Lines[start:end], "\n"))
	}
}

// Summary は段 0 の概要（1 件 1〜3 行）。
func Summary(d *Doc) []string {
	tags := []string{string(kindOf(d))}
	if d.Project != "" {
		tags = append(tags, d.Project)
	}
	if d.Archived {
		tags = append(tags, "archived")
	}
	if d.Kind == KindTask {
		if s := d.Status(); s != "" {
			tags = append(tags, s)
		}
		if d.NeedsHuman() {
			tags = append(tags, "needs_human")
		}
	}
	lines := []string{d.Path + "  [" + strings.Join(tags, " · ") + "]"}
	if !d.IsMD {
		return lines
	}
	if d.Kind == KindTask {
		lines = append(lines, "  "+d.Title())
		if now := d.Now(); now != "" {
			lines = append(lines, "  いま: "+now)
		}
		return lines
	}
	if desc := d.Description(); desc != "" {
		lines = append(lines, "  "+desc)
	} else if title := d.Title(); title != "" {
		lines = append(lines, "  "+title)
	}
	return lines
}

func kindOf(d *Doc) Kind {
	if d.Kind == "" {
		return "?"
	}
	return d.Kind
}
