//! desk のツリーの形の解釈。CLI（desk-cli）とアプリ（desk-app）が共有する。
//!
//! 読む側（走査・種類・frontmatter・リンク・見出しと節）はいつも使える。書く側は feature `write` の中に置き、
//! 読むだけのアプリはそれを有効にしない。形の正本はスキル desk（skill/SKILL.md）。

pub mod check;
pub mod doc;
#[cfg(feature = "write")]
pub mod edit;
pub mod frontmatter;
pub mod graph;
pub mod link;
pub mod path;
pub mod query;
pub mod quote;
pub mod read;
pub mod rules;
pub mod tree;
#[cfg(feature = "write")]
pub mod write;

pub use doc::{Doc, Heading};
pub use frontmatter::{Frontmatter, Value};
pub use link::{Link, LinkForm};
pub use path::{Info, classify};
pub use rules::Kind;
pub use tree::{Edge, Entry, Error, Tree, find_root};
