use messenger_crypto::IdentityKeypair;
use messenger_protocol::{
    AuthChallenge, AuthChallengeRequest, AuthVerifyRequest, AuthVerifyResponse, Envelope,
    MarkDeliveredResponse, PendingEnvelopesResponse, PublicIdentityDocument, SubmitEnvelopeRequest,
    SubmitEnvelopeResponse,
};
use serde::{de::DeserializeOwned, Serialize};
use std::{
    io::{Read, Write},
    net::TcpStream,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TransportError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid relay URL: {0}")]
    InvalidUrl(String),
    #[error("http error: {0}")]
    Http(String),
}

pub type Result<T> = std::result::Result<T, TransportError>;

#[derive(Debug, Clone)]
pub struct RelayHttpClient {
    endpoint: HttpEndpoint,
}

impl RelayHttpClient {
    pub fn new(server_url: impl Into<String>) -> Result<Self> {
        Ok(Self {
            endpoint: parse_http_url(&server_url.into())?,
        })
    }

    pub fn authenticate(&self, identity: &IdentityKeypair) -> Result<RelaySession> {
        let challenge: AuthChallenge = self.request_json(
            "POST",
            "/v1/auth/challenge",
            None,
            &AuthChallengeRequest {
                peer_id: identity.peer_id(),
            },
        )?;
        let response: AuthVerifyResponse = self.request_json(
            "POST",
            "/v1/auth/verify",
            None,
            &AuthVerifyRequest {
                identity: public_identity_document(identity),
                challenge_id: challenge.challenge_id.clone(),
                signature: identity.sign_auth_challenge(&challenge),
            },
        )?;

        Ok(RelaySession {
            token: response.session_token,
            expires_at_ms: response.expires_at_ms,
        })
    }

    pub fn submit(
        &self,
        session: &RelaySession,
        envelope: Envelope,
    ) -> Result<SubmitEnvelopeResponse> {
        self.request_json(
            "POST",
            "/v1/relay/envelopes",
            Some(session.token()),
            &SubmitEnvelopeRequest { envelope },
        )
    }

    pub fn pending(&self, session: &RelaySession) -> Result<Vec<Envelope>> {
        let response: PendingEnvelopesResponse =
            self.request_empty("GET", "/v1/relay/envelopes/pending", Some(session.token()))?;
        Ok(response.envelopes)
    }

    pub fn mark_delivered(
        &self,
        session: &RelaySession,
        message_id: impl std::fmt::Display,
    ) -> Result<MarkDeliveredResponse> {
        self.request_empty(
            "POST",
            &format!("/v1/relay/envelopes/{message_id}/delivered"),
            Some(session.token()),
        )
    }

    fn request_json<TRequest, TResponse>(
        &self,
        method: &str,
        path: &str,
        bearer_token: Option<&str>,
        payload: &TRequest,
    ) -> Result<TResponse>
    where
        TRequest: Serialize,
        TResponse: DeserializeOwned,
    {
        self.request(
            method,
            path,
            bearer_token,
            Some(serde_json::to_vec(payload)?),
        )
    }

    fn request_empty<TResponse>(
        &self,
        method: &str,
        path: &str,
        bearer_token: Option<&str>,
    ) -> Result<TResponse>
    where
        TResponse: DeserializeOwned,
    {
        self.request(method, path, bearer_token, None)
    }

    fn request<TResponse>(
        &self,
        method: &str,
        path: &str,
        bearer_token: Option<&str>,
        body: Option<Vec<u8>>,
    ) -> Result<TResponse>
    where
        TResponse: DeserializeOwned,
    {
        let body = body.unwrap_or_default();
        let mut stream = TcpStream::connect((&self.endpoint.host[..], self.endpoint.port))?;
        let mut request = format!(
            "{method} {path} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\nContent-Length: {}\r\n",
            self.endpoint.host,
            body.len()
        );

        if !body.is_empty() {
            request.push_str("Content-Type: application/json\r\n");
        }

        if let Some(token) = bearer_token {
            request.push_str(&format!("Authorization: Bearer {token}\r\n"));
        }

        request.push_str("\r\n");
        stream.write_all(request.as_bytes())?;
        stream.write_all(&body)?;

        let mut response = Vec::new();
        stream.read_to_end(&mut response)?;
        let (status, body) = parse_http_response(&response)?;
        if !(200..300).contains(&status) {
            return Err(TransportError::Http(format!(
                "server returned HTTP {status}: {}",
                String::from_utf8_lossy(body)
            )));
        }

        Ok(serde_json::from_slice(body)?)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RelaySession {
    token: String,
    expires_at_ms: u64,
}

impl RelaySession {
    pub fn token(&self) -> &str {
        &self.token
    }

    pub fn expires_at_ms(&self) -> u64 {
        self.expires_at_ms
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct HttpEndpoint {
    host: String,
    port: u16,
}

fn public_identity_document(identity: &IdentityKeypair) -> PublicIdentityDocument {
    let public = identity.public_identity();
    PublicIdentityDocument {
        peer_id: public.peer_id,
        signing_key: public.signing_key,
        agreement_key: public.agreement_key,
    }
}

fn parse_http_url(server: &str) -> Result<HttpEndpoint> {
    let without_scheme = server
        .strip_prefix("http://")
        .ok_or_else(|| TransportError::InvalidUrl("only http:// URLs are supported".to_owned()))?;
    let host_port = without_scheme.trim_end_matches('/');
    let (host, port) = match host_port.rsplit_once(':') {
        Some((host, port)) => (
            host.to_owned(),
            port.parse::<u16>()
                .map_err(|_| TransportError::InvalidUrl("invalid port".to_owned()))?,
        ),
        None => (host_port.to_owned(), 80),
    };

    if host.is_empty() {
        return Err(TransportError::InvalidUrl("missing host".to_owned()));
    }

    Ok(HttpEndpoint { host, port })
}

fn parse_http_response(response: &[u8]) -> Result<(u16, &[u8])> {
    let separator = b"\r\n\r\n";
    let header_end = response
        .windows(separator.len())
        .position(|window| window == separator)
        .ok_or_else(|| TransportError::Http("malformed HTTP response".to_owned()))?;
    let headers = &response[..header_end];
    let body = &response[header_end + separator.len()..];
    let status_line_end = headers
        .windows(2)
        .position(|window| window == b"\r\n")
        .unwrap_or(headers.len());
    let status_line = std::str::from_utf8(&headers[..status_line_end])
        .map_err(|_| TransportError::Http("invalid HTTP status line".to_owned()))?;
    let status = status_line
        .split_whitespace()
        .nth(1)
        .ok_or_else(|| TransportError::Http("missing HTTP status".to_owned()))?
        .parse::<u16>()
        .map_err(|_| TransportError::Http("invalid HTTP status".to_owned()))?;
    Ok((status, body))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_http_urls() -> Result<()> {
        assert_eq!(
            parse_http_url("http://127.0.0.1:8080")?,
            HttpEndpoint {
                host: "127.0.0.1".to_owned(),
                port: 8080,
            }
        );
        assert_eq!(
            parse_http_url("http://localhost")?,
            HttpEndpoint {
                host: "localhost".to_owned(),
                port: 80,
            }
        );
        Ok(())
    }

    #[test]
    fn rejects_non_http_urls() {
        assert!(matches!(
            parse_http_url("https://example.com"),
            Err(TransportError::InvalidUrl(_))
        ));
    }
}
