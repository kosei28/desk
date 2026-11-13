# desk

あらゆる作業ドキュメントを 1 つのツリーで一元管理する仕組み。ドキュメントのツリーとは別に、ここにはスキル・CLI・設計を置く。

| 場所 | 中身 |
| --- | --- |
| `skill/` | スキル `desk` の本体。ツリーの決まり（形・書き方・手順）の正本 |
| `core/` | ライブラリ `desk-core`。ツリーの形の解釈（読む・引く・確かめる・書く）。CLI（と、これから作る desk app）が使う |
| `cli/` | CLI `desk`。スキルと同じ形を実装した補助ツール（作る・引く・読む・確かめる・動かす） |
| `docs/` | 設計文書。`tree.md`（ツリーの構成と書き方）、`operation.md`（スキル・CLI・git で壊さずに使い続ける運用の設計）、`cli.md`（CLI のコマンド） |

## 入れ方

このリポジトリのディレクトリで:

```sh
# CLI（~/.cargo/bin に入る。ほかの場所なら --root <ディレクトリ> で <ディレクトリ>/bin に）
cargo install --locked --path cli

# スキル（Claude Code のスキルの場所にリンクする）
ln -s "$PWD/skill" ~/.claude/skills/desk

# ツリー（置きたい場所に作る）
desk init <ツリーのパス>
```

`desk` はツリーの中（ルートかその下のディレクトリ）で実行する。ルートは `.desk` のあるディレクトリを上にたどって見つける。ツリーの外ではエラーで止まる。

## 開発

```sh
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

## ライセンス

[MIT](LICENSE)
