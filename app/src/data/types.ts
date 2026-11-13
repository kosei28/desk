// Rust のコマンドが返す形（desk-core の Doc などを serialize したものと、desk-app の Index）。
// Rust の型から生成せず手で写しているので、Rust の側の形を変えたらここも合わせる。キーは JSON のまま（snake_case）。

export type Kind =
  | "task"
  | "artifact"
  | "inbox"
  | "library"
  | "principle"
  | "knowledge"
  | "material"
  | "wiki"
  | "project"
  | "readme"
  | "asset"

/** frontmatter の値。型は解かず、書いてあるとおりの文字列か文字列の配列。書いていなければ null */
export type FrontmatterValue = string | string[] | null

/** frontmatter のキーと値 1 組。書いてあるとおりの順に並ぶ */
export type FrontmatterField = { key: string; value: FrontmatterValue }

export type Heading = {
  level: number
  text: string
  /** ファイルの行（0 始まり） */
  line: number
  /** 節の終わりの行 */
  end: number
}

export type LinkForm = "relative" | "absolute" | "home" | "wiki" | "external" | "anchor"

export type Link = {
  /** "body" か frontmatter のキー */
  key: string
  text: string
  /** 書いてあるとおりの先 */
  raw: string
  line: number
  start: number | null
  form: LinkForm
  /** relative ならルートからのパス（外なら ../ 始まり）、absolute・home なら絶対パス、wiki なら名前 */
  target: string
  anchor: string
}

/** ツリーの中のファイル 1 つ。.md でないもの（is_md が false）は、パスで決まることだけを持つ */
export type Doc = {
  /** ルートからのパス（/ 区切り） */
  path: string
  kind: Kind | null
  /** ルートのものは空 */
  project: string
  archived: boolean
  project_archived: boolean
  asset: boolean
  /** 決まりの外の場所なら、その説明 */
  problem: string | null
  is_md: boolean
  frontmatter: FrontmatterField[] | null
  frontmatter_error: string | null
  title: string
  description: string
  /** 「いま」の 1 行目 */
  now: string
  status: string
  needs_human: boolean
  headings: Heading[]
  links: Link[]
}

/** 開いているツリーの索引 */
export type Index = {
  /** ルートの絶対パス */
  root: string
  /** ルートの下の全ファイル（パス順。. で始まる名前は入らない） */
  docs: Doc[]
}
