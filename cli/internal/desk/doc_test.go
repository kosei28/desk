package desk

import (
	"reflect"
	"strings"
	"testing"
)

func TestParseDocFrontmatterAndLinks(t *testing.T) {
	content := strings.Join([]string{
		"---",
		"status: todo",
		"parent: 2026-09-07-meal-planner.md",
		"depends_on: [2026-10-14-shopping-list.md, \"2026-10-20-sync.md\"]",
		"---",
		"",
		"# 献立を印刷できるようにする",
		"",
		"## 目的",
		"[設計](../artifacts/design.md#背景) と ![図](assets/a.png)、[外](https://example.com)、[[名前]]",
		"",
		"```",
		"[コードの中](ignored.md)",
		"```",
		"## いま",
		"レビュー待ち。",
	}, "\n")
	d := ParseDoc(Classify("projects/app/tasks/2026-11-09-home.md"), content)
	if !d.HasFM || d.FMErr != "" {
		t.Fatalf("frontmatter を読めない: %q", d.FMErr)
	}
	if d.Status() != "todo" || d.Owner() != "" {
		t.Errorf("status=%q owner=%q", d.Status(), d.Owner())
	}
	if got := d.List("depends_on"); !reflect.DeepEqual(got, []string{"2026-10-14-shopping-list.md", "2026-10-20-sync.md"}) {
		t.Errorf("depends_on = %v", got)
	}
	if d.Title() != "献立を印刷できるようにする" {
		t.Errorf("Title = %q", d.Title())
	}
	if d.Now() != "レビュー待ち。" {
		t.Errorf("Now = %q", d.Now())
	}

	type got struct {
		key, target string
		form        LinkForm
	}
	var links []got
	for _, l := range d.Links {
		links = append(links, got{l.Key, l.Target, l.Form})
		if l.Start >= 0 && !strings.HasPrefix(d.Lines[l.Line][l.Start:], l.Raw) {
			t.Errorf("リンク %q の位置がずれている（%d 行目 %d バイト目）", l.Raw, l.Line, l.Start)
		}
	}
	want := []got{
		{"parent", "projects/app/tasks/2026-09-07-meal-planner.md", FormRelative},
		{"depends_on", "projects/app/tasks/2026-10-14-shopping-list.md", FormRelative},
		{"depends_on", "projects/app/tasks/2026-10-20-sync.md", FormRelative},
		{"body", "projects/app/artifacts/design.md", FormRelative},
		{"body", "projects/app/tasks/assets/a.png", FormRelative},
		{"body", "", FormExternal},
		{"body", "名前", FormWiki},
	}
	if !reflect.DeepEqual(links, want) {
		t.Errorf("links =\n%v\nwant\n%v", links, want)
	}
}

func TestParseDocBrokenFrontmatter(t *testing.T) {
	d := ParseDoc(Classify("projects/app/tasks/2026-11-09-x.md"), "---\nstatus: [active\n---\n# x\n")
	if d.FMErr == "" {
		t.Error("壊れた frontmatter を読めてしまう")
	}
	d = ParseDoc(Classify("projects/app/tasks/2026-11-09-x.md"), "---\nstatus: active\n# x\n")
	if d.FMErr == "" {
		t.Error("閉じていない frontmatter を読めてしまう")
	}
}

func TestApplyEdits(t *testing.T) {
	lines := []string{"a [x](one.md) [y](two.md)", "b"}
	out, err := ApplyEdits(lines, []Edit{
		{Line: 0, Start: 6, Old: "one.md", New: "../one.md"},
		{Line: 0, Start: 18, Old: "two.md", New: "2.md"},
	})
	if err != nil {
		t.Fatal(err)
	}
	if out[0] != "a [x](../one.md) [y](2.md)" {
		t.Errorf("got %q", out[0])
	}
	if _, err := ApplyEdits(lines, []Edit{{Line: 0, Start: 0, Old: "zzz", New: "q"}}); err == nil {
		t.Error("置換元が違うのに書き換えられてしまう")
	}
}
