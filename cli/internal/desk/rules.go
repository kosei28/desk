package desk

// 形の定義。正本はスキル（skill/SKILL.md）で、ここは同じ形を実装したもの。
// スキルと食い違えばスキルが正しく、ここを直す。

type Kind string

const (
	KindTask      Kind = "task"
	KindArtifact  Kind = "artifact"
	KindInbox     Kind = "inbox"
	KindLibrary   Kind = "library"
	KindPrinciple Kind = "principle"
	KindKnowledge Kind = "knowledge"
	KindMaterial  Kind = "material"
	KindWiki      Kind = "wiki"
	KindProject   Kind = "project"
	KindReadme    Kind = "readme"
	KindAsset     Kind = "asset"
)

// KeyRule は frontmatter のキー 1 つの決まり。
type KeyRule struct {
	Name     string
	List     bool     // 配列
	Link     bool     // 相対パスのリンク
	Required bool     // キーが無ければ挙げる（--fix は空で足す）
	NonEmpty bool     // 値が空なら挙げる
	Enum     []string // 取れる値
	Date     bool     // YYYY-MM-DD
	Note     string   // --rules に出す説明
}

// KindRule は種類 1 つの形。
type KindRule struct {
	Kind       Kind
	Dir        string // 置くディレクトリの名前
	InRoot     bool   // ルートに置く
	InProject  bool   // プロジェクトに置く
	Dated      bool   // 名前が YYYY-MM-DD-<slug>.md
	Nested     bool   // 中を入れ子にしてよい（wiki だけ）
	Assets     bool   // 中に assets/ を置ける
	Archivable bool   // desk archive で archive/ に移す
	NeedFM     bool   // frontmatter が要る
	Keys       []KeyRule
	Sections   []string // desk new が書く節
	Desc       string
}

var (
	Statuses   = []string{"backlog", "todo", "inprogress", "done", "dropped"}
	NeedsHuman = []string{"false", "true"}

	// タスクに要る節（needs_human: false）。needs_human: true は目的と成果物。
	TaskSections      = []string{"目的", "いま", "成果物", "現在地", "決まり", "手順"}
	HumanTaskSections = []string{"目的", "成果物"}
)

const (
	LogSection = "ログ"
	NowSection = "いま"
)

// ルートに置いてよい名前。プロジェクトは projects/ の下に置く。
var RootReserved = []string{"README.md", "principles", "knowledge", "archive", "assets", "projects"}

const (
	ProjectsDir = "projects" // プロジェクトを置くディレクトリ
	ArchiveDir  = "archive"  // 終わったものを置くディレクトリ（プロジェクトの中と、ルート）
	MiscProject = "misc"     // どのプロジェクトにも属さないものを置くプロジェクト
	MarkerFile  = ".desk"    // ルートの印
)

var Rules = []KindRule{
	{
		Kind: KindTask, Dir: "tasks", InProject: true, Dated: true, Archivable: true, NeedFM: true,
		Desc: "タスク。1 タスク 1 ファイル。頭の 3 節（目的・いま・成果物）は人向け、現在地・決まり・手順は AI 向け、最後のログの節は追記だけ",
		Keys: []KeyRule{
			{Name: "status", Required: true, NonEmpty: true, Enum: Statuses, Note: "backlog はやるか決めていない・いつか、todo はやると決めた・未着手、inprogress は着手している"},
			{Name: "needs_human", Required: true, NonEmpty: true, Enum: NeedsHuman, Note: "true は人の判断か作業が要る（確認・人がやる作業）"},
			{Name: "parent", Link: true, Note: "親タスク"},
			{Name: "depends_on", List: true, Link: true, Note: "待っているタスク"},
			{Name: "repo", Note: "~ からのパス"},
			{Name: "branch"},
			{Name: "worktree", Note: "~ からのパス"},
			{Name: "pr", Note: "URL"},
		},
		Sections: TaskSections,
	},
	{
		Kind: KindArtifact, Dir: "artifacts", InProject: true, Assets: true, Archivable: true, NeedFM: true,
		Desc: "タスクの文書とメモ（整理した文書か溜めるメモ。ファイルごとに決まる）。タスクと一緒に役目を終える",
		Keys: []KeyRule{
			{Name: "tasks", List: true, Link: true, Required: true, Note: "どのタスクのための文書か"},
		},
	},
	{
		Kind: KindWiki, Dir: "wiki", InProject: true, Nested: true, Assets: true, NeedFM: true,
		Desc: "プロジェクトの成果物としてメンテする文書。構成はプロジェクトで設計する（入れ子にしてよい）",
		Keys: []KeyRule{
			{Name: "description", Required: true, NonEmpty: true, Note: "何が書いてあるか、いつ読むか"},
			{Name: "verified", Date: true, Note: "確かめたが変えなかった日（任意）"},
		},
	},
	{
		Kind: KindInbox, Dir: "inbox", InProject: true, Dated: true,
		Desc: "まだどこにも属さない未整理のものの受け箱。棚卸しして空にする。frontmatter は任意",
		Keys: []KeyRule{
			{Name: "about", Link: true, Note: "関係するタスクや文書（任意）"},
		},
	},
	{
		Kind: KindLibrary, Dir: "library", InProject: true, Dated: true,
		Desc: "あとで参照したい情報の控え。書いた時点のまま残し、メンテしない。frontmatter は全部任意",
		Keys: []KeyRule{
			{Name: "url"},
			{Name: "source", Note: "URL がないときの出どころ。自由な文（リンクではない）"},
			{Name: "tags", List: true},
			{Name: "about", Link: true, Note: "きっかけになったタスク"},
		},
		Sections: []string{"要約"},
	},
	{
		Kind: KindPrinciple, Dir: "principles", InRoot: true, InProject: true, NeedFM: true,
		Desc: "抽象的で根源的な考え方。ルートは全プロジェクト、プロジェクトのものはそこだけ",
		Keys: []KeyRule{
			{Name: "description", Required: true, NonEmpty: true},
			{Name: "principle", Link: true, Note: "プロジェクトの原則が具体化しているルートの原則"},
			{Name: "verified", Date: true},
		},
	},
	{
		Kind: KindKnowledge, Dir: "knowledge", InRoot: true, InProject: true, NeedFM: true,
		Desc: "AI が作業で使う決まり・手順・事実。段 0 は description、段 1 は本文、段 2 は本文が指す詳細",
		Keys: []KeyRule{
			{Name: "description", Required: true, NonEmpty: true, Note: "いつ読むかと何が書いてあるか"},
			{Name: "principle", Required: true, NonEmpty: true, Link: true, Note: "どの原則の当てはめか（1 つ以上。1 つなら文字列、複数ならリスト）。詳細（ほかのナレッジの本文から指されるもの）には要らない"},
			{Name: "sources", List: true, Link: true, Note: "根拠（素材、決定）"},
			{Name: "verified", Date: true},
		},
	},
	{
		Kind: KindMaterial, Dir: "materials", InProject: true, Dated: true, NeedFM: true,
		Desc: "ナレッジの素材（指摘、やり直し、思っていたのと違ったこと）。1 件 1 ファイル",
		Keys: []KeyRule{
			{Name: "source", Link: true, Note: "起きた場所。ツリーにあるときだけ書く"},
			{Name: "knowledge", List: true, Link: true, Required: true, Note: "蒸留してできたナレッジ。空なら蒸留待ち"},
		},
		Sections: []string{"何が起きたか", "一般化の候補"},
	},
}

var (
	ruleByKind = map[Kind]*KindRule{}
	ruleByDir  = map[string]*KindRule{}
)

func init() {
	for i := range Rules {
		r := &Rules[i]
		ruleByKind[r.Kind] = r
		ruleByDir[r.Dir] = r
	}
}

// RuleOf は種類の形を返す。無ければ nil。
func RuleOf(k Kind) *KindRule { return ruleByKind[k] }

// KeyRuleOf はその種類の frontmatter のキーの決まりを返す。
func KeyRuleOf(k Kind, key string) *KeyRule {
	r := ruleByKind[k]
	if r == nil {
		return nil
	}
	for i := range r.Keys {
		if r.Keys[i].Name == key {
			return &r.Keys[i]
		}
	}
	return nil
}

// genericLinkKeys は種類が分からないファイルでリンクとして読むキー。
var genericLinkKeys = []string{"parent", "depends_on", "tasks", "about", "principle", "sources", "source", "knowledge"}

// LinkKeys はその種類で相対パスのリンクとして読む frontmatter のキー。
func LinkKeys(k Kind) []string {
	r := ruleByKind[k]
	if r == nil {
		return genericLinkKeys
	}
	var keys []string
	for _, kr := range r.Keys {
		if kr.Link {
			keys = append(keys, kr.Name)
		}
	}
	return keys
}

// AllKinds は ls で選べる種類。
var AllKinds = []Kind{KindTask, KindArtifact, KindInbox, KindLibrary, KindPrinciple, KindKnowledge, KindMaterial, KindWiki, KindProject}

func contains(list []string, s string) bool {
	for _, x := range list {
		if x == s {
			return true
		}
	}
	return false
}
