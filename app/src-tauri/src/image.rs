//! 本文の画像を webview に配る、独自の scheme（`desk://localhost/<パスを %xx にしたもの>`）。
//!
//! webview にはファイルを読む権限を渡さないので、ローカルの画像は Rust が配る。パスは、開いているツリーのルートからのパス
//! （外なら `../` 始まり）か、`/` 始まり・`~/` 始まり。ツリーの中か外か、索引にあるかは問わず、拡張子が画像のファイルだけを読む。
//! 画面の側で URL を作るのは `app/src/features/preview/resolve.ts` の `imageUrl`（パス全体を 1 つの段として %xx にする）。

use std::borrow::Cow;
use std::path::{Path, PathBuf};

use percent_encoding::percent_decode_str;
use tauri::http::{Method, Request, Response, StatusCode, header};
use tauri::{Manager, Runtime, UriSchemeContext, UriSchemeResponder};

use crate::desk::Desk;

/// 独自の scheme の名前。CSP の img-src にも書く（tauri.conf.json）。
pub const SCHEME: &str = "desk";

/// scheme への読み込みに答える。ファイルを読むので、別のスレッドで答える。
pub fn handle<R: Runtime>(
    ctx: UriSchemeContext<'_, R>,
    request: Request<Vec<u8>>,
    responder: UriSchemeResponder,
) {
    let app = ctx.app_handle().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let response = match request_path(&request) {
            Some(path) => match read(&app, &path) {
                Ok((bytes, content_type)) => ok(content_type, bytes),
                Err(e) => {
                    eprintln!("画像を配れない: {e}");
                    status(StatusCode::NOT_FOUND)
                }
            },
            None => status(StatusCode::BAD_REQUEST),
        };
        responder.respond(response);
    });
}

fn read<R: Runtime>(
    app: &tauri::AppHandle<R>,
    path: &str,
) -> Result<(Vec<u8>, &'static str), String> {
    let root = app.state::<Desk>().root().ok_or("ツリーを開いていない")?;
    let home = app.path().home_dir().map_err(|e| e.to_string())?;
    let (file, content_type) = image_file(&root, &home, path)?;
    let bytes = std::fs::read(&file).map_err(|e| format!("{} を読めない: {e}", file.display()))?;
    Ok((bytes, content_type))
}

/// 読み込みの URL が指すパス。GET でないものと、%xx を戻して UTF-8 にならないものは None。
fn request_path(request: &Request<Vec<u8>>) -> Option<String> {
    if request.method() != Method::GET {
        return None;
    }
    let path = request.uri().path().strip_prefix('/')?;
    percent_decode_str(path)
        .decode_utf8()
        .ok()
        .map(Cow::into_owned)
}

/// パス（ルートからのパス、`/` 始まり、`~/` 始まり）が指すディスクの上のファイルと、その種類。
/// 拡張子が画像でないものはエラー（読まない）。
fn image_file(root: &Path, home: &Path, path: &str) -> Result<(PathBuf, &'static str), String> {
    let content_type =
        content_type(path).ok_or_else(|| format!("{path} は画像のファイルではない"))?;
    let file = match path.strip_prefix("~/") {
        Some(rest) => home.join(rest),
        // `/` 始まりなら join はそのパスになる
        None => root.join(path),
    };
    Ok((file, content_type))
}

fn ok(content_type: &str, bytes: Vec<u8>) -> Response<Vec<u8>> {
    Response::builder()
        .header(header::CONTENT_TYPE, content_type)
        // 中身を推し量らせない（拡張子と違う種類として読ませない）。ページとして開かれても何も動かさない
        // （SVG の script など。webview の移動は navigation.rs が止めるが、念のため）。ファイルは変わるので覚えさせない
        .header(header::X_CONTENT_TYPE_OPTIONS, "nosniff")
        .header(header::CONTENT_SECURITY_POLICY, "sandbox")
        .header(header::CACHE_CONTROL, "no-store")
        .body(bytes)
        .unwrap()
}

fn status(code: StatusCode) -> Response<Vec<u8>> {
    Response::builder().status(code).body(Vec::new()).unwrap()
}

/// 拡張子から決める画像の種類。画像でないものは None。
fn content_type(path: &str) -> Option<&'static str> {
    let ext = path
        .rsplit_once('.')
        .map(|(_, ext)| ext.to_ascii_lowercase());
    Some(match ext.as_deref()? {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "avif" => "image/avif",
        "svg" => "image/svg+xml",
        "bmp" => "image/bmp",
        "ico" => "image/x-icon",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(method: Method, uri: &str) -> Request<Vec<u8>> {
        Request::builder()
            .method(method)
            .uri(uri)
            .body(Vec::new())
            .unwrap()
    }

    #[test]
    fn request_path_decodes_the_whole_path() {
        let got = |uri: &str| request_path(&request(Method::GET, uri));
        assert_eq!(
            got("desk://localhost/projects%2Fa%2Fassets%2F50%25%231%20%E8%A6%8B%E6%9C%AC.png")
                .as_deref(),
            Some("projects/a/assets/50%#1 見本.png")
        );
        assert_eq!(
            got("desk://localhost/..%2Fphotos%2Fa.png").as_deref(),
            Some("../photos/a.png")
        );
        assert_eq!(got("desk://localhost/a%2F%FF.png"), None);
        assert_eq!(
            request_path(&request(Method::POST, "desk://localhost/a.png")),
            None
        );
    }

    #[test]
    fn image_file_reads_images_anywhere() {
        let root = Path::new("/trees/desk");
        let home = Path::new("/Users/someone");
        let got = |path: &str| image_file(root, home, path).map(|(file, _)| file);
        // ツリーの中（assets/ の外・索引に無いものも）、ツリーの外、/ 始まり、~/ 始まり
        assert_eq!(
            got("projects/a/inbox/photo.png").unwrap(),
            Path::new("/trees/desk/projects/a/inbox/photo.png")
        );
        assert_eq!(
            got("../photos/a.JPG").unwrap(),
            Path::new("/trees/desk/../photos/a.JPG")
        );
        assert_eq!(
            got("/Volumes/x/a.svg").unwrap(),
            Path::new("/Volumes/x/a.svg")
        );
        assert_eq!(
            got("~/Pictures/b.webp").unwrap(),
            Path::new("/Users/someone/Pictures/b.webp")
        );
        // 画像でないものは読まない
        for path in [
            "README.md",
            "../.ssh/id_ed25519",
            "/etc/hosts",
            "notes.html",
            "~/.zshrc",
            "assets/Makefile",
        ] {
            let err = got(path).unwrap_err();
            assert!(err.contains("画像のファイルではない"), "{path}: {err}");
        }
    }

    #[test]
    fn content_type_by_extension() {
        assert_eq!(content_type("assets/a.PNG"), Some("image/png"));
        assert_eq!(content_type("assets/a.jpeg"), Some("image/jpeg"));
        assert_eq!(content_type("assets/a.svg"), Some("image/svg+xml"));
        assert_eq!(content_type("assets/notes.html"), None);
        assert_eq!(content_type("assets/Makefile"), None);
    }
}
