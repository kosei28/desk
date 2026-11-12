package desk

import (
	"fmt"
	"io"
	"sort"
	"strconv"
	"strings"
)

// Edge はリンク 1 本（書いてあるファイル → 指された先）。Key は frontmatter のキーか body。
type Edge struct {
	From, To, Key string
}

// Edges はツリーの中のリンクのうち、先がツリーの中のファイルにあるもの。
func (t *Tree) Edges() []Edge {
	var out []Edge
	seen := map[Edge]bool{}
	for _, d := range t.Docs {
		for _, l := range d.Links {
			if l.Form != FormRelative || l.Target == d.Path {
				continue
			}
			if _, ok := t.ByPath[l.Target]; !ok {
				continue
			}
			e := Edge{From: d.Path, To: l.Target, Key: l.Key}
			if !seen[e] {
				seen[e] = true
				out = append(out, e)
			}
		}
	}
	return out
}

func cmdGraph(e *env, args []string) error {
	fs := newFlags("graph", e)
	in := fs.Bool("in", false, "指されているもの")
	out := fs.Bool("out", false, "指しているもの")
	via := fs.String("via", "", "辺の種類（frontmatter のキーか body。カンマ区切り）")
	depth := fs.String("depth", "1", "たどる段数（n か all）")
	format := fs.String("format", "", "tree | paths | dot（--all は edges | dot）")
	level := fs.Int("level", -1, "たどった先を read のその段（0|1|2）で出す")
	all := fs.Bool("all", false, "ツリー全体のリンク")
	fs.Usage = func() {
		fmt.Fprintln(e.stderr, "使い方: desk graph <パス> [--in | --out] [--via <キー>,...] [--depth <n>|all] [--format tree|paths|dot] [--level 0|1|2]\n       desk graph --all [--via <キー>,...] [--format edges|dot]")
		fs.PrintDefaults()
	}
	if err := fs.Parse(args); err != nil {
		return err
	}
	if *in && *out {
		return usageErr("--in と --out はどちらか（両方なら付けない）")
	}
	if *level > 2 {
		return usageErr("--level は 0・1・2")
	}
	keys := splitComma(*via)
	keep := func(ed Edge) bool { return len(keys) == 0 || contains(keys, ed.Key) }

	t, err := loadTree()
	if err != nil {
		return err
	}
	var edges []Edge
	for _, ed := range t.Edges() {
		if keep(ed) {
			edges = append(edges, ed)
		}
	}

	if *all {
		if fs.NArg() != 0 {
			return usageErr("--all にパスは付けない")
		}
		switch *format {
		case "", "edges":
			var rows [][]string
			for _, ed := range edges {
				rows = append(rows, []string{ed.From, ed.Key, ed.To})
			}
			table(e.stdout, []string{"from", "via", "to"}, rows)
		case "dot":
			writeDot(e.stdout, edges)
		default:
			return usageErr("--all の --format は edges か dot")
		}
		return nil
	}

	if fs.NArg() != 1 {
		fs.Usage()
		return usageErr("パスを 1 つ指定する")
	}
	start, err := t.Resolve(fs.Arg(0))
	if err != nil {
		return err
	}
	if _, ok := t.ByPath[start]; !ok {
		return fmt.Errorf("%s はファイルでない", start)
	}
	maxDepth := 1
	if *depth == "all" {
		maxDepth = -1
	} else if n, err := strconv.Atoi(*depth); err == nil && n >= 1 {
		maxDepth = n
	} else {
		return usageErr("--depth は 1 以上の数か all")
	}

	outs := map[string][]Edge{}
	ins := map[string][]Edge{}
	for _, ed := range edges {
		outs[ed.From] = append(outs[ed.From], ed)
		ins[ed.To] = append(ins[ed.To], ed)
	}
	for _, m := range []map[string][]Edge{outs, ins} {
		for k := range m {
			sort.Slice(m[k], func(i, j int) bool {
				a, b := m[k][i], m[k][j]
				if a.Key != b.Key {
					return a.Key < b.Key
				}
				return a.From+a.To < b.From+b.To
			})
		}
	}

	g := &walker{t: t, outs: outs, ins: ins, max: maxDepth, seen: map[string]bool{start: true}}
	dirs := []bool{true, false} // true: out
	if *in {
		dirs = []bool{false}
	} else if *out {
		dirs = []bool{true}
	}
	for _, d := range dirs {
		g.walk(start, d, 1)
	}

	switch *format {
	case "", "tree":
		fmt.Fprintln(e.stdout, nodeLine(t, start))
		for _, l := range g.lines {
			fmt.Fprintln(e.stdout, l)
		}
	case "paths":
		for _, p := range g.order {
			fmt.Fprintln(e.stdout, p)
		}
	case "dot":
		writeDot(e.stdout, g.edges)
	default:
		return usageErr("--format は tree・paths・dot")
	}
	if *level >= 0 && len(g.order) > 0 {
		fmt.Fprintln(e.stdout)
		for i, p := range g.order {
			if i > 0 && *level > 0 {
				fmt.Fprintln(e.stdout)
			}
			readDoc(e, t, t.ByPath[p], readOpt{level: *level}, true)
		}
	}
	return nil
}

type walker struct {
	t     *Tree
	outs  map[string][]Edge
	ins   map[string][]Edge
	max   int
	seen  map[string]bool
	lines []string
	order []string
	edges []Edge
}

// walk は 1 つの向きにたどる（枝の途中で向きは変えない）。
func (g *walker) walk(node string, out bool, depth int) {
	list := g.ins[node]
	arrow := "←"
	if out {
		list = g.outs[node]
		arrow = "→"
	}
	for _, ed := range list {
		next := ed.To
		if !out {
			next = ed.From
		}
		g.edges = append(g.edges, ed)
		indent := strings.Repeat("  ", depth)
		if g.seen[next] {
			g.lines = append(g.lines, fmt.Sprintf("%s%s %s  %s（既出）", indent, arrow, ed.Key, next))
			continue
		}
		g.seen[next] = true
		g.order = append(g.order, next)
		g.lines = append(g.lines, fmt.Sprintf("%s%s %s  %s", indent, arrow, ed.Key, nodeLine(g.t, next)))
		if g.max < 0 || depth < g.max {
			g.walk(next, out, depth+1)
		}
	}
}

// nodeLine はパス、種類、description（無ければ見出し）。
func nodeLine(t *Tree, p string) string {
	d := t.ByPath[p]
	if d == nil {
		return p
	}
	s := p + "  [" + string(kindOf(d))
	if d.Kind == KindTask && d.Status() != "" {
		s += " · " + d.Status()
	}
	s += "]"
	if desc := d.Description(); desc != "" {
		s += "  " + desc
	} else if title := d.Title(); title != "" {
		s += "  " + title
	}
	return s
}

func writeDot(w io.Writer, edges []Edge) {
	fmt.Fprintln(w, "digraph desk {")
	fmt.Fprintln(w, "  rankdir=LR;")
	fmt.Fprintln(w, "  node [shape=box, fontsize=10];")
	seen := map[Edge]bool{}
	for _, ed := range edges {
		if seen[ed] {
			continue
		}
		seen[ed] = true
		fmt.Fprintf(w, "  %q -> %q [label=%q];\n", ed.From, ed.To, ed.Key)
	}
	fmt.Fprintln(w, "}")
}
