// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! The web build's entry point, and what it reads from the page's address:
//! `?address=host:port` connects to that host as the page opens (the
//! password, if it has one, is the `#fragment`, which the browser never
//! sends to a server); without it the connect screen starts out with the
//! address the page itself came from, which is the host that serves it.

pub fn run() -> iced::Result {
    crate::app::run("web")
}

/// The host and password the address asks to connect to, if it names one.
pub fn parse_startup(search: &str, hash: &str) -> Option<(String, Option<String>)> {
    let address = search
        .trim_start_matches('?')
        .split('&')
        .filter_map(|pair| pair.split_once('='))
        .find(|(key, _)| *key == "address")
        .map(|(_, value)| percent_decode(value))
        .map(|address| address.trim().to_owned())
        .filter(|address| !address.is_empty())?;

    let password = hash.trim_start_matches('#');
    let password = password.strip_prefix("password=").unwrap_or(password);
    let password = percent_decode(password);
    Some((address, (!password.is_empty()).then_some(password)))
}

/// `%40` is `@`; anything that is not a valid escape stays as written.
fn percent_decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let escaped = (bytes[i] == b'%')
            .then(|| bytes.get(i + 1..i + 3))
            .flatten()
            .and_then(|hex| std::str::from_utf8(hex).ok())
            .and_then(|hex| u8::from_str_radix(hex, 16).ok());
        match escaped {
            Some(byte) => {
                out.push(byte);
                i += 3;
            }
            None => {
                out.push(bytes[i]);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// The fonts of the page: cut-down copies of the UI fonts that the host
/// serves next to the page (`scripts/subset_fonts.py`).
#[cfg(target_arch = "wasm32")]
pub async fn fetch_fonts() -> Result<std::sync::Arc<(Vec<u8>, Vec<u8>)>, String> {
    let ui = fetch("fonts/ui.ttf").await?;
    let mono = fetch("fonts/mono.ttf").await?;
    Ok(std::sync::Arc::new((ui, mono)))
}

#[cfg(target_arch = "wasm32")]
async fn fetch(url: &str) -> Result<Vec<u8>, String> {
    let response = gloo_net::http::Request::get(url)
        .send()
        .await
        .map_err(|e| format!("{url}: {e}"))?;
    if !response.ok() {
        return Err(format!("{url}: HTTP {}", response.status()));
    }
    response.binary().await.map_err(|e| format!("{url}: {e}"))
}

/// The connect screen to start with, and the connection to make at once
/// (when the page's address asks for one).
#[cfg(all(client, target_arch = "wasm32"))]
pub fn startup_connection() -> (
    crate::features::connect::Screen,
    Option<crate::features::connect::Request>,
) {
    use crate::features::connect::{Request, Screen};

    let mut screen = Screen::default();
    let Some(location) = web_sys::window().map(|window| window.location()) else {
        return (screen, None);
    };
    let search = location.search().unwrap_or_default();
    let hash = location.hash().unwrap_or_default();

    let request = parse_startup(&search, &hash).map(|(address, password)| Request {
        address,
        password,
    });
    match &request {
        Some(request) => match request.address.rsplit_once(':') {
            Some((host, port)) => {
                screen.host = host.to_owned();
                screen.port = port.to_owned();
            }
            None => screen.host = request.address.clone(),
        },
        None => {
            screen.host = location.hostname().unwrap_or_default();
            if let Ok(port) = location.port()
                && !port.is_empty()
            {
                screen.port = port;
            }
        }
    }
    (screen, request)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_address_comes_from_the_query_and_the_password_from_the_fragment() {
        assert_eq!(
            parse_startup("?address=192.168.1.5:5800", "#secret"),
            Some(("192.168.1.5:5800".to_owned(), Some("secret".to_owned())))
        );
        assert_eq!(
            parse_startup("?address=host:5800", ""),
            Some(("host:5800".to_owned(), None))
        );
    }

    #[test]
    fn other_parameters_are_skipped_and_the_password_may_be_named() {
        assert_eq!(
            parse_startup("?lang=ja&address=host:1&x=2", "#password=pw"),
            Some(("host:1".to_owned(), Some("pw".to_owned())))
        );
    }

    #[test]
    fn escapes_are_decoded_and_bad_ones_are_left_alone() {
        assert_eq!(
            parse_startup("?address=h%3A1", "#p%40ss%20word"),
            Some(("h:1".to_owned(), Some("p@ss word".to_owned())))
        );
        assert_eq!(percent_decode("100%"), "100%");
        assert_eq!(percent_decode("%zz%4"), "%zz%4");
        assert_eq!(percent_decode("%E3%81%82"), "あ");
    }

    #[test]
    fn without_an_address_there_is_nothing_to_connect_to() {
        assert_eq!(parse_startup("", "#pw"), None);
        assert_eq!(parse_startup("?address=", ""), None);
        assert_eq!(parse_startup("?other=1", ""), None);
    }
}
