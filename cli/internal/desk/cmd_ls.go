package desk

import (
	"fmt"
	"path"
	"strconv"
	"strings"
	"time"
)

type cond struct{ key, op, val string }

var ops = []string{"!=", "=", "~", "<", ">"}

func parseWhere(s string) (cond, error) {
	i := 0
	for i < len(s) && (s[i] == '_' || s[i] == '-' || s[i] == '.' || s[i] >= 'a' && s[i] <= 'z' || s[i] >= 'A' && s[i] <= 'Z' || s[i] >= '0' && s[i] <= '9') {
		i++
	}
	if i == 0 {
		return cond{}, usageErr("--where %q: キーが無い（例: status=inprogress）", s)
	}
	for _, op := range ops {
		if strings.HasPrefix(s[i:], op) {
			return cond{key: s[:i], op: op, val: s[i+len(op):]}, nil
		}
	}
	return cond{}, usageErr("--where %q: 演算子は = != ~ < >", s)
}

// fieldValues はキーの値（frontmatter か、name・path・project・kind・title・now・modified）。
func fieldValues(t *Tree, d *Doc, key string) []string {
	one := func(s string) []string {
		if s == "" {
			return nil
		}
		return []string{s}
	}
	switch key {
	case "name":
		if d.Kind == KindProject {
			return one(d.Project)
		}
		return one(d.Name())
	case "path":
		return one(d.Path)
	case "project":
		return one(d.Project)
	case "kind":
		return one(string(d.Kind))
	case "title":
		return one(d.Title())
	case "now":
		return one(d.Now())
	case "modified":
		return one(modifiedOf(t, d))
	}
	if !d.HasKey(key) {
		return nil
	}
	return d.List(key)
}

func modifiedOf(t *Tree, d *Doc) string {
	p := d.Path
	if d.Kind == KindProject {
		p = d.Path + "/README.md"
	}
	return t.Modified(p)
}

// valueMatches は値 v（パスかもしれない）が val に当たるか。パスなら名前だけでも当たる。
func valueMatches(d *Doc, key, v, val string) bool {
	if v == val {
		return true
	}
	p, _, _ := strings.Cut(v, "#")
	b := path.Base(p)
	if p == val || b == val || strings.TrimSuffix(b, ".md") == val {
		return true
	}
	if contains(LinkKeys(d.Kind), key) && !strings.HasPrefix(p, "/") && !strings.HasPrefix(p, "~") {
		r := path.Clean(path.Join(path.Dir(d.Path), p))
		if r == val || r == strings.TrimSuffix(val, "/") {
			return true
		}
	}
	return false
}

func (c cond) match(t *Tree, d *Doc) bool {
	vals := fieldValues(t, d, c.key)
	switch c.op {
	case "=", "!=":
		var ok bool
		if c.val == "[]" || c.val == "" {
			ok = len(vals) == 0
		} else {
			for _, v := range vals {
				if valueMatches(d, c.key, v, c.val) {
					ok = true
					break
				}
			}
		}
		if c.op == "!=" {
			return !ok
		}
		return ok
	case "~":
		for _, v := range vals {
			for _, alt := range splitComma(c.val) {
				if strings.Contains(v, alt) {
					return true
				}
			}
		}
		return false
	case "<", ">":
		for _, v := range vals {
			if compare(v, c.val, c.op) {
				return true
			}
		}
	}
	return false
}

func compare(a, b, op string) bool {
	fa, ea := strconv.ParseFloat(a, 64)
	fb, eb := strconv.ParseFloat(b, 64)
	if ea == nil && eb == nil {
		if op == "<" {
			return fa < fb
		}
		return fa > fb
	}
	if op == "<" {
		return a < b
	}
	return a > b
}

// defaultFields は種類ごとの既定の列。
func defaultFields(kinds []Kind) []string {
	if len(kinds) != 1 {
		return []string{"path", "kind", "title"}
	}
	switch kinds[0] {
	case KindTask:
		return []string{"path", "status", "needs_human", "now"}
	case KindArtifact:
		return []string{"path", "tasks", "title"}
	case KindWiki, KindPrinciple, KindKnowledge:
		return []string{"path", "description"}
	case KindMaterial:
		return []string{"path", "knowledge", "title"}
	case KindLibrary:
		return []string{"path", "tags", "title"}
	case KindProject:
		return []string{"name", "title"}
	}
	return []string{"path", "title"}
}

func fieldString(t *Tree, d *Doc, key string) string {
	if key == "description" {
		return d.Description()
	}
	return oneLine(strings.Join(fieldValues(t, d, key), ", "))
}

// lsItems は種類とプロジェクトで絞った項目（プロジェクトは README を見出しに使う）。
func lsItems(t *Tree, kinds []Kind, project string, archived bool) []*Doc {
	want := map[Kind]bool{}
	for _, k := range kinds {
		want[k] = true
	}
	all := len(kinds) == 0
	var out []*Doc
	for _, d := range t.Docs {
		if !d.IsMD || d.Problem != "" || d.Kind == "" || d.Kind == KindReadme {
			continue
		}
		if !all && !want[d.Kind] {
			continue
		}
		if d.Archived && !archived {
			continue
		}
		if project != "" && d.Project != project {
			// ルートの原則とナレッジは全プロジェクトに当てはまるので入れる
			if !(d.Project == "" && (d.Kind == KindPrinciple || d.Kind == KindKnowledge)) {
				continue
			}
		}
		out = append(out, d)
	}
	if all || want[KindProject] {
		for _, p := range t.Projects(archived) {
			if project != "" && p.Project != project {
				continue
			}
			d := &Doc{Info: p}
			if r := t.ByPath[p.Path+"/README.md"]; r != nil {
				cp := *r
				cp.Info = p
				d = &cp
			}
			out = append(out, d)
		}
	}
	return out
}

func cmdLs(e *env, args []string) error {
	fs := newFlags("ls", e)
	name := fs.String("name", "", "ファイル名で引く")
	project := fs.String("project", "", "プロジェクトで絞る（原則とナレッジはルートのものも入る）")
	archived := fs.Bool("archived", false, "archive/ の中も入れる")
	wheres := fs.StringArray("where", nil, "<キー><演算子><値>（= != ~ < >。複数は AND）")
	before := fs.Int("modified-before", -1, "最終コミットが N 日より前")
	after := fs.Int("modified-after", -1, "最終コミットが N 日以内")
	fields := fs.String("fields", "", "列（name path project modified kind title now と frontmatter のキー）")
	asJSON := fs.Bool("json", false, "JSON で出す")
	paths := fs.Bool("paths", false, "パスだけ出す")
	level := fs.Int("level", -1, "当たったものを read のその段（0|1|2）で出す")
	fs.Usage = func() {
		fmt.Fprintln(e.stderr, "使い方: desk ls [<種類>] [--name <名前>] [--project <名前>] [--archived] [--where <キー><演算子><値>]...\n               [--modified-before <日数>] [--modified-after <日数>] [--fields <列>,...] [--json | --paths | --level 0|1|2]")
		fs.PrintDefaults()
	}
	if err := fs.Parse(args); err != nil {
		return err
	}
	if fs.NArg() > 1 {
		return usageErr("種類は 1 つ（カンマ区切りで複数）")
	}
	var kinds []Kind
	if fs.NArg() == 1 {
		for _, k := range splitComma(fs.Arg(0)) {
			if !contains(kindNames(), k) {
				return usageErr("知らない種類 %q（%s）", k, strings.Join(kindNames(), " "))
			}
			kinds = append(kinds, Kind(k))
		}
	}
	var conds []cond
	for _, w := range *wheres {
		c, err := parseWhere(w)
		if err != nil {
			return err
		}
		conds = append(conds, c)
	}
	if *level > 2 {
		return usageErr("--level は 0・1・2")
	}
	t, err := loadTree()
	if err != nil {
		return err
	}
	now := time.Now()
	day := func(n int) string { return now.AddDate(0, 0, -n).Format("2006-01-02") }
	var hits []*Doc
	for _, d := range lsItems(t, kinds, *project, *archived) {
		if *name != "" && !(d.Name() == *name || d.Name() == *name+".md" || d.Kind == KindProject && d.Project == *name) {
			continue
		}
		ok := true
		for _, c := range conds {
			if !c.match(t, d) {
				ok = false
				break
			}
		}
		if ok && (*before >= 0 || *after >= 0) {
			m := modifiedOf(t, d)
			if m == "" {
				m = today()
			}
			if *before >= 0 && !(m < day(*before)) {
				ok = false
			}
			if *after >= 0 && !(m >= day(*after)) {
				ok = false
			}
		}
		if ok {
			hits = append(hits, d)
		}
	}

	switch {
	case *paths:
		for _, d := range hits {
			fmt.Fprintln(e.stdout, d.Path)
		}
	case *level >= 0:
		for i, d := range hits {
			if i > 0 && *level > 0 {
				fmt.Fprintln(e.stdout)
			}
			if d.Kind == KindProject {
				if r := t.ByPath[d.Path+"/README.md"]; r != nil {
					readDoc(e, t, r, readOpt{level: *level}, true)
				} else {
					fmt.Fprintln(e.stdout, d.Path+"  [project]")
				}
				continue
			}
			readDoc(e, t, d, readOpt{level: *level}, true)
		}
	case *asJSON:
		cols := splitComma(*fields)
		var out []map[string]any
		for _, d := range hits {
			m := map[string]any{
				"path": d.Path, "kind": d.Kind, "project": d.Project, "archived": d.Archived,
				"name": d.Name(), "title": d.Title(), "frontmatter": d.FMMap(),
			}
			if d.Kind == KindProject {
				m["name"] = d.Project
			}
			if d.Kind == KindTask {
				m["now"] = d.Now()
			}
			for _, c := range cols {
				if _, ok := m[c]; !ok {
					m[c] = fieldString(t, d, c)
				}
			}
			out = append(out, m)
		}
		if out == nil {
			out = []map[string]any{}
		}
		return writeJSON(e.stdout, out)
	default:
		cols := splitComma(*fields)
		if len(cols) == 0 {
			cols = defaultFields(kinds)
		}
		var rows [][]string
		for _, d := range hits {
			row := make([]string, len(cols))
			for i, c := range cols {
				row[i] = fieldString(t, d, c)
			}
			rows = append(rows, row)
		}
		if len(rows) == 0 {
			fmt.Fprintln(e.stderr, "（当たるものは無い）")
			return nil
		}
		table(e.stdout, cols, rows)
	}
	return nil
}

func kindNames() []string {
	var out []string
	for _, k := range AllKinds {
		out = append(out, string(k))
	}
	return out
}
