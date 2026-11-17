// 外の URL として既定のアプリで開く scheme。本文・図の中・右の列で同じ並びを使う

/**
 * 本文の消毒（rehype-sanitize）の既定（http・https・irc・ircs・mailto・xmpp）と、
 * 図（pele）の既定（http・https・mailto・tel）を合わせたもの。
 * 開くときは Rust の open_url（external.rs の SCHEMES）がもう一度確かめるので、並びを変えるときは両方を変える
 */
export const urlSchemes = ["http", "https", "mailto", "tel", "irc", "ircs", "xmpp"]

const scheme = urlSchemes.join("|")
const startsWithScheme = new RegExp(`^(${scheme}):`, "i")
const wholeUrl = new RegExp(`^(${scheme}):\\S+$`, "i")

/** 書いてある先（本文や図の中のリンクの href）が、外の URL として開く scheme で始まるか */
export const hasUrlScheme = (href: string) => startsWithScheme.test(href.trim())

/** 値（右の列の frontmatter の値）が、まるごと外の URL 1 つか（空白を含まない） */
export const isUrl = (value: string) => wholeUrl.test(value)
