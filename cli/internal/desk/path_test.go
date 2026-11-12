package desk

import "testing"

func TestClassify(t *testing.T) {
	tests := []struct {
		path            string
		kind            Kind
		project         string
		archived        bool
		projectArchived bool
		problem         bool
	}{
		{path: "README.md", kind: KindReadme},
		{path: "principles/x.md", kind: KindPrinciple},
		{path: "knowledge/x.md", kind: KindKnowledge},
		{path: "assets/a.png", kind: KindAsset},
		{path: "projects/app/README.md", kind: KindReadme, project: "app"},
		{path: "projects/app/tasks/2026-01-02-x.md", kind: KindTask, project: "app"},
		{path: "projects/app/artifacts/x.md", kind: KindArtifact, project: "app"},
		{path: "projects/app/artifacts/assets/a.png", kind: KindAsset, project: "app"},
		{path: "projects/app/wiki/a/b/c.md", kind: KindWiki, project: "app"},
		{path: "projects/app/archive/tasks/2026-01-02-x.md", kind: KindTask, project: "app", archived: true},
		{path: "projects/app/archive/artifacts/assets/a.png", kind: KindAsset, project: "app", archived: true},
		{path: "projects/misc/inbox/2026-01-02-memo.md", kind: KindInbox, project: "misc"},
		{path: "archive/projects/app/tasks/2026-01-02-x.md", kind: KindTask, project: "app", archived: true, projectArchived: true},
		{path: "archive/projects/app/archive/tasks/2026-01-02-x.md", kind: KindTask, project: "app", archived: true, projectArchived: true},

		{path: "notes.md", problem: true},
		{path: "app/tasks/2026-01-02-x.md", problem: true},
		{path: "projects/x.md", problem: true},
		{path: "projects/app/notes.md", project: "app", problem: true},
		{path: "projects/app/drafts/x.md", project: "app", problem: true},
		{path: "projects/app/tasks/sub/x.md", kind: KindTask, project: "app", problem: true},
		{path: "projects/app/tasks/a.png", kind: KindTask, project: "app", problem: true},
		{path: "projects/app/archive/wiki/x.md", project: "app", archived: true, problem: true},
		{path: "archive/tasks/x.md", problem: true},
	}
	for _, tt := range tests {
		t.Run(tt.path, func(t *testing.T) {
			got := Classify(tt.path)
			if got.Kind != tt.kind || got.Project != tt.project || got.Archived != tt.archived || got.ProjectArchived != tt.projectArchived || (got.Problem != "") != tt.problem {
				t.Errorf("Classify(%q) = %+v", tt.path, got)
			}
		})
	}
}

func TestArchivePath(t *testing.T) {
	tests := []struct{ from, to string }{
		{"projects/app/tasks/x.md", "projects/app/archive/tasks/x.md"},
		{"projects/app/artifacts/assets/a.png", "projects/app/archive/artifacts/assets/a.png"},
		{"archive/projects/app/tasks/x.md", "archive/projects/app/tasks/x.md"},
		{"principles/x.md", "principles/x.md"},
	}
	for _, tt := range tests {
		if got := ArchivePath(Classify(tt.from)); got != tt.to {
			t.Errorf("ArchivePath(%q) = %q, want %q", tt.from, got, tt.to)
		}
		if tt.from != tt.to {
			if back := UnarchivePath(Classify(tt.to)); back != tt.from {
				t.Errorf("UnarchivePath(%q) = %q, want %q", tt.to, back, tt.from)
			}
		}
	}
}

func TestNameProblem(t *testing.T) {
	tests := []struct {
		path    string
		problem bool
	}{
		{"projects/app/tasks/2026-11-09-x.md", false},
		{"projects/app/tasks/x.md", true},
		{"projects/app/tasks/2026-13-40-x.md", true},
		{"projects/app/artifacts/x.md", false},
		{"projects/app/materials/x.md", true},
	}
	for _, tt := range tests {
		if got := NameProblem(Classify(tt.path)); (got != "") != tt.problem {
			t.Errorf("NameProblem(%q) = %q", tt.path, got)
		}
	}
}
