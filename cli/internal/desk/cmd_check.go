package desk

import (
	"fmt"
	"io"
	"os"
	"path"
	"path/filepath"
	"regexp"
	"sort"
	"strconv"
	"strings"
	"time"
)

// Finding は check で見つかったこと 1 つ。
type Finding struct {
	Path     string `json:"path"`
	Line     int    `json:"line,omitempty"` // 1 始まり。ファイル全体なら 0
	Category string `json:"category"`
	Problem  string `json:"problem"`
	Fix      string `json:"fix"`
	Fixable  bool   `json:"fixable"`

	related []string // --name で一緒に挙げるファイル
	edits   []Edit   // --fix で当てる書き換え
	inserts []string // --fix で frontmatter に足す行
	mv      *move    // --fix で動かす
}

const (
	catDir   = "ディレクトリ"
	catName  = "名前"
	catFM    = "frontmatter"
	catLink  = "リンク"
	catShape = "構成"
	catDrift = "ずれ"
)

type checker struct {
	t       *Tree
	out     []*Finding
	details map[string]bool // ナレッジの詳細（ほかのナレッジの本文から指されているナレッジ）
}

func (c *checker) add(f *Finding) { c.out = append(c.out, f) }

// Check はツリー全体を確かめる。
func Check(t *Tree) []*Finding {
	c := &checker{t: t, details: knowledgeDetails(t)}
	bad := c.checkDirs()
	for _, d := range t.Docs {
		c.checkPlace(d, bad)
		if !d.IsMD {
			continue
		}
		c.checkFM(d)
		c.checkLinks(d)
		c.checkShape(d)
	}
	c.checkNames()
	c.checkAssets()
	c.checkDrift()
	sort.SliceStable(c.out, func(i, j int) bool {
		if c.out[i].Path != c.out[j].Path {
			return c.out[i].Path < c.out[j].Path
		}
		return c.out[i].Line < c.out[j].Line
	})
	return c.out
}

// ディレクトリ

func listDir(abs string) []os.DirEntry {
	entries, _ := os.ReadDir(abs)
	var out []os.DirEntry
	for _, e := range entries {
		if !strings.HasPrefix(e.Name(), ".") {
			out = append(out, e)
		}
	}
	return out
}

// projectEntries はプロジェクトの直下に置いてよい名前。
func projectEntries() []string {
	out := []string{"README.md", "assets", ArchiveDir}
	for _, r := range Rules {
		if r.InProject {
			out = append(out, r.Dir)
		}
	}
	return out
}

// checkDirs はディレクトリの形を確かめ、決まりの外のディレクトリ（中のファイルは個別に挙げない）を返す。
func (c *checker) checkDirs() []string {
	t := c.t
	var bad []string
	badDir := func(rel, problem string) {
		bad = append(bad, rel)
		c.add(&Finding{Path: rel + "/", Category: catDir, Problem: problem, Fix: "中身を決まりの場所に desk mv で動かす"})
	}
	for _, e := range listDir(t.Root) {
		if !e.IsDir() || contains(RootReserved, e.Name()) {
			continue // ファイルは Classify が挙げる
		}
		badDir(e.Name(), "ルートに置けるのは決まった名前だけ（プロジェクトは projects/ の下に置く）")
	}
	checkProject := func(dir string) {
		for _, e := range listDir(t.Abs(dir)) {
			if e.IsDir() && !contains(projectEntries(), e.Name()) {
				badDir(dir+"/"+e.Name(), "プロジェクトに置けないディレクトリ")
			}
		}
		for _, e := range listDir(t.Abs(dir + "/" + ArchiveDir)) {
			if e.IsDir() {
				if r := ruleByDir[e.Name()]; r == nil || !r.Archivable {
					badDir(dir+"/"+ArchiveDir+"/"+e.Name(), "プロジェクトの archive/ に置くのは tasks/ と artifacts/ だけ")
				}
			}
		}
	}
	for _, e := range listDir(t.Abs(ProjectsDir)) {
		if e.IsDir() {
			checkProject(ProjectsDir + "/" + e.Name())
		}
	}
	for _, e := range listDir(t.Abs(ArchiveDir)) {
		if e.IsDir() && e.Name() != ProjectsDir {
			badDir(ArchiveDir+"/"+e.Name(), "ルートの archive/ には projects/<プロジェクト>/ だけを置く")
		}
	}
	for _, e := range listDir(t.Abs(ArchiveDir + "/" + ProjectsDir)) {
		if e.IsDir() {
			checkProject(ArchiveDir + "/" + ProjectsDir + "/" + e.Name())
		}
	}
	return bad
}

// checkPlace はファイルの置き場所（Classify の結果）を挙げる。
func (c *checker) checkPlace(d *Doc, bad []string) {
	if d.Problem == "" {
		return
	}
	for _, b := range bad {
		if strings.HasPrefix(d.Path, b+"/") {
			return
		}
	}
	c.add(&Finding{Path: d.Path, Category: catDir, Problem: d.Problem, Fix: "決まりの場所に desk mv で動かす"})
}

// 名前

func (c *checker) checkNames() {
	groups := map[string][]*Doc{}
	var keys []string
	for _, d := range c.t.Docs {
		if d.Problem != "" {
			continue
		}
		if p := NameProblem(d.Info); p != "" {
			c.add(&Finding{Path: d.Path, Category: catName, Problem: p, Fix: "desk mv で名前を変える（日付は作った日）"})
		}
		var key string
		switch {
		case d.Kind == KindWiki || d.Kind == KindReadme || d.Kind == "":
			continue // wiki/ は入れ子でパスが識別になる
		case d.Asset:
			key = path.Dir(UnarchivePath(d.Info)) + "|" + d.Name()
		default:
			key = uniqueScope(d.Info) + "|" + d.Name()
		}
		if _, ok := groups[key]; !ok {
			keys = append(keys, key)
		}
		groups[key] = append(groups[key], d)
	}
	for _, k := range keys {
		g := groups[k]
		if len(g) < 2 {
			continue
		}
		var paths []string
		for _, d := range g {
			paths = append(paths, d.Path)
		}
		for _, d := range g {
			c.add(&Finding{Path: d.Path, Category: catName, Problem: "同じ種類の中で名前が重複している（" + strings.Join(paths, "、") + "）", Fix: "中身を 1 つに寄せて片方を消すか、desk mv で名前を変える", related: paths})
		}
	}
}

// frontmatter

// knowledgeDetails は、ほかのナレッジの本文から指されているナレッジ（詳細）を返す。
func knowledgeDetails(t *Tree) map[string]bool {
	out := map[string]bool{}
	for _, d := range t.Docs {
		if d.Kind != KindKnowledge {
			continue
		}
		for _, l := range d.Links {
			if l.Key != "body" || l.Form != FormRelative || l.Target == d.Path {
				continue
			}
			if to := t.ByPath[l.Target]; to != nil && to.Kind == KindKnowledge {
				out[l.Target] = true
			}
		}
	}
	return out
}

// keyRules は d に当てる frontmatter のキーの決まり。ナレッジの詳細には principle を求めない。
func (c *checker) keyRules(d *Doc, r *KindRule) []KeyRule {
	if d.Kind != KindKnowledge || !c.details[d.Path] {
		return r.Keys
	}
	var out []KeyRule
	for _, k := range r.Keys {
		if k.Name == "principle" {
			k.Required, k.NonEmpty = false, false
		}
		out = append(out, k)
	}
	return out
}

var dateRe = regexp.MustCompile(`^\d{4}-\d{2}-\d{2}$`)

func (c *checker) checkFM(d *Doc) {
	r := RuleOf(d.Kind)
	if d.FMErr != "" {
		c.add(&Finding{Path: d.Path, Line: 1, Category: catFM, Problem: d.FMErr, Fix: "手で直す"})
		return
	}
	if r == nil || d.Problem != "" {
		return
	}
	keys := c.keyRules(d, r)
	if !d.HasFM {
		if !r.NeedFM {
			return
		}
		var ins []string
		for _, k := range keys {
			if k.Required {
				ins = append(ins, keyLine(k))
			}
		}
		c.add(&Finding{Path: d.Path, Line: 1, Category: catFM, Problem: "frontmatter が無い", Fix: fixText(ins), Fixable: true, inserts: ins})
		for _, k := range keys {
			if k.Required && k.NonEmpty {
				c.add(&Finding{Path: d.Path, Line: 1, Category: catFM, Problem: k.Name + " が空", Fix: "中身を書く"})
			}
		}
		return
	}
	for _, k := range keys {
		n := d.fmValue(k.Name)
		line := 1
		if n != nil {
			line = n.Line
		}
		if n == nil {
			if k.Required {
				ins := []string{keyLine(k)}
				c.add(&Finding{Path: d.Path, Line: 1, Category: catFM, Problem: k.Name + " が無い", Fix: fixText(ins), Fixable: true, inserts: ins})
				if k.NonEmpty {
					c.add(&Finding{Path: d.Path, Line: 1, Category: catFM, Problem: k.Name + " が空", Fix: "中身を書く"})
				}
			}
			continue
		}
		vals := d.List(k.Name)
		if k.NonEmpty && len(vals) == 0 || k.NonEmpty && strings.TrimSpace(strings.Join(vals, "")) == "" {
			c.add(&Finding{Path: d.Path, Line: line, Category: catFM, Problem: k.Name + " が空", Fix: "中身を書く"})
			continue
		}
		for _, v := range vals {
			if len(k.Enum) > 0 && !contains(k.Enum, v) {
				c.add(&Finding{Path: d.Path, Line: line, Category: catFM, Problem: fmt.Sprintf("%s の値 %q は決まりの外（%s）", k.Name, v, strings.Join(k.Enum, "・")), Fix: "手で直す"})
			}
			if k.Date && !isDate(v) {
				c.add(&Finding{Path: d.Path, Line: line, Category: catFM, Problem: fmt.Sprintf("%s の値 %q が日付（YYYY-MM-DD）でない", k.Name, v), Fix: "手で直す"})
			}
		}
	}
}

func isDate(s string) bool {
	if !dateRe.MatchString(s) {
		return false
	}
	_, err := time.Parse("2006-01-02", s)
	return err == nil
}

func keyLine(k KeyRule) string {
	if k.List {
		return k.Name + ": []"
	}
	return k.Name + ":"
}

func fixText(ins []string) string {
	return "--fix で足す（" + strings.Join(ins, "、") + "）"
}

// リンク

func (c *checker) linkExists(target string) bool {
	if strings.HasPrefix(target, "../") || target == ".." {
		_, err := os.Stat(filepath.Join(c.t.Root, filepath.FromSlash(target)))
		return err == nil
	}
	return c.t.Exists(target)
}

func (c *checker) checkLinks(d *Doc) {
	for _, l := range d.Links {
		where := "本文"
		if l.Key != "body" {
			where = l.Key
		}
		switch l.Form {
		case FormAbsolute, FormHome:
			c.add(&Finding{Path: d.Path, Line: l.Line + 1, Category: catLink, Problem: fmt.Sprintf("%s のリンク %s が相対パスでない", where, l.Raw), Fix: "相対パスに書き直す"})
		case FormWiki:
			c.add(&Finding{Path: d.Path, Line: l.Line + 1, Category: catLink, Problem: fmt.Sprintf("%s のリンク %s が名前だけ", where, l.Raw), Fix: "相対パスの markdown のリンクに書き直す"})
		case FormRelative:
			p, _, _ := strings.Cut(l.Raw, "#")
			if p == "" || c.linkExists(l.Target) {
				continue
			}
			f := &Finding{Path: d.Path, Line: l.Line + 1, Category: catLink, Problem: fmt.Sprintf("%s のリンク %s の先が無い", where, l.Raw)}
			cands := c.t.FindByName(path.Base(l.Target))
			if len(cands) == 1 && l.Start >= 0 {
				to := LinkText(RelPath(path.Dir(d.Path), cands[0].Path), l.Anchor)
				f.Fix = "--fix で " + to + " に付け替える（同じ名前のファイルがツリーに 1 つだけある）"
				f.Fixable = true
				f.edits = []Edit{{Line: l.Line, Start: l.Start, Old: l.Raw, New: to}}
			} else if len(cands) > 1 {
				f.Fix = "同じ名前のファイルが複数ある。どれを指すか確かめて手で直す"
			} else {
				f.Fix = "先を探して手で直す（消したならリンクも消す）"
			}
			c.add(f)
		}
	}
}

// checkAssets は assets/ の中でどこからも指されていないファイルを挙げる。
func (c *checker) checkAssets() {
	used := map[string]bool{}
	for _, d := range c.t.Docs {
		for _, l := range d.Links {
			if l.Form == FormRelative {
				used[l.Target] = true
			}
		}
	}
	for _, d := range c.t.Docs {
		if d.Asset && d.Problem == "" && !used[d.Path] {
			c.add(&Finding{Path: d.Path, Category: catLink, Problem: "assets/ の中のファイルがどこからも指されていない", Fix: "要らなければ消す。要るなら指す文書からリンクする"})
		}
	}
}

// 内容の構成

func (c *checker) checkShape(d *Doc) {
	if d.Kind != KindTask || d.Problem != "" {
		return
	}
	want := TaskSections
	if d.Owner() == "human" {
		want = HumanTaskSections
	}
	h2 := d.H2()
	have := map[string]bool{}
	for _, h := range h2 {
		have[h.Text] = true
	}
	for _, s := range want {
		if !have[s] {
			c.add(&Finding{Path: d.Path, Category: catShape, Problem: "必須の節「" + s + "」が無い", Fix: "節を足して書く"})
		}
	}
	for i, h := range h2 {
		if h.Text == LogSection && i != len(h2)-1 {
			c.add(&Finding{Path: d.Path, Line: h.Line + 1, Category: catShape, Problem: "ログの節が最後にない", Fix: "ログの節をいちばん下に移す"})
		}
	}
}

// ツリーの中のずれ

func finished(status string) bool { return status == "done" || status == "dropped" }

func (c *checker) linkTargets(d *Doc, key string) []string {
	var out []string
	for _, l := range d.Links {
		if l.Key == key && l.Form == FormRelative {
			out = append(out, l.Target)
		}
	}
	return out
}

func (c *checker) checkDrift() {
	t := c.t
	children := map[string][]*Doc{}
	var tasks []*Doc
	for _, d := range t.Docs {
		if d.Kind != KindTask || d.Problem != "" || !d.IsMD {
			continue
		}
		tasks = append(tasks, d)
		for _, p := range c.linkTargets(d, "parent") {
			children[p] = append(children[p], d)
		}
	}
	for _, d := range tasks {
		st := d.Status()
		switch {
		case finished(st) && !d.Archived:
			to := ArchivePath(d.Info)
			f := &Finding{Path: d.Path, Category: catDrift, Problem: "status が " + st + " なのに archive/ の外"}
			if t.Exists(to) {
				f.Fix = to + " が既にある。中身を 1 つに寄せる"
			} else {
				f.Fix = "--fix で archive/ に移す（desk archive と同じ）"
				f.Fixable = true
				f.mv = &move{from: d.Path, to: to}
			}
			c.add(f)
		case isOpen(st) && d.Archived:
			if d.ProjectArchived {
				c.add(&Finding{Path: d.Path, Category: catDrift, Problem: "archive に移したプロジェクトの中で status が " + st, Fix: "閉じるか、プロジェクトを projects/ に戻す"})
				break
			}
			to := UnarchivePath(d.Info)
			f := &Finding{Path: d.Path, Category: catDrift, Problem: "status が " + st + " なのに archive/ の中"}
			if t.Exists(to) {
				f.Fix = to + " が既にある。中身を 1 つに寄せる"
			} else {
				f.Fix = "--fix で archive/ の外に戻す"
				f.Fixable = true
				f.mv = &move{from: d.Path, to: to}
			}
			c.add(f)
		}

		// 子と親
		if kids := children[d.Path]; len(kids) > 0 {
			all := true
			var paths []string
			for _, k := range kids {
				paths = append(paths, k.Path)
				if !finished(k.Status()) {
					all = false
				}
			}
			if st == "inprogress" && all {
				c.add(&Finding{Path: d.Path, Category: catDrift, Problem: "子タスクが全部終わっているのに inprogress", Fix: "親を閉じるか、残りを手順に書く", related: paths})
			}
		}
		for _, p := range c.linkTargets(d, "parent") {
			pd := t.ByPath[p]
			if pd == nil || pd == d {
				continue
			}
			ps := pd.Status()
			switch {
			case finished(ps) && isOpen(st):
				c.add(&Finding{Path: d.Path, Category: catDrift, Problem: "親タスク " + p + " が " + ps + " なのに " + st, Fix: "子を閉じるか、親に戻す", related: []string{p}})
			case st == "inprogress" && (ps == "backlog" || ps == "todo"), st == "todo" && ps == "backlog":
				c.add(&Finding{Path: d.Path, Category: catDrift, Problem: "親タスク " + p + " が " + ps + " なのに " + st, Fix: "親の status を進めるか、子を戻す", related: []string{p}})
			}
		}

		// 依存
		for _, p := range c.linkTargets(d, "depends_on") {
			dd := t.ByPath[p]
			if dd == nil || dd == d {
				continue
			}
			ds := dd.Status()
			if (st == "inprogress" || st == "done") && ds != "done" {
				c.add(&Finding{Path: d.Path, Category: catDrift, Problem: "depends_on の " + p + " が " + ds + " なのに " + st, Fix: "依存が done になるまで status を戻すか、依存を外す", related: []string{p}})
			}
			if st == "todo" && ds == "backlog" {
				c.add(&Finding{Path: d.Path, Category: catDrift, Problem: "todo なのに depends_on の " + p + " が backlog", Fix: "依存を todo 以上にするか、このタスクを backlog に戻す", related: []string{p}})
			}
			if st != "dropped" && ds == "dropped" {
				c.add(&Finding{Path: d.Path, Category: catDrift, Problem: "depends_on の " + p + " が dropped", Fix: "依存を外すか差し替える。続けないならこのタスクも dropped にする", related: []string{p}})
			}
		}
	}
	c.checkCycles(tasks, "parent")
	c.checkCycles(tasks, "depends_on")
}

// isOpen はまだ終わっていない status。
func isOpen(status string) bool {
	return status == "backlog" || status == "todo" || status == "inprogress"
}

// checkCycles は key のリンクが自分自身を指すか、循環しているタスクを挙げる。
func (c *checker) checkCycles(tasks []*Doc, key string) {
	next := map[string][]string{}
	for _, d := range tasks {
		for _, p := range c.linkTargets(d, key) {
			if dd := c.t.ByPath[p]; dd != nil && dd.Kind == KindTask {
				next[d.Path] = append(next[d.Path], p)
			}
		}
	}
	for _, d := range tasks {
		for _, p := range next[d.Path] {
			if p == d.Path {
				c.add(&Finding{Path: d.Path, Category: catDrift, Problem: key + " が自分自身を指している", Fix: key + " から自分を外す"})
			}
		}
	}
	for _, scc := range sccs(tasks, next) {
		if len(scc) < 2 {
			continue
		}
		for _, p := range scc {
			var others []string
			for _, q := range scc {
				if q != p {
					others = append(others, q)
				}
			}
			c.add(&Finding{Path: p, Category: catDrift, Problem: key + " が循環している（" + strings.Join(scc, "、") + "）", Fix: "どれかの " + key + " を外す", related: others})
		}
	}
}

// sccs は強連結成分を返す（Tarjan）。各成分の中はパスの順。
func sccs(tasks []*Doc, next map[string][]string) [][]string {
	index := map[string]int{}
	low := map[string]int{}
	on := map[string]bool{}
	var stack []string
	var out [][]string
	n := 0
	var visit func(v string)
	visit = func(v string) {
		index[v], low[v] = n, n
		n++
		stack = append(stack, v)
		on[v] = true
		for _, w := range next[v] {
			if _, ok := index[w]; !ok {
				visit(w)
				low[v] = min(low[v], low[w])
			} else if on[w] {
				low[v] = min(low[v], index[w])
			}
		}
		if low[v] == index[v] {
			var comp []string
			for {
				w := stack[len(stack)-1]
				stack = stack[:len(stack)-1]
				on[w] = false
				comp = append(comp, w)
				if w == v {
					break
				}
			}
			sort.Strings(comp)
			out = append(out, comp)
		}
	}
	for _, d := range tasks {
		if _, ok := index[d.Path]; !ok {
			visit(d.Path)
		}
	}
	return out
}

// コマンド

func cmdCheck(e *env, args []string) error {
	fs := newFlags("check", e)
	project := fs.String("project", "", "プロジェクトで絞る")
	name := fs.String("name", "", "ファイルで絞る（パスか名前。関係するファイルの分も出す）")
	fix := fs.Bool("fix", false, "機械的に 1 通りに決まる直しをする")
	rules := fs.Bool("rules", false, "CLI が実装している形の一覧を出す")
	asJSON := fs.Bool("json", false, "JSON で出す")
	fs.Usage = func() {
		fmt.Fprintln(e.stderr, "使い方: desk check [--project <名前>] [--name <名前>] [--fix] [--rules] [--json]")
		fs.PrintDefaults()
	}
	if err := fs.Parse(args); err != nil {
		return err
	}
	if fs.NArg() != 0 {
		return usageErr("check に引数は付けない（絞るなら --project か --name）")
	}
	if *rules {
		printRules(e.stdout)
		return nil
	}
	t, err := loadTree()
	if err != nil {
		return err
	}
	var target string
	if *name != "" {
		if target, err = t.Resolve(*name); err != nil {
			return err
		}
	}
	keep := func(f *Finding) bool {
		if *project != "" && Classify(strings.TrimSuffix(f.Path, "/")).Project != *project {
			return false
		}
		if target != "" && f.Path != target && !contains(f.related, target) {
			return false
		}
		return true
	}
	filter := func(all []*Finding) []*Finding {
		var out []*Finding
		for _, f := range all {
			if keep(f) {
				out = append(out, f)
			}
		}
		return out
	}
	found := filter(Check(t))
	if *fix {
		changed, err := applyFixes(t, found)
		if err != nil {
			return err
		}
		if len(changed) > 0 {
			if t, err = LoadTree(t.Root); err != nil {
				return err
			}
			found = filter(Check(t))
		}
		if !*asJSON {
			if len(changed) > 0 {
				fmt.Fprintln(e.stdout, "直した:")
				for _, p := range changed {
					fmt.Fprintln(e.stdout, "  "+p)
				}
				fmt.Fprintf(e.stdout, "コミットするとき: git -C %s add %s\n", displayPath(t.Root), strings.Join(changed, " "))
				if len(found) > 0 {
					fmt.Fprintln(e.stdout)
				}
			}
		}
	}
	if *asJSON {
		if found == nil {
			found = []*Finding{}
		}
		if err := writeJSON(e.stdout, found); err != nil {
			return err
		}
	} else {
		printFindings(e.stdout, e.stderr, found, *fix)
	}
	if len(found) > 0 {
		return errFound
	}
	return nil
}

func printFindings(w, ew io.Writer, found []*Finding, fixed bool) {
	if len(found) == 0 {
		fmt.Fprintln(ew, "（形に合っている）")
		return
	}
	var rows [][]string
	fixable := 0
	for _, f := range found {
		p := f.Path
		if f.Line > 0 {
			p += ":" + strconv.Itoa(f.Line)
		}
		if f.Fixable {
			fixable++
		}
		rows = append(rows, []string{p, f.Category, f.Problem, f.Fix})
	}
	table(w, []string{"path", "何を", "何が", "どう直すか"}, rows)
	if !fixed && fixable > 0 {
		fmt.Fprintf(ew, "%d 件中 %d 件は --fix で直せる\n", len(found), fixable)
	}
}

// applyFixes は直せるものを直し、書き換えたファイル（動かしたものは動かした先）を返す。
func applyFixes(t *Tree, found []*Finding) ([]string, error) {
	var changed []string
	seen := map[string]bool{}
	note := func(p string) {
		if !seen[p] {
			seen[p] = true
			changed = append(changed, p)
		}
	}
	edits := map[string]*fileEdit{}
	var order []string
	var moves []move
	for _, f := range found {
		if !f.Fixable {
			continue
		}
		if len(f.edits) > 0 || len(f.inserts) > 0 {
			fe := edits[f.Path]
			if fe == nil {
				fe = &fileEdit{}
				edits[f.Path] = fe
				order = append(order, f.Path)
			}
			fe.edits = append(fe.edits, f.edits...)
			fe.inserts = append(fe.inserts, f.inserts...)
		}
		if f.mv != nil {
			moves = append(moves, *f.mv)
		}
	}
	for _, p := range order {
		if err := writeEdits(t, t.ByPath[p], edits[p]); err != nil {
			return nil, err
		}
		note(p)
	}
	if len(moves) > 0 {
		if len(order) > 0 {
			var err error
			if t, err = LoadTree(t.Root); err != nil {
				return nil, err
			}
		}
		res, err := performMoves(io.Discard, t, moves, nil, false)
		if err != nil {
			return nil, err
		}
		to := map[string]string{}
		for _, m := range res.moved {
			to[m.from] = m.to
		}
		for i, p := range changed {
			if np, ok := to[p]; ok {
				changed[i] = np
				seen[np] = true
			}
		}
		for _, m := range res.moved {
			note(m.to)
		}
		for _, p := range res.rewritten {
			note(p)
		}
	}
	return changed, nil
}

// --rules

func printRules(w io.Writer) {
	fmt.Fprintln(w, "CLI が実装している形（正本はスキル desk。食い違えばスキルが正しい）")
	fmt.Fprintln(w)
	fmt.Fprintln(w, "ルートに置く名前: "+MarkerFile+"、"+strings.Join(RootReserved, "、"))
	fmt.Fprintln(w, "プロジェクト: "+ProjectsDir+"/<名前>/（どこにも属さないものは "+ProjectsDir+"/"+MiscProject+"/）。直下に置く名前: "+strings.Join(projectEntries(), "、"))
	fmt.Fprintln(w, "archive: プロジェクトの "+ArchiveDir+"/ は "+ArchiveDir+"/tasks/・"+ArchiveDir+"/artifacts/（元と同じ形）。プロジェクトごとは "+ArchiveDir+"/"+ProjectsDir+"/<名前>/")
	fmt.Fprintln(w, "リンク: 本文も frontmatter も、そのファイルからの相対パス。絶対パス・~ 始まり・名前だけは書かない")
	fmt.Fprintln(w, "名前: <slug>.md（slug は英小文字・数字・ハイフン）。同じ種類の中（archive/ も含めて）で一意")
	fmt.Fprintln(w)
	for _, r := range Rules {
		var where []string
		if r.InRoot {
			where = append(where, r.Dir+"/")
		}
		if r.InProject {
			where = append(where, ProjectsDir+"/<p>/"+r.Dir+"/")
		}
		fmt.Fprintf(w, "%s — %s\n", r.Kind, r.Desc)
		fmt.Fprintf(w, "  置き場所: %s", strings.Join(where, "、"))
		var notes []string
		if r.Nested {
			notes = append(notes, "中は入れ子にしてよい")
		} else {
			notes = append(notes, "中はフラット")
		}
		if r.Assets {
			notes = append(notes, "静的ファイルは "+r.Dir+"/assets/")
		}
		if r.Archivable {
			notes = append(notes, "終わったら "+ArchiveDir+"/"+r.Dir+"/")
		}
		fmt.Fprintf(w, "（%s）\n", strings.Join(notes, "。"))
		if r.Dated {
			fmt.Fprintln(w, "  名前: YYYY-MM-DD-<slug>.md（作った日）")
		} else {
			fmt.Fprintln(w, "  名前: <slug>.md")
		}
		fm := "任意"
		if r.NeedFM {
			fm = "要る"
		}
		fmt.Fprintf(w, "  frontmatter: %s\n", fm)
		for _, k := range r.Keys {
			var a []string
			if k.Required {
				a = append(a, "必須")
			}
			if k.NonEmpty {
				a = append(a, "空にしない")
			}
			if k.List {
				a = append(a, "配列")
			}
			if k.Link {
				a = append(a, "相対パス")
			}
			if len(k.Enum) > 0 {
				a = append(a, strings.Join(k.Enum, " | "))
			}
			if k.Date {
				a = append(a, "YYYY-MM-DD")
			}
			if k.Note != "" && k.Note != "必須" {
				a = append(a, k.Note)
			}
			fmt.Fprintf(w, "    %s: %s\n", k.Name, strings.Join(a, "。"))
		}
		if r.Kind == KindTask {
			fmt.Fprintf(w, "  必須の節: %s（owner: human は %s）。「%s」の節は最後で追記だけ\n", strings.Join(TaskSections, "・"), strings.Join(HumanTaskSections, "・"), LogSection)
		} else if len(r.Sections) > 0 {
			fmt.Fprintf(w, "  desk new が書く節: %s\n", strings.Join(r.Sections, "・"))
		}
	}
}
