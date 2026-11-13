# CLI `desk`

ツリーの「形」を扱う補助ツール。設計上の位置づけは [operation.md](operation.md)、扱うツリーの形は [tree.md](tree.md)。

## 目的

CLI の目的は 1 つ。**ツリーの形を、人とエージェントが毎回読んで手で守らなくても済むようにする。** 形とは、ディレクトリの構成、ファイルの名前と節、frontmatter のキーと値、相対パスのリンク、`assets/`、git の最終更新。形を扱う操作は 5 つで、コマンドはそれに対応する。

| 形を… | コマンド | 素の操作でやると |
| --- | --- | --- |
| 作る | `init`（ルート）、`new`（プロジェクト・タスク・各種ファイル） | 形を見ながら手で書く。雛形が無いので節やキーが抜ける |
| 引く | `ls`（frontmatter と git で絞る）、`graph`（リンクのグラフをたどる） | YAML の配列や何段ものリンクで、毎回スクリプトを書く |
| 読む | `read`（概要だけ・見出しだけ・節だけ・本文・詳細まで） | 「概要だけ」が切り出せず `cat` で全部読む |
| 確かめる | `check`（形に合っているか。ツリーの中だけで分かるずれ。機械的な直しは `--fix`） | 決まりを全部覚えて 1 ファイルずつ見る。見落とす |
| 動かす | `archive`（まとめて同じ形で `archive/` へ。タスクには `status` を書く）、`mv`（任意の移動）。どちらもリンクを付け替える | `git mv` のたびに `grep -rl` で古いパスを探して直す。漏れてリンクが切れる |

形の定義（種類ごとの置き場所、名前の形、frontmatter のキーと値、必須の節、予約名）の正本はスキルで、CLI は同じものを実装する。`desk rules` は CLI が実装している形の一覧で、スキルと食い違えばスキルが正しく、CLI を直す。

CLI にしないもの: 中身を書く、何を読むか決める（組み立て）、外の世界（コードリポジトリ・PR・worktree）と突き合わせる、コミット、ロックや hook や「CLI を通さないと書けない」仕組み。CLI は補助ツールで、無くても形の一覧とスキルを読めば手でできる（「概要だけ読む」だけは `cat` で全部読むことになる）。

## 前提

- Rust の単一バイナリ。外部依存は `git` だけ。コードは仕組みのリポジトリの `cli/`（ツリーの形の解釈は `core/` のライブラリ `desk-core` にある）で、`cargo install --locked --path cli` でバイナリを入れる
- 印のファイル `.desk` があるディレクトリがルートで、作業ディレクトリから上にたどって見つける。見つからなければ（ツリーの外で実行したら）何も書き換えずにエラーで止まり、推測して続けない。`desk root` がいまのルートを出す
- 読むたびにツリー全体を走査する（数千ファイルまでなら十分速い）。保存しない。設定ファイルも持たない
- 書く操作（`init`・`new`・`archive`・`mv`・`check --fix`）はファイルを作る・動かす・直すだけで、コミットはしない（例外は `init` の最初のコミットだけ）。コミットはエージェント（か人）が `git add <パス>` でやる。既にあるものは作らず、済んでいることはやらない（何度やっても同じ）
- ファイルの指定はパス（作業ディレクトリから、またはルートから）。名前だけでも、ツリーの中で 1 件に決まれば通る。複数あれば、作業ディレクトリのプロジェクトで絞り、それでも決まらなければ候補を出して止まる
- 出力は人もモデルも読める表。`--json` と `--paths`（パスだけ）と `--level 0|1|2`（当たったものを `read` のその段で出す）を持ち、どれか 1 つを選ぶ
- 複数の値を取るフラグ（`--via`・`--fields`・`--section`、`ls` の種類）は、カンマ区切りでも、繰り返しても書ける（`--via parent,tasks` も `--via parent --via tasks` も同じ）
- 終了コードは、0 が成功、1 が失敗（`desk: …` を出す）か `check` で見つけたとき、2 が引数の誤り（組めないフラグを組んだときも）。`-h`・`--help` は使い方を出す

## `desk init`・`desk root`

```
desk init [<パス>]        # 省略時は作業ディレクトリ
desk root                 # いまのルートの絶対パス
```

- ルートを作る: 印の `.desk`、`README.md`（概要。何のツリーか、ツリー図、人が書くときの決まり 3 行、詳しくはスキル `desk`）、プロジェクト `projects/misc/` の README、`.gitignore`。空のディレクトリは作らない（`principles/`・`knowledge/`・`archive/`・`assets/` は要るときに作る）。どこに置いたかはツリーにも仕組みにも書かない
- そのディレクトリが git リポジトリのいちばん上でなければ `git init` する。まだコミットが無ければ、作ったファイルで最初のコミットを作る（これだけは CLI がコミットする。空のリポジトリに履歴を作るため）。コミットがあるリポジトリではコミットせず、作ったファイルを出す
- 既にあるファイルは触らず、足りないものだけ作る。もうルート（`.desk` がある）なら何もしない。ツリーの中（上のディレクトリに `.desk` がある）には作らず、何も書き換えずにエラーで止まる（ツリーの中にツリーを作らない）
- `desk root` は、エージェントがコミットや素の操作でルートを使うとき（`git -C "$(desk root)" …`）のため

## `desk new`

```
desk new project <名前>
desk new <種類> <slug> [--project <名前>] [--parent <タスク>] [--needs-human] [--title <見出し>]
```

- 種類は `task` `artifact` `inbox` `library` `principle` `knowledge` `material` `wiki`。日付が要る種類（`task` `material` `inbox` `library`）は今日の日付を付ける。日付付きの slug を渡すと、その日付で作る
- frontmatter と節の見出しを形どおりに書いたファイルを、正しい置き場所に作る。中身は書かない（中身はエージェントと人）。ディレクトリが無ければ作る（要るときに作る、の原則どおり空のディレクトリは作らない）
- 同じ種類に同じ名前が既にあれば（`archive/` の中も含めて）作らずに止まり、そのパスを出す。同じ日に同じ slug で 2 つのセッションが作ろうとしても 2 つにならない
- `project` はプロジェクトのディレクトリと README.md（概要）だけ
- `task` は `status: todo` と `needs_human`（`--needs-human` なら `true`、無ければ `false`）を書き、`--parent` は相対パスにして書く（プロジェクトは親から決まる）。ログの節は要るときにエージェントが足す
- `artifact` は `--parent <タスク>` を `tasks` に書く。`principle` と `knowledge` は `--project` が無ければルートに作る。`wiki` の slug は `meal-planner/overview` のように入れ子で渡せる
- 作ったパスを出すので、エージェントはそのまま中身を書いて `git add <パス>` する

## `desk ls`

```
desk ls [<種類>,...] [--name <名前>] [--project <名前>] [--archived]
        [--where <キー><演算子><値>]... [--modified-before <日数|日付>] [--modified-after <日数|日付>]
        [--fields <列>,...] [--json | --paths | --level 0|1|2]
```

- 種類は `task` `artifact` `inbox` `library` `principle` `knowledge` `material` `wiki` `project`。複数を並べてもよく、省略すると全部。`archive/` の中は `--archived` で含める
- `--name` はファイル名で引く。同じ名前が複数のプロジェクトにあれば全部出す（`--project` で絞る）
- `--project <p>` の原則とナレッジには、ルートのものも入る
- `--where <キー><演算子><値>`。演算子は `=` `!=` `~`（含む。配列は要素のどれか、文字列は部分一致、`~a,b` はどれか）`<` `>`（日付と数）。複数は AND。キーは frontmatter のどれでも。`[]` は空の配列。パスの値は名前だけでも当たる
- `--modified-before`・`--modified-after`: 最終コミットの日付で絞る。値は日数（`7` は 7 日前）か日付（`2026-11-01`）。before はその日より前、after はその日以降
- `--fields`: `name` `path` `project` `kind` `modified`、frontmatter のキー、本文から取る `title`（見出し）と `now`（「いま」の 1 行目）。`--paths`・`--level` とは組めない
- 例

  ```
  desk ls task --where status=inprogress --fields name,repo,branch,pr,modified
  desk ls task --where needs_human=true --where status~todo,inprogress
  desk ls task --where 'status~done,dropped'                       # archive/ の外に残っているもの
  desk ls artifact --where tasks~2026-11-09-printable-menu --level 1   # パスの部分一致
  desk ls material --where knowledge=[] --project recipe-app          # 蒸留待ち
  desk ls knowledge --where principle=prefer-structure-over-patches
  desk ls library --where tags~pagination --fields name,url,title
  ```

## `desk graph`

```
desk graph <パス> [--in | --out] [--via <キー>,...] [--depth <n> | all]
           [--format tree | paths | dot] [--level 0|1|2]
desk graph --all [--via ...] [--format edges | dot]
```

- ノードはファイル（`assets/` の中も含む）、辺は相対パスのリンクで、frontmatter のキーが辺の種類になる（本文の中は `body`）。方向は「書いてあるファイル → 指された先」。`--in` は指されているもの、`--out` は指しているもの、省略は両方を 1 段
- 出力の各行はパス、種類（タスクは `status` も）、`description`（無ければ見出し）。`--level` を付けると各ノードを `read` のその段で出す
- `--all` はツリー全体の辺を出す（`--format` は `edges` か `dot`、既定は `edges`）。`--in`・`--out`・`--depth`・`--level` とは組めない
- 例

  ```
  desk graph projects/recipe-app/tasks/2026-09-07-meal-planner.md --in --via parent --depth all --format tree   # 子孫の木
  desk graph 2026-11-11-print-preview.md --out --via depends_on --depth all   # 何を待っているか（名前だけ）
  desk graph 2026-10-14-meal-planner-shopping-list.md --in --via depends_on   # これを待っているもの
  desk graph principles/prefer-structure-over-patches.md --in --via principle       # この原則に紐づくナレッジ
  desk graph projects/recipe-app/wiki/meal-planner/overview.md --in                       # この wiki を指しているもの
  desk graph 2026-11-09-printable-menu.md --in --format paths            # 閉じるとき、一緒に archive に移す候補
  desk graph --all --via parent,depends_on --format dot | dot -Tsvg             # タスクの全体図
  ```

## `desk read`

```
desk read <パス>... [--level 0 | 1 | 2] [--outline] [--section <見出し>,...]
desk ls ... --paths | desk read - [...]   # `-` は標準入力の 1 行を 1 つのパスとして読む
```

- パス（か、1 件に決まる名前）で引く。省略時は `--level 1`
- `--level 0`: 概要。パス、種類、プロジェクト、`description`。タスクなら `status`・人が要るなら `needs_human`・見出し・「いま」の 1 行目も。1 件 1〜3 行
- `--level 1`: そのファイル全体（frontmatter と本文）
- `--level 2`: そのファイルと、本文がリンクで指す `.md`（1 段）。指す先はまず `--level 0` で並べ、本文は指された順に続ける
- `--outline`: 見出しと、各節の行数だけ（`--section` とは組めない）
- `--section <見出し>,...`: その節だけ（見出しの行から、次の同じ深さの見出しの前まで）。無い見出しは注記して飛ばす
- 例

  ```
  desk read principles/prefer-structure-over-patches.md             # 段 1
  desk read knowledge/fix-by-design-not-by-patch.md --level 2       # 段 2（詳細まで）
  desk read 2026-09-07-meal-planner.md --section 目的,いま,決まり     # 親タスクの頭と決まりだけ（名前だけ）
  desk read projects/recipe-app/wiki/meal-planner/overview.md --outline   # 長い wiki の見出しだけ
  desk ls knowledge --project recipe-app --level 0                     # 段 0（ls 側で同じこと）
  ```

## `desk check`

```
desk check [--project <名前>] [--name <名前>] [--fix] [--json]
desk rules
```

形に合っているかと、ツリーの中だけで分かるずれを挙げる。1 行 = ファイル、何が、どう直すか。見つかれば終了コードは 1（何かを止めるためではなく、スクリプトから使えるように）。`--name` はそのファイルと、関係するファイルの分を出す。定期的には回さない。

見るもの:

- **ディレクトリ。** ルートに決まった名前以外のもの、`projects/` の直下にプロジェクト以外のもの、プロジェクトに決まり以外のディレクトリ、`archive/` の中の形
- **ファイルの名前。** 日付が要る種類に日付が無い、同じ種類の中で名前が重複している（`archive/` の中と外に同じ名前）
- **frontmatter。** 無い、必須キーが無い、必須キーの値が空、`status`・`needs_human` の値が決まりの外、日付のキーが日付でない
- **リンク。** 本文と frontmatter のリンクの先が無い（切れたリンク）、相対パスでないリンク（絶対パス、`~` 始まり、名前だけ）、`assets/` の中にどこからも指されていないファイル
- **内容の構成。** タスクに必須の節（目的・いま・成果物・現在地・決まり・手順。`needs_human: true` は目的・成果物）が無い、ログの節が最後にない。ナレッジに `description` か `principle` が無い（ほかのナレッジの本文から指されている詳細には `principle` を求めない）
- **ツリーの中のずれ。** `done`・`dropped` のタスクが `archive/` の外（逆も）。`depends_on` に `done` でないものがあるのに `inprogress`・`done`、`todo` が `backlog` に依存、`dropped` 以外が `dropped` に依存。子が `inprogress` なのに親が `backlog`・`todo`、子が `todo` なのに親が `backlog`、親が終わっているのに子が終わっていない。`parent`・`depends_on` が自分自身を指す、循環している。`depends_on` が祖先か子孫を指す、ほかの依存から導ける

見ないもの: 中身の正しさ。外の世界との食い違い（ブランチのコミット、PR の状態、worktree の実在。operation.md 3.1 でエージェントが見る）。「触られていない」（守るのは不整合であって放置ではない）。関連の文書の置き場所（タスクが `archive/` にあるのに `artifacts/` が外、など）は決まりではないので挙げない。

- `--fix` は、機械的に 1 通りに決まる直しだけ: 足りない frontmatter のキーを空で足す（`status`・`needs_human` も値は埋めない）、`status` と置き場所のずれを `archive/` との間の移動で直す、切れたリンクを同じ名前のファイルがツリーに 1 つだけあればそこへ付け替える（人が動かした後）。中身には触らない。直したファイルを出すので、エージェントがその変更だけをコミットする
- `desk rules` は、CLI が実装している形（種類ごとの置き場所、名前の形、frontmatter のキーと値、必須の節、予約名）を人が読める形で出す。正本はスキル。ツリーを読まないので、ツリーの外でも動く

## `desk archive`・`desk mv`

```
desk archive <パス>... [--status done | dropped] [--dry-run]
desk archive --project <名前> [--dry-run]
desk mv <元>... <先のディレクトリ> [--dry-run]
desk mv <元のファイル> <新しいパス.md> [--dry-run]   # 名前の変更
```

- `archive` は、指定したファイルをまとめて、同じ形で `archive/` に `git mv` する（`tasks/x.md` → `archive/tasks/x.md`、`artifacts/assets/a.png` → `archive/artifacts/assets/a.png`）。タスクを指定したら `status` を書く（`--status` があればその値、無ければ `done`。既に `done`・`dropped` ならそのまま）。移せるのはタスクと `artifacts/`（とその `assets/`）だけで、タスクに関係する文書を自動では動かさない。一緒に移すものはエージェントが選んで並べる（候補は `desk graph <タスク> --in --format paths`。tree.md 8 節）
- **動かしたあと、リンクを付け替える**: 動かしたファイルの中の相対パスと、ツリー中で動かしたファイルを指している相対パス（本文と frontmatter）。まとめて動かすので付け替えは 1 回。`--dry-run` は、何を動かし、どのファイルのどの行を書き換えるかだけ出す
- 既に `archive/` にあるものは移動を飛ばす。`--status` があって今の値と違えば、`status` だけ書き換える。戻すのは `desk mv`（同じ操作の逆）
- `--project` はプロジェクトごと `projects/<p>/` から `archive/projects/<p>/` へ（中同士のリンクは深さが同じなのでそのまま。中から外を指すリンクと、外から指しているリンクを付け替える）
- `mv` は `archive/` 以外への移動と名前の変更（ナレッジをルートへ、archive から戻す）。先が決まりの外の場所なら止まる
- コミットはしない。エージェントがその変更だけを先にコミットする
