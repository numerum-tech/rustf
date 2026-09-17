//! WebSocket opening-handshake validation (RFC 6455 §4.2).
//!
//! Everything here runs on the ordinary HTTP request, before any upgrade
//! happens, so a rejected handshake produces a normal HTTP response that a
//! browser's `WebSocket` constructor surfaces as a failed connection and a
//! `curl` user can read.

use crate::http::{Request, Response};
use hyper::StatusCode;
use tokio_tungstenite::tungstenite::handshake::derive_accept_key;

/// What the handshake negotiated, once it has been accepted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Accepted {
    /// The `Sec-WebSocket-Accept` value derived from the client's key.
    pub(crate) accept_key: String,
    /// The subprotocol to echo back, if client and server agreed on one.
    pub(crate) subprotocol: Option<String>,
}

/// Server-side policy applied to an incoming handshake.
#[derive(Debug, Clone, Default)]
pub(crate) struct HandshakeOptions {
    /// Origins allowed in addition to the request's own origin.
    ///
    /// Empty means same-origin only. A single `"*"` entry disables the check
    /// entirely — which re-opens Cross-Site WebSocket Hijacking, so it is
    /// only ever appropriate for an endpoint that carries no ambient
    /// authority.
    pub(crate) allowed_origins: Vec<String>,
    /// Subprotocols this server understands, most preferred first.
    pub(crate) accept_subprotocols: Vec<String>,
    /// Refuse a handshake that carries no `Origin` at all.
    ///
    /// An absent `Origin` normally means a non-browser client, which attaches
    /// no cookies of its own — but a reverse proxy can strip the header while
    /// forwarding `Cookie`, and then "same-origin only" would be fooled by its
    /// absence. Off by default, because turning it on breaks every legitimate
    /// non-browser client; turn it on for an endpoint that is only ever opened
    /// by a browser.
    pub(crate) require_origin: bool,
}

/// Validate an opening handshake.
///
/// `Ok` carries what was negotiated; `Err` carries the HTTP response to send
/// instead of upgrading. Nothing here mutates the request.
pub(crate) fn validate(
    request: &Request,
    options: &HandshakeOptions,
) -> std::result::Result<Accepted, Response> {
    // 1. Is an upgrade even being attempted? A plain GET on a socket route is
    //    a client that does not know what this endpoint is; RFC 7231 §6.5.15
    //    says 426 with the protocol it should use.
    if !request.is_websocket_upgrade() {
        return Err(upgrade_required(
            "This endpoint speaks WebSocket. Connect with ws:// or wss://.",
        ));
    }

    // 2. Duplicated singleton fields. `Request.headers` is a map, so a second
    //    `Origin` or `Sec-WebSocket-Key` would silently overwrite the first and
    //    the check above would run against whichever value survived. RFC 6455
    //    §4.2.1 requires exactly one of each, and accepting a duplicate is how
    //    a proxy and a backend end up disagreeing about who is connecting.
    if request.has_duplicate_handshake_header() {
        return Err(bad_request(
            "A WebSocket handshake must carry exactly one Origin, \
             Sec-WebSocket-Key and Sec-WebSocket-Version header.",
        ));
    }

    // 3. Version. RFC 6455 §4.4: an unsupported version is answered with 426
    //    plus the versions the server does support, so the client can retry.
    match request.headers.get("sec-websocket-version") {
        Some(version) if version.trim() == "13" => {}
        Some(version) => {
            return Err(upgrade_required(&format!(
                "Unsupported WebSocket version `{}`; this server speaks 13.",
                version.trim()
            )));
        }
        None => {
            return Err(bad_request("Missing `Sec-WebSocket-Version` header."));
        }
    }

    // 4. Key. Must be the base64 encoding of 16 random bytes; the response's
    //    `Sec-WebSocket-Accept` is derived from it, and a client that gets
    //    that wrong must have its connection refused rather than silently
    //    accepted.
    let key = match request.headers.get("sec-websocket-key") {
        Some(key) if is_valid_key(key.trim()) => key.trim().to_string(),
        Some(_) => {
            return Err(bad_request(
                "`Sec-WebSocket-Key` must be 16 bytes, base64-encoded.",
            ));
        }
        None => {
            return Err(bad_request("Missing `Sec-WebSocket-Key` header."));
        }
    };

    // 5. Origin. The handshake is a plain GET, so the browser attaches cookies
    //    to it and CSRF tokens do not apply — the Origin check is the whole
    //    defence against Cross-Site WebSocket Hijacking.
    if !origin_allowed(request, options) {
        return Err(Response::forbidden(Some(
            "WebSocket connections from this origin are not allowed.",
        )));
    }

    Ok(Accepted {
        accept_key: derive_accept_key(key.as_bytes()),
        subprotocol: negotiate_subprotocol(request, &options.accept_subprotocols),
    })
}

/// Build the `101 Switching Protocols` response that completes the handshake.
pub(crate) fn accept_response(accepted: &Accepted) -> Response {
    let mut response = Response::new(StatusCode::SWITCHING_PROTOCOLS)
        .with_header("Upgrade", "websocket")
        .with_header("Connection", "Upgrade")
        .with_header("Sec-WebSocket-Accept", &accepted.accept_key);

    if let Some(subprotocol) = &accepted.subprotocol {
        response = response.with_header("Sec-WebSocket-Protocol", subprotocol);
    }

    response
}

/// `426 Upgrade Required`, advertising WebSocket 13.
pub(crate) fn upgrade_required(message: &str) -> Response {
    Response::new(StatusCode::UPGRADE_REQUIRED)
        .with_header("Upgrade", "websocket")
        .with_header("Connection", "Upgrade")
        .with_header("Sec-WebSocket-Version", "13")
        .with_header("Content-Type", "text/plain; charset=utf-8")
        .with_body(message.as_bytes().to_vec())
}

fn bad_request(message: &str) -> Response {
    Response::new(StatusCode::BAD_REQUEST)
        .with_header("Content-Type", "text/plain; charset=utf-8")
        .with_body(message.as_bytes().to_vec())
}

/// Is this a well-formed `Sec-WebSocket-Key`?
///
/// RFC 6455 §4.1: base64 of exactly 16 bytes, which is always 24 characters
/// ending in `==`. Checking the decoded length rather than just the string
/// length rejects padded junk that happens to be the right size.
fn is_valid_key(key: &str) -> bool {
    use base64::Engine as _;

    base64::engine::general_purpose::STANDARD
        .decode(key)
        .map(|decoded| decoded.len() == 16)
        .unwrap_or(false)
}

/// Pick a subprotocol both sides understand.
///
/// Server preference wins: the first entry of `accepted` that the client also
/// offered is chosen. When the server declares none, nothing is echoed, which
/// RFC 6455 §4.2.2 allows and every browser treats as "no subprotocol".
///
/// Comparison is **case-sensitive**. Subprotocol names are RFC 7230 tokens and
/// the server must echo one of the exact strings the client sent; answering
/// `chat` to a client that offered `Chat` is a value it never proposed, and a
/// conforming client is entitled to fail the connection over it.
fn negotiate_subprotocol(request: &Request, accepted: &[String]) -> Option<String> {
    if accepted.is_empty() {
        return None;
    }

    let requested = request.headers.get("sec-websocket-protocol")?;
    let requested: Vec<&str> = requested.split(',').map(str::trim).collect();

    accepted
        .iter()
        .find(|candidate| requested.iter().any(|offer| offer == &candidate.as_str()))
        .cloned()
}

/// Is the handshake's `Origin` acceptable?
///
/// Order matters. A request with no `Origin` is not a browser request — it
/// carries no ambient credentials, so there is nothing to hijack and it is
/// allowed. `null` is *not* that case: sandboxed iframes and `file://` pages
/// send it, and treating it as same-origin would hand them the session.
fn origin_allowed(request: &Request, options: &HandshakeOptions) -> bool {
    let allowed = &options.allowed_origins;

    let origin = match request.headers.get("origin") {
        Some(origin) => origin.trim(),
        // Usually a non-browser client (curl, a Rust client, a service), which
        // attaches no cookies of its own — but see `require_origin`, for
        // deployments where a proxy may have stripped the header.
        None => return !options.require_origin,
    };

    if allowed.iter().any(|entry| entry == "*") {
        return true;
    }

    if allowed
        .iter()
        .any(|entry| entry.trim().eq_ignore_ascii_case(origin))
    {
        return true;
    }

    // An explicit allowlist extends the default rather than replacing it:
    // otherwise naming one partner origin would lock out your own pages.
    same_origin(request, origin)
}

/// Does `origin` name the same scheme/host/port the request was sent to?
fn same_origin(request: &Request, origin: &str) -> bool {
    // `null` is an opaque origin. It matches nothing.
    if origin.eq_ignore_ascii_case("null") {
        return false;
    }

    let host_header = match request.headers.get("host") {
        Some(host) => host.trim(),
        // Without a Host header there is nothing to compare against, and
        // HTTP/1.1 requires one. Refuse rather than guess.
        None => return false,
    };

    let (scheme, authority) = match origin.split_once("://") {
        Some((scheme, rest)) => (scheme, rest),
        None => return false,
    };

    // RFC 6454 serialises an origin as scheme://host[:port] with no path, but
    // be defensive: a trailing slash must not become part of the host.
    let authority = authority.split('/').next().unwrap_or("");

    let expected_scheme = if request.is_secure() { "https" } else { "http" };
    if !scheme.eq_ignore_ascii_case(expected_scheme) {
        return false;
    }

    // Two unparseable authorities must not compare equal — `None == None`
    // would quietly declare a malformed Origin same-origin.
    match (
        normalize_authority(authority, scheme),
        normalize_authority(host_header, expected_scheme),
    ) {
        (Some(origin), Some(host)) => origin == host,
        _ => false,
    }
}

/// Lowercase the host and make the port explicit, so `example.com` and
/// `example.com:80` compare equal over http.
fn normalize_authority(authority: &str, scheme: &str) -> Option<(String, u16)> {
    let default_port = if scheme.eq_ignore_ascii_case("https") {
        443
    } else {
        80
    };

    let (host, port) = if let Some(rest) = authority.strip_prefix('[') {
        // IPv6 literal: `[::1]` or `[::1]:8000`.
        let (inner, tail) = rest.split_once(']')?;
        let port = match tail {
            "" => default_port,
            tail => tail.strip_prefix(':')?.parse().ok()?,
        };
        (format!("[{}]", inner.to_ascii_lowercase()), port)
    } else {
        match authority.rsplit_once(':') {
            Some((host, port)) => (host.to_ascii_lowercase(), port.parse().ok()?),
            None => (authority.to_ascii_lowercase(), default_port),
        }
    };

    if host.is_empty() {
        return None;
    }

    Some((host, port))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A well-formed key: base64 of 16 bytes.
    const KEY: &str = "dGhlIHNhbXBsZSBub25jZQ==";
    /// The `Sec-WebSocket-Accept` RFC 6455 §1.3 derives from `KEY`.
    const ACCEPT: &str = "s3pPLMBiTxaQ9kYGzzhZRbK+xOo=";

    fn handshake() -> Request {
        let mut request = Request::new("GET", "/chat", "HTTP/1.1");
        request
            .headers
            .insert("upgrade".into(), "websocket".into());
        request
            .headers
            .insert("connection".into(), "Upgrade".into());
        request
            .headers
            .insert("sec-websocket-version".into(), "13".into());
        request
            .headers
            .insert("sec-websocket-key".into(), KEY.into());
        request
            .headers
            .insert("host".into(), "example.com".into());
        request
    }

    fn header(response: &Response, name: &str) -> Option<String> {
        response
            .headers
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.clone())
    }

    #[test]
    fn accepts_a_valid_handshake_and_derives_the_rfc_accept_key() {
        let accepted = validate(&handshake(), &HandshakeOptions::default()).unwrap();
        assert_eq!(accepted.accept_key, ACCEPT);
        assert_eq!(accepted.subprotocol, None);
    }

    #[test]
    fn accept_response_is_a_101_with_the_upgrade_headers() {
        let accepted = validate(&handshake(), &HandshakeOptions::default()).unwrap();
        let response = accept_response(&accepted);

        assert_eq!(response.status, StatusCode::SWITCHING_PROTOCOLS);
        assert_eq!(header(&response, "Upgrade").as_deref(), Some("websocket"));
        assert_eq!(header(&response, "Connection").as_deref(), Some("Upgrade"));
        assert_eq!(
            header(&response, "Sec-WebSocket-Accept").as_deref(),
            Some(ACCEPT)
        );
        assert_eq!(header(&response, "Sec-WebSocket-Protocol"), None);
    }

    #[test]
    fn a_plain_get_is_refused_with_426() {
        let mut request = handshake();
        request.headers.remove("upgrade");

        let response = validate(&request, &HandshakeOptions::default()).unwrap_err();
        assert_eq!(response.status, StatusCode::UPGRADE_REQUIRED);
        assert_eq!(header(&response, "Upgrade").as_deref(), Some("websocket"));
        assert_eq!(
            header(&response, "Sec-WebSocket-Version").as_deref(),
            Some("13")
        );
    }

    #[test]
    fn an_old_protocol_version_is_refused_with_426_and_the_supported_version() {
        let mut request = handshake();
        request
            .headers
            .insert("sec-websocket-version".into(), "8".into());

        let response = validate(&request, &HandshakeOptions::default()).unwrap_err();
        assert_eq!(response.status, StatusCode::UPGRADE_REQUIRED);
        assert_eq!(
            header(&response, "Sec-WebSocket-Version").as_deref(),
            Some("13")
        );
    }

    #[test]
    fn a_missing_or_malformed_key_is_a_400() {
        let mut missing = handshake();
        missing.headers.remove("sec-websocket-key");
        assert_eq!(
            validate(&missing, &HandshakeOptions::default())
                .unwrap_err()
                .status,
            StatusCode::BAD_REQUEST
        );

        // Valid base64, but 15 bytes rather than 16.
        let mut wrong_length = handshake();
        wrong_length
            .headers
            .insert("sec-websocket-key".into(), "AAAAAAAAAAAAAAAAAAAA".into());
        assert_eq!(
            validate(&wrong_length, &HandshakeOptions::default())
                .unwrap_err()
                .status,
            StatusCode::BAD_REQUEST
        );

        let mut not_base64 = handshake();
        not_base64
            .headers
            .insert("sec-websocket-key".into(), "not base64!!".into());
        assert_eq!(
            validate(&not_base64, &HandshakeOptions::default())
                .unwrap_err()
                .status,
            StatusCode::BAD_REQUEST
        );
    }

    #[test]
    fn a_missing_version_header_is_a_400_not_a_426() {
        // 426 means "retry over WebSocket", which this client already is.
        let mut request = handshake();
        request.headers.remove("sec-websocket-version");
        assert_eq!(
            validate(&request, &HandshakeOptions::default())
                .unwrap_err()
                .status,
            StatusCode::BAD_REQUEST
        );
    }

    #[test]
    fn a_missing_origin_is_allowed() {
        // curl, a Rust client, a service mesh: no user agent attached cookies,
        // so there is no ambient authority to hijack.
        assert!(validate(&handshake(), &HandshakeOptions::default()).is_ok());
    }

    #[test]
    fn same_origin_is_allowed_with_and_without_the_default_port() {
        for origin in ["http://example.com", "http://example.com:80"] {
            let mut request = handshake();
            request.headers.insert("origin".into(), origin.into());
            assert!(
                validate(&request, &HandshakeOptions::default()).is_ok(),
                "{origin} is same-origin as example.com"
            );
        }
    }

    #[test]
    fn a_foreign_origin_is_refused_with_403() {
        let mut request = handshake();
        request
            .headers
            .insert("origin".into(), "https://evil.example".into());

        let response = validate(&request, &HandshakeOptions::default()).unwrap_err();
        assert_eq!(response.status, StatusCode::FORBIDDEN);
    }

    #[test]
    fn a_port_mismatch_is_a_different_origin() {
        let mut request = handshake();
        request.headers.insert("host".into(), "example.com".into());
        request
            .headers
            .insert("origin".into(), "http://example.com:8080".into());

        assert_eq!(
            validate(&request, &HandshakeOptions::default())
                .unwrap_err()
                .status,
            StatusCode::FORBIDDEN
        );
    }

    #[test]
    fn a_scheme_mismatch_is_a_different_origin() {
        // The request arrived over plain http, so an https origin is not the
        // same origin — that is a page on another site, or a downgrade.
        let mut request = handshake();
        request
            .headers
            .insert("origin".into(), "https://example.com".into());

        assert_eq!(
            validate(&request, &HandshakeOptions::default())
                .unwrap_err()
                .status,
            StatusCode::FORBIDDEN
        );
    }

    #[test]
    fn null_origin_is_never_same_origin() {
        // Sandboxed iframes and file:// pages send `null`. Accepting it would
        // hand any of them the visitor's session.
        let mut request = handshake();
        request.headers.insert("origin".into(), "null".into());

        assert_eq!(
            validate(&request, &HandshakeOptions::default())
                .unwrap_err()
                .status,
            StatusCode::FORBIDDEN
        );

        // Unless it is named explicitly, which is an informed choice.
        let options = HandshakeOptions {
            allowed_origins: vec!["null".into()],
            ..HandshakeOptions::default()
        };
        assert!(validate(&request, &options).is_ok());
    }

    #[test]
    fn an_allowlisted_origin_is_allowed_and_does_not_disable_same_origin() {
        let options = HandshakeOptions {
            allowed_origins: vec!["https://partner.example".into()],
            ..HandshakeOptions::default()
        };

        let mut partner = handshake();
        partner
            .headers
            .insert("origin".into(), "https://partner.example".into());
        assert!(validate(&partner, &options).is_ok());

        // The site's own pages keep working after an allowlist is added.
        let mut own = handshake();
        own.headers
            .insert("origin".into(), "http://example.com".into());
        assert!(validate(&own, &options).is_ok());

        let mut other = handshake();
        other
            .headers
            .insert("origin".into(), "https://evil.example".into());
        assert_eq!(
            validate(&other, &options).unwrap_err().status,
            StatusCode::FORBIDDEN
        );
    }

    #[test]
    fn a_star_allowlist_accepts_anything() {
        let options = HandshakeOptions {
            allowed_origins: vec!["*".into()],
            ..HandshakeOptions::default()
        };

        let mut request = handshake();
        request
            .headers
            .insert("origin".into(), "https://evil.example".into());
        assert!(validate(&request, &options).is_ok());
    }

    #[test]
    fn ipv6_hosts_compare_correctly() {
        let mut request = handshake();
        request.headers.insert("host".into(), "[::1]:8000".into());
        request
            .headers
            .insert("origin".into(), "http://[::1]:8000".into());
        assert!(validate(&request, &HandshakeOptions::default()).is_ok());

        request
            .headers
            .insert("origin".into(), "http://[::1]:9000".into());
        assert_eq!(
            validate(&request, &HandshakeOptions::default())
                .unwrap_err()
                .status,
            StatusCode::FORBIDDEN
        );
    }

    #[test]
    fn a_trailing_slash_does_not_become_part_of_the_host() {
        let mut request = handshake();
        request
            .headers
            .insert("origin".into(), "http://example.com/".into());
        assert!(validate(&request, &HandshakeOptions::default()).is_ok());
    }

    #[test]
    fn a_missing_origin_can_be_made_a_403() {
        // Off by default, because it would break every non-browser client.
        assert!(validate(&handshake(), &HandshakeOptions::default()).is_ok());

        let options = HandshakeOptions {
            require_origin: true,
            ..HandshakeOptions::default()
        };
        assert_eq!(
            validate(&handshake(), &options).unwrap_err().status,
            StatusCode::FORBIDDEN,
            "with require_origin on, a handshake with no Origin must be refused"
        );

        // And a same-origin one still gets through.
        let mut same_origin = handshake();
        same_origin
            .headers
            .insert("origin".into(), "http://example.com".into());
        assert!(validate(&same_origin, &options).is_ok());
    }

    #[test]
    fn subprotocol_matching_is_case_sensitive() {
        // RFC 7230 tokens. Echoing `chat` to a client that offered `Chat`
        // returns a value it never proposed.
        let options = HandshakeOptions {
            accept_subprotocols: vec!["chat".into()],
            ..HandshakeOptions::default()
        };

        let mut request = handshake();
        request
            .headers
            .insert("sec-websocket-protocol".into(), "Chat".into());
        assert_eq!(validate(&request, &options).unwrap().subprotocol, None);

        request
            .headers
            .insert("sec-websocket-protocol".into(), "chat".into());
        assert_eq!(
            validate(&request, &options).unwrap().subprotocol.as_deref(),
            Some("chat")
        );
    }

    #[test]
    fn subprotocol_negotiation_prefers_the_server_order() {
        let options = HandshakeOptions {
            accept_subprotocols: vec!["json".into(), "msgpack".into()],
            ..HandshakeOptions::default()
        };

        let mut request = handshake();
        request
            .headers
            .insert("sec-websocket-protocol".into(), "msgpack, json".into());

        let accepted = validate(&request, &options).unwrap();
        assert_eq!(accepted.subprotocol.as_deref(), Some("json"));
        assert_eq!(
            header(&accept_response(&accepted), "Sec-WebSocket-Protocol").as_deref(),
            Some("json")
        );
    }

    #[test]
    fn an_unknown_subprotocol_is_simply_not_echoed() {
        // RFC 6455 §4.2.2: the server may decline to select one. Refusing the
        // connection outright would break clients that offer an optional
        // protocol they can live without.
        let options = HandshakeOptions {
            accept_subprotocols: vec!["json".into()],
            ..HandshakeOptions::default()
        };

        let mut request = handshake();
        request
            .headers
            .insert("sec-websocket-protocol".into(), "graphql-ws".into());

        assert_eq!(validate(&request, &options).unwrap().subprotocol, None);
    }

    #[test]
    fn no_subprotocol_is_echoed_when_the_server_declares_none() {
        let mut request = handshake();
        request
            .headers
            .insert("sec-websocket-protocol".into(), "json".into());

        assert_eq!(
            validate(&request, &HandshakeOptions::default())
                .unwrap()
                .subprotocol,
            None
        );
    }
}
