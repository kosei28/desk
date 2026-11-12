package desk

import (
	"path"
	"regexp"
	"strings"
	"time"
)

// Info はルートからのパスで決まること（種類、プロジェクト、archive の中か）。
type Info struct {
	Path            string // ルートからのパス（/ 区切り）
	Kind            Kind   // 分からなければ空
	Project         string // ルートのものは空
	Archived        bool   // archive/ の中（プロジェクトの archive/ か、ルートの archive/ のプロジェクト）
	ProjectArchived bool   // ルートの archive/ に移ったプロジェクトの中
	Asset           bool   // assets/ の中の静的ファイル
	Problem         string // 決まりの外の場所なら、その説明
}

var datedName = regexp.MustCompile(`^(\d{4}-\d{2}-\d{2})-(.+)\.md$`)

// Name はファイル名。
func (i Info) Name() string { return path.Base(i.Path) }

// ProjectDir はそのファイルがあるプロジェクトのディレクトリ（projects/<p>、archive に移ったものは archive/projects/<p>）。ルートのものは空。
func (i Info) ProjectDir() string {
	if i.Project == "" {
		return ""
	}
	return projectDir(i.Project, i.ProjectArchived)
}

// projectDir はプロジェクトのディレクトリのルートからのパス。
func projectDir(name string, archived bool) string {
	if archived {
		return ArchiveDir + "/" + ProjectsDir + "/" + name
	}
	return ProjectsDir + "/" + name
}

// Classify はルートからのパス（ファイル）の置き場所を決める。
func Classify(rel string) Info {
	info := Info{Path: rel}
	parts := strings.Split(rel, "/")
	switch parts[0] {
	case ArchiveDir:
		if len(parts) < 4 || parts[1] != ProjectsDir {
			info.Problem = "ルートの archive/ には projects/<プロジェクト>/ だけを置く"
			return info
		}
		info.ProjectArchived = true
		info.Archived = true
		classifyInProject(&info, parts[2], parts[3:])
		return info
	case ProjectsDir:
		if len(parts) < 3 {
			info.Problem = "projects/ の中にはプロジェクトのディレクトリだけを置く"
			return info
		}
		classifyInProject(&info, parts[1], parts[2:])
		return info
	}
	if len(parts) == 1 {
		if parts[0] == "README.md" {
			info.Kind = KindReadme
			return info
		}
		info.Problem = "ルートに置けるファイルは README.md だけ（ほかは予約名のディレクトリかプロジェクトに）"
		return info
	}
	switch parts[0] {
	case "assets":
		info.Kind = KindAsset
		info.Asset = true
		if len(parts) != 2 {
			info.Problem = "assets/ の中はフラットにする"
		}
		return info
	case "principles", "knowledge":
		r := ruleByDir[parts[0]]
		classifyKindFile(&info, r, parts)
		return info
	}
	info.Problem = "ルートに置けるのは予約名だけ（プロジェクトは projects/ の下に置く）"
	return info
}

func classifyInProject(info *Info, project string, rest []string) {
	info.Project = project
	if len(rest) == 1 {
		if rest[0] == "README.md" {
			info.Kind = KindReadme
			return
		}
		info.Problem = "プロジェクトの直下に置けるファイルは README.md だけ"
		return
	}
	switch rest[0] {
	case "assets":
		info.Kind = KindAsset
		info.Asset = true
		if len(rest) != 2 {
			info.Problem = "assets/ の中はフラットにする"
		}
		return
	case "archive":
		if info.Archived && !info.ProjectArchived {
			info.Problem = "archive/ の中に archive/ を置かない"
			return
		}
		info.Archived = true
		if len(rest) < 3 {
			info.Problem = "プロジェクトの archive/ の中はプロジェクトと同じ形（archive/tasks/・archive/artifacts/）"
			return
		}
		r := ruleByDir[rest[1]]
		if r == nil || !r.Archivable {
			info.Problem = "プロジェクトの archive/ に置くのは tasks/ と artifacts/ だけ"
			return
		}
		classifyKindFile(info, r, rest[1:])
		return
	}
	r := ruleByDir[rest[0]]
	if r == nil || !r.InProject {
		info.Problem = "プロジェクトに置けないディレクトリ（" + rest[0] + "/）"
		return
	}
	classifyKindFile(info, r, rest)
}

// classifyKindFile は種類のディレクトリ（sub[0]）の中のファイルを決める。
func classifyKindFile(info *Info, r *KindRule, sub []string) {
	info.Kind = r.Kind
	name := sub[len(sub)-1]
	if len(sub) >= 3 && sub[len(sub)-2] == "assets" && r.Assets {
		info.Kind = KindAsset
		info.Asset = true
		if !r.Nested && len(sub) != 3 {
			info.Problem = "assets/ の中はフラットにする"
		}
		return
	}
	for _, p := range sub[1 : len(sub)-1] {
		if p == "assets" {
			info.Problem = "assets/ の中はフラットにする"
			return
		}
	}
	if !r.Nested && len(sub) != 2 {
		info.Problem = r.Dir + "/ の中はフラットにする"
		return
	}
	if !strings.HasSuffix(name, ".md") {
		if r.Assets {
			info.Problem = "静的ファイルは " + r.Dir + "/assets/ に置く"
		} else {
			info.Problem = r.Dir + "/ には .md だけを置く"
		}
		return
	}
}

// NameProblem はファイル名の形の問題を返す（無ければ空）。
func NameProblem(info Info) string {
	r := RuleOf(info.Kind)
	if r == nil || !r.Dated || info.Asset {
		return ""
	}
	m := datedName.FindStringSubmatch(info.Name())
	if m == nil {
		return "名前に日付が無い（YYYY-MM-DD-<slug>.md にする）"
	}
	if _, err := time.Parse("2006-01-02", m[1]); err != nil {
		return "名前の日付が日付になっていない（" + m[1] + "）"
	}
	return ""
}

// uniqueScope は名前が一意であるべき範囲の鍵（プロジェクトの archive/ の中と外は同じ範囲）。
func uniqueScope(info Info) string {
	r := RuleOf(info.Kind)
	if r == nil {
		return ""
	}
	return info.ProjectDir() + "|" + r.Dir
}

// SplitDated は YYYY-MM-DD-<slug>.md を日付と slug に分ける。
func SplitDated(name string) (date, slug string, ok bool) {
	m := datedName.FindStringSubmatch(name)
	if m == nil {
		return "", "", false
	}
	return m[1], m[2], true
}

// ArchivePath はプロジェクトの中のファイルを archive/ に移した先のパスを返す。
func ArchivePath(info Info) string {
	if info.ProjectArchived || info.Project == "" {
		return info.Path
	}
	pd := info.ProjectDir()
	return pd + "/" + ArchiveDir + "/" + strings.TrimPrefix(info.Path, pd+"/")
}

// UnarchivePath は archive/ の中のファイルの元の場所を返す。
func UnarchivePath(info Info) string {
	pd := info.ProjectDir()
	if pd == "" {
		return info.Path
	}
	rest := strings.TrimPrefix(info.Path, pd+"/")
	if strings.HasPrefix(rest, ArchiveDir+"/") {
		return pd + "/" + strings.TrimPrefix(rest, ArchiveDir+"/")
	}
	return info.Path
}
