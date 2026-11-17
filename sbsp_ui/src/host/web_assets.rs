// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! Serves the web build to browsers that open the server's address, in
//! place of the Tauri build's `ServeDir` over a resource folder.
//!
//! The files are embedded in the host binary from `web_dist/` (the web
//! build's output; see `build.rs`). The lookup, content types, caching and
//! the single-page fallback are in plain functions over a [`Source`], so
//! they are tested without embedding anything; the axum handler at the end
//! only turns the result into a response.
//!
//! - A path that is not a file and has no extension is a page of the app,
//!   so it gets `index.html` (the same for `/`). A missing file with an
//!   extension is a 404.
//! - `<file>.br` / `<file>.gz` next to a file are served instead of it when
//!   the browser accepts that encoding (the web build can be compressed
//!   ahead of time, which matters for the wasm).
//! - Every response carries an `ETag` and `Cache-Control: no-cache`, so a
//!   browser revalidates and gets a 304 until the host is updated; file
//!   names are not assumed to be hashed.

use std::borrow::Cow;

use axum::{
    Router,
    body::Body,
    http::{HeaderMap, Method, Response, StatusCode, Uri, header},
};

#[derive(rust_embed::RustEmbed)]
#[folder = "$CARGO_MANIFEST_DIR/web_dist/"]
struct WebDist;

/// One embedded file.
pub struct Asset {
    pub data: Cow<'static, [u8]>,
    /// A hex digest of `data`, for the `ETag`.
    pub hash: String,
}

/// Where files come from (the embedded folder, or a fake in the tests).
pub trait Source {
    fn get(&self, path: &str) -> Option<Asset>;
}

struct Embedded;

impl Source for Embedded {
    fn get(&self, path: &str) -> Option<Asset> {
        let file = WebDist::get(path)?;
        let hash = file
            .metadata
            .sha256_hash()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        Some(Asset {
            data: file.data,
            hash,
        })
    }
}

/// What to send for a request, before it becomes an HTTP response.
pub struct Found {
    pub asset: Asset,
    pub content_type: &'static str,
    /// The `Content-Encoding`, when a pre-compressed file was chosen.
    pub encoding: Option<&'static str>,
}

/// The file a request path names, with the single-page fallback applied.
/// `None` is a 404.
fn locate(source: &dyn Source, request_path: &str) -> Option<(String, Asset)> {
    let path = request_path.trim_start_matches('/');
    // Nothing embedded has these, and nothing here should be asked to
    // look for them.
    if path
        .split('/')
        .any(|part| part == ".." || part.contains('\\'))
    {
        return None;
    }
    let path = if path.is_empty() { "index.html" } else { path };
    if let Some(asset) = source.get(path) {
        return Some((path.to_string(), asset));
    }
    let last = path.rsplit('/').next().unwrap_or(path);
    if last.contains('.') {
        return None;
    }
    source
        .get("index.html")
        .map(|asset| ("index.html".to_string(), asset))
}

/// What the browser's `Accept-Encoding` allows among the encodings that can
/// be pre-compressed here.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Accepted {
    pub br: bool,
    pub gzip: bool,
}

impl Accepted {
    /// Reads an `Accept-Encoding` value. Coding names are case-insensitive
    /// and a q-value of 0 refuses a coding (`br;q=0`); a malformed q-value
    /// counts as a refusal, since the plain file is always a valid answer.
    /// `*` stands for every coding the header does not name.
    pub fn parse(header: &str) -> Self {
        let (mut br, mut gzip, mut any) = (None, None, None);
        for item in header.split(',') {
            let mut parts = item.split(';');
            let coding = parts.next().unwrap_or("").trim().to_ascii_lowercase();
            // No q-value: accepted. One that does not parse: refused.
            let allowed = parts
                .find_map(|param| {
                    let (key, value) = param.trim().split_once('=')?;
                    key.trim()
                        .eq_ignore_ascii_case("q")
                        .then(|| value.trim().parse::<f32>().is_ok_and(|q| q > 0.0))
                })
                .unwrap_or(true);
            match coding.as_str() {
                "br" => br = Some(allowed),
                "gzip" | "x-gzip" => gzip = Some(allowed),
                "*" => any = Some(allowed),
                _ => {}
            }
        }
        Self {
            br: br.or(any).unwrap_or(false),
            gzip: gzip.or(any).unwrap_or(false),
        }
    }
}

/// Picks the file and, when the browser accepts it and it exists, the
/// pre-compressed copy of it.
pub fn find(source: &dyn Source, request_path: &str, accepted: Accepted) -> Option<Found> {
    let (path, plain) = locate(source, request_path)?;
    let content_type = content_type(&path);
    for (allowed, encoding, extension) in [(accepted.br, "br", "br"), (accepted.gzip, "gzip", "gz")]
    {
        if allowed && let Some(asset) = source.get(&format!("{path}.{extension}")) {
            return Some(Found {
                asset,
                content_type,
                encoding: Some(encoding),
            });
        }
    }
    Some(Found {
        asset: plain,
        content_type,
        encoding: None,
    })
}

/// The content type for a file name. `wasm` matters: browsers only
/// compile it while streaming when it is served as `application/wasm`.
fn content_type(path: &str) -> &'static str {
    let extension = path.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    match extension.as_str() {
        "html" | "htm" => "text/html; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" | "map" => "application/json",
        "wasm" => "application/wasm",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "ico" => "image/x-icon",
        "ttf" => "font/ttf",
        "otf" => "font/otf",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "txt" | "md" => "text/plain; charset=utf-8",
        "webmanifest" => "application/manifest+json",
        _ => "application/octet-stream",
    }
}

fn respond(source: &dyn Source, method: &Method, uri: &Uri, headers: &HeaderMap) -> Response<Body> {
    let status = |code: StatusCode| {
        Response::builder()
            .status(code)
            .body(Body::empty())
            .expect("a status-only response")
    };
    if method != Method::GET && method != Method::HEAD {
        return status(StatusCode::METHOD_NOT_ALLOWED);
    }
    let accepted = Accepted::parse(
        headers
            .get(header::ACCEPT_ENCODING)
            .and_then(|value| value.to_str().ok())
            .unwrap_or(""),
    );
    let Some(found) = find(source, uri.path(), accepted) else {
        return status(StatusCode::NOT_FOUND);
    };

    let etag = format!("\"{}\"", found.asset.hash);
    let mut builder = Response::builder()
        .header(header::ETAG, &etag)
        .header(header::CACHE_CONTROL, "no-cache")
        .header(header::VARY, "Accept-Encoding");

    let unchanged = headers
        .get(header::IF_NONE_MATCH)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.split(',').any(|item| item.trim() == etag));
    if unchanged {
        return builder
            .status(StatusCode::NOT_MODIFIED)
            .body(Body::empty())
            .expect("a 304 response");
    }

    builder = builder
        .header(header::CONTENT_TYPE, found.content_type)
        .header(header::CONTENT_LENGTH, found.asset.data.len());
    if let Some(encoding) = found.encoding {
        builder = builder.header(header::CONTENT_ENCODING, encoding);
    }
    let body = if method == Method::HEAD {
        Body::empty()
    } else {
        match found.asset.data {
            Cow::Borrowed(bytes) => Body::from(bytes),
            Cow::Owned(bytes) => Body::from(bytes),
        }
    };
    builder.body(body).expect("a file response")
}

async fn handler(method: Method, uri: Uri, headers: HeaderMap) -> Response<Body> {
    respond(&Embedded, &method, &uri, &headers)
}

/// Adds the web build to the API server's router, behind its own routes
/// (the websocket), as the fallback.
pub fn extend(router: Router) -> Router {
    router.fallback(handler)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    struct Fake(HashMap<&'static str, &'static [u8]>);

    impl Source for Fake {
        fn get(&self, path: &str) -> Option<Asset> {
            self.0.get(path).map(|data| Asset {
                data: Cow::Borrowed(*data),
                hash: format!("hash-of-{}", String::from_utf8_lossy(data)),
            })
        }
    }

    fn site() -> Fake {
        Fake(HashMap::from([
            ("index.html", &b"index"[..]),
            ("app.js", &b"js"[..]),
            ("app.wasm", &b"wasm"[..]),
            ("app.wasm.br", &b"wasm-br"[..]),
            ("app.wasm.gz", &b"wasm-gz"[..]),
            ("fonts/ui.ttf", &b"font"[..]),
        ]))
    }

    fn body(found: &Found) -> &[u8] {
        &found.asset.data
    }

    #[test]
    fn files_are_found_by_path_and_the_root_is_the_index() {
        let site = site();
        assert_eq!(
            body(&find(&site, "/app.js", Accepted::default()).unwrap()),
            b"js"
        );
        assert_eq!(
            body(&find(&site, "/fonts/ui.ttf", Accepted::default()).unwrap()),
            b"font"
        );
        assert_eq!(
            body(&find(&site, "/", Accepted::default()).unwrap()),
            b"index"
        );
    }

    #[test]
    fn pages_fall_back_to_the_index_but_missing_files_do_not() {
        let site = site();
        assert_eq!(
            body(&find(&site, "/remote/connect", Accepted::default()).unwrap()),
            b"index"
        );
        assert!(find(&site, "/missing.js", Accepted::default()).is_none());
        assert!(find(&site, "/fonts/missing.ttf", Accepted::default()).is_none());
        assert!(find(&Fake(HashMap::new()), "/", Accepted::default()).is_none());
    }

    #[test]
    fn climbing_out_of_the_folder_is_not_answered() {
        let site = site();
        assert!(find(&site, "/../secret", Accepted::default()).is_none());
        assert!(find(&site, "/fonts/../../secret", Accepted::default()).is_none());
    }

    #[test]
    fn a_compressed_copy_is_used_only_when_accepted() {
        let site = site();
        let plain = find(&site, "/app.wasm", Accepted::default()).unwrap();
        assert_eq!((body(&plain), plain.encoding), (&b"wasm"[..], None));

        let brotli = find(&site, "/app.wasm", Accepted::parse("gzip, deflate, br")).unwrap();
        assert_eq!(
            (body(&brotli), brotli.encoding),
            (&b"wasm-br"[..], Some("br"))
        );
        // The content type is the original file's, not the compressed one's.
        assert_eq!(brotli.content_type, "application/wasm");

        let gzip = find(&site, "/app.wasm", Accepted::parse("gzip;q=0.8")).unwrap();
        assert_eq!(
            (body(&gzip), gzip.encoding),
            (&b"wasm-gz"[..], Some("gzip"))
        );

        // No compressed copy exists for this one.
        assert_eq!(
            find(&site, "/app.js", Accepted::parse("br, gzip"))
                .unwrap()
                .encoding,
            None
        );
    }

    #[test]
    fn a_refused_encoding_is_not_used() {
        let site = site();
        let gzip = find(&site, "/app.wasm", Accepted::parse("br;q=0, gzip")).unwrap();
        assert_eq!(gzip.encoding, Some("gzip"));
        let plain = find(&site, "/app.wasm", Accepted::parse("br;q=0")).unwrap();
        assert_eq!((body(&plain), plain.encoding), (&b"wasm"[..], None));
    }

    #[test]
    fn accept_encoding_is_read_into_what_is_allowed() {
        let both = Accepted {
            br: true,
            gzip: true,
        };
        let only_gzip = Accepted {
            br: false,
            gzip: true,
        };
        let none = Accepted::default();

        assert_eq!(Accepted::parse(""), none);
        assert_eq!(Accepted::parse("deflate, identity"), none);
        assert_eq!(Accepted::parse("br, gzip"), both);
        assert_eq!(Accepted::parse("BR, GZip"), both);
        assert_eq!(Accepted::parse("x-gzip"), only_gzip);
        assert_eq!(Accepted::parse("br;q=0, gzip"), only_gzip);
        assert_eq!(Accepted::parse("br; q=0.0, gzip;q=1"), only_gzip);
        assert_eq!(Accepted::parse("gzip;q=0.5, br;q=0.8"), both);
        // A q-value that does not parse refuses the coding.
        assert_eq!(Accepted::parse("br;q=abc, gzip"), only_gzip);
        // `*` covers what is not named; what is named wins over it.
        assert_eq!(Accepted::parse("*"), both);
        assert_eq!(Accepted::parse("*, br;q=0"), only_gzip);
        assert_eq!(Accepted::parse("*;q=0, gzip"), only_gzip);
    }

    #[test]
    fn a_request_header_picks_the_encoding() {
        let response = request(
            Method::GET,
            "/app.wasm",
            &[(header::ACCEPT_ENCODING, "br;q=0, gzip")],
        );
        assert_eq!(response.headers()[header::CONTENT_ENCODING], "gzip");
        assert_eq!(response.headers()[header::VARY], "Accept-Encoding");

        let refused = request(
            Method::GET,
            "/app.wasm",
            &[(header::ACCEPT_ENCODING, "br;q=0, gzip;q=0")],
        );
        assert!(refused.headers().get(header::CONTENT_ENCODING).is_none());
    }

    #[test]
    fn content_types_cover_what_the_web_build_has() {
        assert_eq!(content_type("app.wasm"), "application/wasm");
        assert_eq!(content_type("index.html"), "text/html; charset=utf-8");
        assert_eq!(content_type("a/b.JS"), "text/javascript; charset=utf-8");
        assert_eq!(content_type("ui.ttf"), "font/ttf");
        assert_eq!(content_type("unknown.bin"), "application/octet-stream");
        assert_eq!(content_type("no-extension"), "application/octet-stream");
    }

    fn request(
        method: Method,
        path: &str,
        headers: &[(header::HeaderName, &str)],
    ) -> Response<Body> {
        let mut map = HeaderMap::new();
        for (name, value) in headers {
            map.insert(name.clone(), value.parse().unwrap());
        }
        respond(&site(), &method, &path.parse().unwrap(), &map)
    }

    #[test]
    fn responses_carry_the_validators_and_answer_not_modified() {
        let response = request(Method::GET, "/app.js", &[]);
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers()[header::CONTENT_TYPE],
            "text/javascript; charset=utf-8"
        );
        assert_eq!(response.headers()[header::CACHE_CONTROL], "no-cache");
        let etag = response.headers()[header::ETAG]
            .to_str()
            .unwrap()
            .to_string();

        let again = request(Method::GET, "/app.js", &[(header::IF_NONE_MATCH, &etag)]);
        assert_eq!(again.status(), StatusCode::NOT_MODIFIED);
    }

    #[test]
    fn only_get_and_head_are_answered_and_head_has_no_body() {
        assert_eq!(
            request(Method::POST, "/app.js", &[]).status(),
            StatusCode::METHOD_NOT_ALLOWED
        );
        assert_eq!(
            request(Method::GET, "/missing.js", &[]).status(),
            StatusCode::NOT_FOUND
        );
        let head = request(Method::HEAD, "/app.js", &[]);
        assert_eq!(head.status(), StatusCode::OK);
        assert_eq!(head.headers()[header::CONTENT_LENGTH], "2");
    }
}
