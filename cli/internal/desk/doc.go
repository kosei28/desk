package desk

import (
	"fmt"
	"net/url"
	"path"
	"regexp"
	"strings"
	"unicode/utf8"

	"gopkg.in/yaml.v3"
)

// Doc はツリーの中のファイル 1 つ。.md なら中身を読んだもの。
type Doc struct {
	Info
	IsMD      bool
	Lines     []string // 行（改行を除く）
	HasFM     bool
	FMErr     string
	FMEnd     int // frontmatter を閉じる --- の行（HasFM のとき）
	fm        *yaml.Node
	BodyStart int
	Headings  []Heading
	Links     []Link
}

type Heading struct {
	Level int
	Text  string
	Line  int
}

type LinkForm int

const (
	FormRelative LinkForm = iota
	FormAbsolute          // / 始まり、file:
	FormHome              // ~ 始まり
	FormWiki              // [[名前]]
	FormExternal          // URL
	FormAnchor            // #だけ
)

// Link はファイルの中のリンク 1 つ。
type Link struct {
	Key    string // "body" か frontmatter のキー
	Text   string // 本文のリンクの文字
	Raw    string // 書いてあるとおりの先（アンカー込み）
	Line   int    // ファイルの行（0 始まり）
	Start  int    // 行の中のバイト位置（-1 なら書き換えられない）
	Form   LinkForm
	Target string // FormRelative: ルートからのパス（外なら ../ 始まり）。FormAbsolute・FormHome: 絶対パス。FormWiki: 名前
	Anchor string
}

var (
	headingRe = regexp.MustCompile(`^(#{1,6})\s+(.*?)\s*#*\s*$`)
	mdLinkRe  = regexp.MustCompile(`(!?)\[((?:[^\[\]]|\[[^\]]*\])*)\]\(\s*(<[^>]*>|[^)\s]+)(?:\s+(?:"[^"]*"|'[^']*'|\([^)]*\)))?\s*\)`)
	refDefRe  = regexp.MustCompile(`^\s{0,3}\[[^\]]+\]:\s*(<[^>]*>|\S+)`)
	wikiRe    = regexp.MustCompile(`\[\[([^\]]+)\]\]`)
	schemeRe  = regexp.MustCompile(`^[a-zA-Z][a-zA-Z0-9+.-]*:`)
	inlineRe  = regexp.MustCompile("`+[^`]*`+")
	listMark  = regexp.MustCompile(`^\s*(?:[-*+]\s+(?:\[[ xX]\]\s+)?|\d+\.\s+)`)
)

// ParseDoc は .md の中身を読む。
func ParseDoc(info Info, content string) *Doc {
	d := &Doc{Info: info, IsMD: true}
	content = strings.ReplaceAll(content, "\r\n", "\n")
	d.Lines = strings.Split(content, "\n")
	d.parseFrontmatter()
	d.parseBody()
	return d
}

func (d *Doc) parseFrontmatter() {
	if len(d.Lines) == 0 || strings.TrimSpace(d.Lines[0]) != "---" {
		return
	}
	end := -1
	for i := 1; i < len(d.Lines); i++ {
		t := strings.TrimRight(d.Lines[i], " \t")
		if t == "---" || t == "..." {
			end = i
			break
		}
	}
	if end < 0 {
		d.FMErr = "frontmatter が閉じていない"
		return
	}
	d.HasFM = true
	d.FMEnd = end
	d.BodyStart = end + 1
	text := strings.Join(d.Lines[1:end], "\n")
	var node yaml.Node
	if err := yaml.Unmarshal([]byte(text), &node); err != nil {
		d.FMErr = "frontmatter が YAML として読めない: " + err.Error()
		return
	}
	if node.Kind == 0 || len(node.Content) == 0 {
		d.fm = &yaml.Node{Kind: yaml.MappingNode}
		return
	}
	m := node.Content[0]
	if m.Kind == yaml.ScalarNode && m.Tag == "!!null" {
		d.fm = &yaml.Node{Kind: yaml.MappingNode}
		return
	}
	if m.Kind != yaml.MappingNode {
		d.FMErr = "frontmatter が「キー: 値」の形でない"
		return
	}
	d.fm = m
	for i := 0; i+1 < len(m.Content); i += 2 {
		key := m.Content[i].Value
		if !contains(LinkKeys(d.Kind), key) {
			continue
		}
		for _, v := range scalarNodes(m.Content[i+1]) {
			if strings.TrimSpace(v.Value) == "" {
				continue
			}
			line := v.Line // frontmatter の 1 行目はファイルの 1 行目（0 始まり）
			l := Link{Key: key, Raw: v.Value, Line: line, Start: d.locate(line, v)}
			d.Links = append(d.Links, l)
		}
	}
}

// locate は frontmatter の値が行の何バイト目から始まるかを探す。
func (d *Doc) locate(line int, v *yaml.Node) int {
	if line < 0 || line >= len(d.Lines) {
		return -1
	}
	s := d.Lines[line]
	// yaml の Column は 1 始まりの文字数
	off := 0
	for i := 1; i < v.Column && off < len(s); i++ {
		_, size := utf8.DecodeRuneInString(s[off:])
		off += size
	}
	if v.Style&(yaml.DoubleQuotedStyle|yaml.SingleQuotedStyle) != 0 {
		off++
	}
	if off <= len(s) && strings.HasPrefix(s[off:], v.Value) {
		return off
	}
	return strings.Index(s, v.Value)
}

func scalarNodes(n *yaml.Node) []*yaml.Node {
	switch n.Kind {
	case yaml.ScalarNode:
		if n.Tag == "!!null" {
			return nil
		}
		return []*yaml.Node{n}
	case yaml.SequenceNode:
		var out []*yaml.Node
		for _, c := range n.Content {
			if c.Kind == yaml.ScalarNode && c.Tag != "!!null" {
				out = append(out, c)
			}
		}
		return out
	}
	return nil
}

func (d *Doc) parseBody() {
	fence := ""
	for i := d.BodyStart; i < len(d.Lines); i++ {
		line := d.Lines[i]
		trim := strings.TrimLeft(line, " ")
		if fence != "" {
			if strings.HasPrefix(trim, fence) {
				fence = ""
			}
			continue
		}
		if strings.HasPrefix(trim, "```") || strings.HasPrefix(trim, "~~~") {
			fence = trim[:3]
			continue
		}
		if m := headingRe.FindStringSubmatch(line); m != nil {
			d.Headings = append(d.Headings, Heading{Level: len(m[1]), Text: m[2], Line: i})
		}
		masked := inlineRe.ReplaceAllStringFunc(line, func(s string) string { return strings.Repeat(" ", len(s)) })
		for _, m := range mdLinkRe.FindAllStringSubmatchIndex(masked, -1) {
			start, end := m[6], m[7]
			raw := line[start:end]
			if strings.HasPrefix(raw, "<") && strings.HasSuffix(raw, ">") {
				raw = raw[1 : len(raw)-1]
				start++
			}
			d.Links = append(d.Links, Link{Key: "body", Text: line[m[4]:m[5]], Raw: raw, Line: i, Start: start})
		}
		if m := refDefRe.FindStringSubmatchIndex(masked); m != nil {
			start, end := m[2], m[3]
			raw := line[start:end]
			if strings.HasPrefix(raw, "<") && strings.HasSuffix(raw, ">") {
				raw = raw[1 : len(raw)-1]
				start++
			}
			d.Links = append(d.Links, Link{Key: "body", Raw: raw, Line: i, Start: start})
		}
		for _, m := range wikiRe.FindAllStringSubmatchIndex(masked, -1) {
			d.Links = append(d.Links, Link{Key: "body", Raw: line[m[0]:m[1]], Line: i, Start: m[0], Form: FormWiki, Target: line[m[2]:m[3]]})
		}
	}
	for i := range d.Links {
		if d.Links[i].Form == FormWiki {
			continue
		}
		resolveLink(d.Path, &d.Links[i])
	}
}

// resolveLink はリンクの形を決め、相対パスならルートからのパスにする。
func resolveLink(from string, l *Link) {
	raw := l.Raw
	p, anchor, _ := strings.Cut(raw, "#")
	l.Anchor = anchor
	switch {
	case strings.HasPrefix(raw, "#"):
		l.Form = FormAnchor
	case strings.HasPrefix(raw, "file:"):
		l.Form = FormAbsolute
		if u, err := url.Parse(p); err == nil {
			l.Target = u.Path
		}
	case schemeRe.MatchString(raw):
		l.Form = FormExternal
	case strings.HasPrefix(raw, "/"):
		l.Form = FormAbsolute
		l.Target = unescape(p)
	case strings.HasPrefix(raw, "~"):
		l.Form = FormHome
		l.Target = unescape(p)
	default:
		l.Form = FormRelative
		l.Target = path.Clean(path.Join(path.Dir(from), unescape(p)))
	}
}

func unescape(p string) string {
	if u, err := url.PathUnescape(p); err == nil {
		return u
	}
	return p
}

// FM の読み出し

func (d *Doc) fmValue(key string) *yaml.Node {
	if d.fm == nil {
		return nil
	}
	for i := 0; i+1 < len(d.fm.Content); i += 2 {
		if d.fm.Content[i].Value == key {
			return d.fm.Content[i+1]
		}
	}
	return nil
}

// HasKey は frontmatter にキーがあるか。
func (d *Doc) HasKey(key string) bool { return d.fmValue(key) != nil }

// Str は frontmatter の値を文字列で返す（配列ならカンマでつなぐ）。
func (d *Doc) Str(key string) string {
	n := d.fmValue(key)
	if n == nil {
		return ""
	}
	if n.Kind == yaml.ScalarNode {
		if n.Tag == "!!null" {
			return ""
		}
		return n.Value
	}
	return strings.Join(d.List(key), ", ")
}

// List は frontmatter の値を配列で返す（1 つの値なら 1 件の配列）。
func (d *Doc) List(key string) []string {
	n := d.fmValue(key)
	if n == nil {
		return nil
	}
	var out []string
	for _, v := range scalarNodes(n) {
		out = append(out, v.Value)
	}
	return out
}

// FMKeys は frontmatter のキーを順に返す。
func (d *Doc) FMKeys() []string {
	if d.fm == nil {
		return nil
	}
	var keys []string
	for i := 0; i+1 < len(d.fm.Content); i += 2 {
		keys = append(keys, d.fm.Content[i].Value)
	}
	return keys
}

// FMMap は frontmatter を JSON に出せる形で返す。
func (d *Doc) FMMap() map[string]any {
	out := map[string]any{}
	if d.fm == nil {
		return out
	}
	for i := 0; i+1 < len(d.fm.Content); i += 2 {
		var v any
		n := d.fm.Content[i+1]
		if n.Kind == yaml.ScalarNode {
			if n.Tag == "!!null" {
				v = nil
			} else {
				v = n.Value
			}
		} else if err := n.Decode(&v); err != nil {
			v = nil
		}
		out[d.fm.Content[i].Value] = v
	}
	return out
}

// 本文の読み出し

// Title は最初の見出し。
func (d *Doc) Title() string {
	for _, h := range d.Headings {
		if h.Level == 1 {
			return h.Text
		}
	}
	if len(d.Headings) > 0 {
		return d.Headings[0].Text
	}
	return ""
}

// Description は description を 1 行にしたもの。
func (d *Doc) Description() string { return oneLine(d.Str("description")) }

// Now は「いま」の 1 行目。
func (d *Doc) Now() string {
	start, end, ok := d.SectionRange(NowSection)
	if !ok {
		return ""
	}
	for i := start + 1; i < end; i++ {
		t := strings.TrimSpace(d.Lines[i])
		if t == "" {
			continue
		}
		return strings.TrimSpace(listMark.ReplaceAllString(t, ""))
	}
	return ""
}

// Status は status（無ければ空）。
func (d *Doc) Status() string { return d.Str("status") }

// NeedsHuman は needs_human が true か。
func (d *Doc) NeedsHuman() bool { return d.Str("needs_human") == "true" }

// SectionRange は見出しの行と、節の終わり（次の同じか浅い見出しの行。無ければファイルの終わり）を返す。
func (d *Doc) SectionRange(name string) (int, int, bool) {
	for i, h := range d.Headings {
		if h.Text != name {
			continue
		}
		end := len(d.Lines)
		for _, h2 := range d.Headings[i+1:] {
			if h2.Level <= h.Level {
				end = h2.Line
				break
			}
		}
		// 末尾の空行は節に含めない
		for end > h.Line+1 && strings.TrimSpace(d.Lines[end-1]) == "" {
			end--
		}
		return h.Line, end, true
	}
	return 0, 0, false
}

// H2 は ## の見出しを順に返す。
func (d *Doc) H2() []Heading {
	var out []Heading
	for _, h := range d.Headings {
		if h.Level == 2 {
			out = append(out, h)
		}
	}
	return out
}

// Content はファイルの中身。
func (d *Doc) Content() string { return strings.Join(d.Lines, "\n") }

func oneLine(s string) string { return strings.Join(strings.Fields(s), " ") }

// Edit は行の中の書き換え 1 つ。
type Edit struct {
	Line  int
	Start int
	Old   string
	New   string
}

// ApplyEdits は行に書き換えを当てる。同じ行の書き換えは後ろから。
func ApplyEdits(lines []string, edits []Edit) ([]string, error) {
	out := append([]string(nil), lines...)
	// 後ろから当てる
	sorted := append([]Edit(nil), edits...)
	for i := 1; i < len(sorted); i++ {
		for j := i; j > 0 && (sorted[j].Line > sorted[j-1].Line || sorted[j].Line == sorted[j-1].Line && sorted[j].Start > sorted[j-1].Start); j-- {
			sorted[j], sorted[j-1] = sorted[j-1], sorted[j]
		}
	}
	for _, e := range sorted {
		if e.Line < 0 || e.Line >= len(out) || e.Start < 0 || e.Start+len(e.Old) > len(out[e.Line]) || out[e.Line][e.Start:e.Start+len(e.Old)] != e.Old {
			return nil, fmt.Errorf("%d 行目の %q を書き換えられない", e.Line+1, e.Old)
		}
		s := out[e.Line]
		out[e.Line] = s[:e.Start] + e.New + s[e.Start+len(e.Old):]
	}
	return out, nil
}
