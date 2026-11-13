package desk

import (
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"strings"
	"unicode"

	"github.com/spf13/pflag"
	"golang.org/x/text/width"
)

const usage = `desk — 作業ドキュメントのツリーの形を扱う補助ツール（決まりの正本はスキル desk）

作る:
  desk init [<パス>]
  desk new project <名前>
  desk new <種類> <slug> [--project <名前>] [--parent <タスク>] [--needs-human] [--title <見出し>]
引く:
  desk ls [<種類>] [--name <名前>] [--project <名前>] [--archived] [--where <キー><演算子><値>]...
          [--modified-before <日数>] [--modified-after <日数>] [--fields <列>,...] [--json | --paths | --level 0|1|2]
  desk graph <パス> [--in | --out] [--via <キー>,...] [--depth <n>|all] [--format tree|paths|dot] [--level 0|1|2]
  desk graph --all [--via <キー>,...] [--format edges|dot]
読む:
  desk read <パス>... [--level 0|1|2] [--outline] [--section <見出し>,...]
  desk read --stdin [...]
確かめる:
  desk check [--project <名前>] [--name <名前>] [--fix] [--rules] [--json]
動かす:
  desk archive <パス>... [--status done|dropped] [--dry-run]
  desk archive --project <名前> [--dry-run]
  desk mv <元>... <先> [--dry-run]
そのほか:
  desk root                  # いまのルート

種類: task artifact inbox library principle knowledge material wiki project
ルートは、作業ディレクトリから上にたどって .desk のあるディレクトリ。ツリーの外ではエラーで止まる。
desk root でいまのルートを出す。書く操作はコミットしない（init の最初のコミットだけ例外）。
`

// exitError は終了コードを持つエラー。
type exitError struct {
	code int
	msg  string
}

func (e *exitError) Error() string { return e.msg }

// errFound は check で何か見つかったとき（メッセージは出力済み）。
var errFound = &exitError{code: 1}

type env struct {
	stdin  io.Reader
	stdout io.Writer
	stderr io.Writer
}

// Run は CLI の入口。終了コードを返す。
func Run(args []string, stdin io.Reader, stdout, stderr io.Writer) int {
	e := &env{stdin: stdin, stdout: stdout, stderr: stderr}
	if len(args) == 0 || args[0] == "help" || args[0] == "-h" || args[0] == "--help" {
		fmt.Fprint(stdout, usage)
		return 0
	}
	cmds := map[string]func(*env, []string) error{
		"init":    cmdInit,
		"root":    cmdRoot,
		"new":     cmdNew,
		"ls":      cmdLs,
		"graph":   cmdGraph,
		"read":    cmdRead,
		"check":   cmdCheck,
		"archive": cmdArchive,
		"mv":      cmdMv,
	}
	f, ok := cmds[args[0]]
	if !ok {
		fmt.Fprintf(stderr, "desk: 知らないコマンド %q\n\n%s", args[0], usage)
		return 2
	}
	err := f(e, args[1:])
	if err == nil {
		return 0
	}
	if errors.Is(err, pflag.ErrHelp) {
		return 0
	}
	var ee *exitError
	if errors.As(err, &ee) {
		if ee.msg != "" {
			fmt.Fprintln(stderr, "desk:", ee.msg)
		}
		return ee.code
	}
	fmt.Fprintln(stderr, "desk:", err)
	return 1
}

func newFlags(name string, e *env) *pflag.FlagSet {
	fs := pflag.NewFlagSet("desk "+name, pflag.ContinueOnError)
	fs.SetOutput(e.stderr)
	fs.SetInterspersed(true)
	fs.SortFlags = false
	return fs
}

func usageErr(format string, a ...any) error {
	return &exitError{code: 2, msg: fmt.Sprintf(format, a...)}
}

// cmdRoot はいまのルートの絶対パスを出す。
func cmdRoot(e *env, args []string) error {
	fs := newFlags("root", e)
	fs.Usage = func() { fmt.Fprintln(e.stderr, "使い方: desk root") }
	if err := fs.Parse(args); err != nil {
		return err
	}
	if fs.NArg() != 0 {
		return usageErr("desk root に引数は付けない")
	}
	root, err := FindRoot()
	if err != nil {
		return err
	}
	fmt.Fprintln(e.stdout, root)
	return nil
}

func loadTree() (*Tree, error) {
	root, err := FindRoot()
	if err != nil {
		return nil, err
	}
	return LoadTree(root)
}

// table は列をそろえて出す。列の幅は表示の幅（全角は 2）で数える。
func table(w io.Writer, header []string, rows [][]string) {
	var all [][]string
	if header != nil {
		all = append(all, header)
	}
	for _, r := range rows {
		cells := make([]string, len(r))
		for i, c := range r {
			c = strings.ReplaceAll(c, "\t", " ")
			if c == "" {
				c = "-"
			}
			cells[i] = c
		}
		all = append(all, cells)
	}
	var widths []int
	for _, r := range all {
		for i, c := range r {
			if i >= len(widths) {
				widths = append(widths, 0)
			}
			widths[i] = max(widths[i], displayWidth(c))
		}
	}
	for _, r := range all {
		var b strings.Builder
		for i, c := range r {
			b.WriteString(c)
			if i < len(r)-1 {
				b.WriteString(strings.Repeat(" ", widths[i]-displayWidth(c)+2))
			}
		}
		fmt.Fprintln(w, b.String())
	}
}

// displayWidth は端末に出したときの幅。全角・幅広の文字は 2、結合文字は 0。
func displayWidth(s string) int {
	n := 0
	for _, r := range s {
		switch {
		case unicode.Is(unicode.Mn, r) || unicode.Is(unicode.Me, r) || r == '\u200d':
		case width.LookupRune(r).Kind() == width.EastAsianWide || width.LookupRune(r).Kind() == width.EastAsianFullwidth:
			n += 2
		default:
			n++
		}
	}
	return n
}

func writeJSON(w io.Writer, v any) error {
	enc := json.NewEncoder(w)
	enc.SetIndent("", "  ")
	enc.SetEscapeHTML(false)
	return enc.Encode(v)
}

func splitComma(s string) []string {
	var out []string
	for _, p := range strings.Split(s, ",") {
		if p = strings.TrimSpace(p); p != "" {
			out = append(out, p)
		}
	}
	return out
}
