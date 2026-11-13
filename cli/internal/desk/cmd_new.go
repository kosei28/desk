package desk

import (
	"fmt"
	"os"
	"path"
	"path/filepath"
	"regexp"
	"strings"
)

var slugRe = regexp.MustCompile(`^[a-z0-9]+(?:-[a-z0-9]+)*$`)

func cmdInit(e *env, args []string) error {
	fs := newFlags("init", e)
	fs.Usage = func() {
		fmt.Fprintln(e.stderr, "使い方: desk init [<パス>]   # 省略時は作業ディレクトリ")
	}
	if err := fs.Parse(args); err != nil {
		return err
	}
	dir := "."
	switch fs.NArg() {
	case 0:
	case 1:
		dir = fs.Arg(0)
	default:
		return usageErr("desk init に渡せるパスは 1 つ")
	}
	root, err := filepath.Abs(dir)
	if err != nil {
		return err
	}
	if st, err := os.Stat(root); err == nil && !st.IsDir() {
		return fmt.Errorf("%s はディレクトリではない", root)
	}
	if r := findMarker(root); r == root {
		fmt.Fprintf(e.stdout, "%s はもうツリーのルート（作るものは無い）\n", root)
		return nil
	} else if r != "" {
		return fmt.Errorf("%s はツリー（%s）の中。ツリーの中にツリーは作らない", root, r)
	}

	var created []string
	if _, err := os.Stat(root); err != nil {
		if err := os.MkdirAll(root, 0o755); err != nil {
			return err
		}
		created = append(created, ".")
	}
	misc := projectDir(MiscProject, false)
	files := []string{MarkerFile, "README.md", misc + "/README.md", ".gitignore"}
	contents := []string{markerContent, rootReadme(root), miscReadme, ".DS_Store\n"}
	var written []string
	for i, f := range files {
		p := filepath.Join(root, f)
		if _, err := os.Stat(p); err == nil {
			continue
		}
		if err := os.MkdirAll(filepath.Dir(p), 0o755); err != nil {
			return err
		}
		if err := os.WriteFile(p, []byte(contents[i]), 0o644); err != nil {
			return err
		}
		written = append(written, f)
	}
	created = append(created, written...)
	if !isGitRepo(root) {
		if err := gitRun(root, "init", "-q", "-b", "main"); err != nil {
			return fmt.Errorf("git init に失敗: %v", err)
		}
		created = append(created, ".git/")
	}
	committed := false
	if gitRun(root, "rev-parse", "-q", "--verify", "HEAD") != nil {
		if err := gitRun(root, append([]string{"add", "--"}, written...)...); err != nil {
			return fmt.Errorf("git add に失敗: %v", err)
		}
		if err := gitRun(root, "commit", "-q", "-m", "docs: desk を作る"); err != nil {
			return fmt.Errorf("最初のコミットに失敗: %v", err)
		}
		committed = true
	}
	fmt.Fprintf(e.stdout, "%s を作った:\n", root)
	for _, c := range created {
		fmt.Fprintf(e.stdout, "  %s\n", c)
	}
	if committed {
		fmt.Fprintln(e.stdout, "最初のコミットを作った")
	} else if len(written) > 0 {
		fmt.Fprintf(e.stdout, "コミットはしていない: git -C %s add %s\n", displayPath(root), strings.Join(written, " "))
	}
	fmt.Fprintln(e.stdout, "desk はこのディレクトリかその下で実行する")
	return nil
}

func displayPath(p string) string {
	if home, err := os.UserHomeDir(); err == nil {
		if rel, err := filepath.Rel(home, p); err == nil && !strings.HasPrefix(rel, "..") {
			return "~/" + filepath.ToSlash(rel)
		}
	}
	return p
}

const markerContent = "# desk のルートの印。desk は作業ディレクトリから上にたどってこれを探し、見つけたディレクトリをルートにする。\n"

func rootReadme(root string) string {
	return "# " + filepath.Base(root) + "\n\n" +
		"あらゆるプロジェクトとタスクの作業ドキュメントを置くツリー（desk）。読み手は本人と AI エージェント。\n\n" +
		"## どこに何があるか\n\n" +
		"```\n" +
		filepath.Base(root) + "/\n" +
		"├── README.md      # これ\n" +
		"├── principles/    # 全プロジェクトに通じる原則\n" +
		"├── knowledge/     # 全プロジェクトで AI が使う知識\n" +
		"├── archive/       # 終わったプロジェクト\n" +
		"└── projects/\n" +
		"    ├── misc/      # どのプロジェクトにも属さないタスク\n" +
		"    └── <project>/ # プロジェクト（tasks/・artifacts/・wiki/・inbox/・library/ など）\n" +
		"```\n\n" +
		"ディレクトリは要るときに作る。動いているタスクや人宛ての一覧は手で書かず、`desk ls` で出す（例: `desk ls task --where needs_human=true --where status~todo,inprogress`）。\n\n" +
		"## 人が書くときの決まり\n\n" +
		"- どこに書いてもよい\n" +
		"- 迷ったら `inbox/` に書く（プロジェクトが分からなければ `projects/misc/inbox/`）\n" +
		"- コミットはしなくてよい\n\n" +
		"## 詳しく\n\n" +
		"決まりの正本はスキル `desk`。スキル・CLI・設計文書は desk のリポジトリにある。\n"
}

const miscReadme = "# misc\n\nどのプロジェクトにも属さないタスクを置く、なんでもありのプロジェクト。\n"

func projectReadme(name string) string {
	return "# " + name + "\n"
}

func cmdNew(e *env, args []string) error {
	fs := newFlags("new", e)
	project := fs.String("project", "", "プロジェクト")
	parent := fs.String("parent", "", "親タスク（task）か、文書のタスク（artifact の tasks）")
	needsHuman := fs.Bool("needs-human", false, "task に needs_human: true を書く（人の判断か作業が要る）")
	title := fs.String("title", "", "見出し（省略は slug）")
	fs.Usage = func() {
		fmt.Fprintln(e.stderr, "使い方: desk new project <名前>\n       desk new <種類> <slug> [--project <名前>] [--parent <タスク>] [--needs-human] [--title <見出し>]")
		fs.PrintDefaults()
	}
	if err := fs.Parse(args); err != nil {
		return err
	}
	if fs.NArg() != 2 {
		fs.Usage()
		return usageErr("種類と名前を指定する")
	}
	t, err := loadTree()
	if err != nil {
		return err
	}
	kind, slug := Kind(fs.Arg(0)), fs.Arg(1)

	if kind == KindProject {
		if !slugRe.MatchString(slug) {
			return usageErr("プロジェクトの名前は英小文字・数字・ハイフン（%s）", slug)
		}
		if t.Exists(projectDir(slug, false)) {
			return &exitError{code: 1, msg: "既にある: " + t.Abs(projectDir(slug, false))}
		}
		if t.Exists(projectDir(slug, true)) {
			return &exitError{code: 1, msg: "archive/ に同じ名前のプロジェクトがある: " + t.Abs(projectDir(slug, true))}
		}
		rel := projectDir(slug, false) + "/README.md"
		if err := writeNew(t, rel, projectReadme(slug)); err != nil {
			return err
		}
		fmt.Fprintln(e.stdout, t.Abs(rel))
		return nil
	}

	rule := RuleOf(kind)
	if rule == nil {
		return usageErr("知らない種類 %q（task artifact inbox library principle knowledge material wiki project）", kind)
	}
	if *needsHuman && kind != KindTask {
		return usageErr("--needs-human は task だけ")
	}
	if *parent != "" && kind != KindTask && kind != KindArtifact {
		return usageErr("--parent は task（parent）と artifact（tasks）だけ")
	}

	// 名前
	date := ""
	if rule.Dated {
		date = today()
		if d, s, ok := SplitDated(slug + ".md"); ok {
			date, slug = d, s
		}
	}
	if err := checkSlug(kind, slug); err != nil {
		return err
	}
	name := slug + ".md"
	if date != "" {
		name = date + "-" + name
	}

	// 親
	var parentDoc *Doc
	if *parent != "" {
		p, err := t.Resolve(*parent)
		if err != nil {
			return err
		}
		parentDoc = t.ByPath[p]
		if parentDoc == nil || parentDoc.Kind != KindTask {
			return usageErr("%s はタスクでない", p)
		}
		if *project == "" {
			*project = parentDoc.Project
			if parentDoc.ProjectArchived {
				return usageErr("%s は archive/ に移ったプロジェクトのタスク", p)
			}
		}
	}

	// 置き場所
	var dir string
	switch {
	case rule.InRoot && *project == "":
		dir = rule.Dir
	default:
		if *project == "" {
			return usageErr("%s はプロジェクトに置く（--project <名前>。どこにも属さないなら misc）", kind)
		}
		pd := projectDir(*project, false)
		if st, err := os.Stat(t.Abs(pd)); strings.Contains(*project, "/") || err != nil || !st.IsDir() {
			return usageErr("プロジェクト %s が無い（desk new project %s で作る）", *project, *project)
		}
		dir = pd + "/" + rule.Dir
	}
	rel := dir + "/" + name

	// 同じ種類に同じ名前（archive/ の中も）
	same := []string{rel}
	if rule.Archivable && *project != "" {
		same = append(same, projectDir(*project, false)+"/"+ArchiveDir+"/"+rule.Dir+"/"+name)
	}
	for _, s := range same {
		if t.Exists(s) {
			return &exitError{code: 1, msg: "既にある: " + t.Abs(s)}
		}
	}

	heading := *title
	if heading == "" {
		heading = path.Base(slug)
	}
	var fm []string
	sections := rule.Sections
	switch kind {
	case KindTask:
		fm = []string{"status: todo", fmt.Sprintf("needs_human: %t", *needsHuman)}
		if parentDoc != nil {
			fm = append(fm, "parent: "+RelPath(path.Dir(rel), parentDoc.Path))
		}
		if *needsHuman {
			sections = HumanTaskSections
		}
	case KindArtifact:
		if parentDoc != nil {
			fm = []string{"tasks: [" + RelPath(path.Dir(rel), parentDoc.Path) + "]"}
		} else {
			fm = []string{"tasks: []"}
		}
	case KindWiki, KindPrinciple:
		fm = []string{"description:"}
	case KindKnowledge:
		fm = []string{"description:", "principle:", "sources: []"}
	case KindMaterial:
		fm = []string{"knowledge: []"}
	}
	if err := writeNew(t, rel, render(fm, heading, sections)); err != nil {
		return err
	}
	fmt.Fprintln(e.stdout, t.Abs(rel))
	return nil
}

func checkSlug(kind Kind, slug string) error {
	segs := []string{slug}
	if kind == KindWiki {
		segs = strings.Split(slug, "/")
	}
	for i, s := range segs {
		if kind == KindWiki && i == len(segs)-1 && s == "README" {
			continue
		}
		if !slugRe.MatchString(s) {
			return usageErr("slug は英小文字・数字・ハイフン（%s）", slug)
		}
	}
	return nil
}

func render(fm []string, title string, sections []string) string {
	var b strings.Builder
	if fm != nil {
		b.WriteString("---\n")
		for _, l := range fm {
			b.WriteString(l + "\n")
		}
		b.WriteString("---\n\n")
	}
	b.WriteString("# " + title + "\n")
	for _, s := range sections {
		b.WriteString("\n## " + s + "\n")
	}
	return b.String()
}

func writeNew(t *Tree, rel, content string) error {
	p := t.Abs(rel)
	if err := os.MkdirAll(filepath.Dir(p), 0o755); err != nil {
		return err
	}
	f, err := os.OpenFile(p, os.O_WRONLY|os.O_CREATE|os.O_EXCL, 0o644)
	if err != nil {
		if os.IsExist(err) {
			return &exitError{code: 1, msg: "既にある: " + p}
		}
		return err
	}
	defer f.Close()
	_, err = f.WriteString(content)
	return err
}
