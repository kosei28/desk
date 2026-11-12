// desk は作業ドキュメントのツリーの形を扱う補助ツール。決まりの正本はスキル desk（skill/SKILL.md）。
package main

import (
	"os"

	"github.com/kosei28/desk/cli/internal/desk"
)

func main() {
	os.Exit(desk.Run(os.Args[1:], os.Stdin, os.Stdout, os.Stderr))
}
