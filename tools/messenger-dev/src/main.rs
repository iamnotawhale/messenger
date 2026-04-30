use messenger_crypto::{IdentityKeypair, PrivateIdentity, PublicIdentity};
use messenger_protocol::{
    AuthChallenge, AuthChallengeRequest, AuthVerifyRequest, AuthVerifyResponse, Envelope,
    MarkDeliveredResponse, PendingEnvelopesResponse, PublicIdentityDocument, SubmitEnvelopeRequest,
    SubmitEnvelopeResponse,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{
    env, fs,
    io::{Read, Write},
    net::TcpStream,
    path::Path,
};
use thiserror::Error;

const DEFAULT_SERVER: &str = "http://127.0.0.1:8080";

#[derive(Debug, Error)]
enum CliError {
    #[error("usage: {0}")]
    Usage(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("crypto error: {0}")]
    Crypto(#[from] messenger_crypto::CryptoError),
    #[error("http error: {0}")]
    Http(String),
}

type Result<T> = std::result::Result<T, CliError>;

#[derive(Debug, Serialize, Deserialize)]
struct IdentityFile {
    identity: PrivateIdentity,
}

#[derive(Debug, Serialize, Deserialize)]
struct PublicIdentityFile {
    identity: PublicIdentity,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    match args.as_slice() {
        [] => {
            println!("{}", usage());
            Ok(())
        }
        [command] if command == "help" || command == "--help" || command == "-h" => {
            println!("{}", usage());
            Ok(())
        }
        [command, subcommand, path] if command == "identity" && subcommand == "new" => {
            identity_new(path)
        }
        [command, subcommand, private_path, public_path]
            if command == "identity" && subcommand == "public" =>
        {
            identity_public(private_path, public_path)
        }
        [command, rest @ ..] if command == "send" => send(rest),
        [command, rest @ ..] if command == "receive" => receive(rest),
        _ => Err(CliError::Usage(usage())),
    }
}

fn identity_new(path: &str) -> Result<()> {
    let identity = IdentityKeypair::generate();
    write_json(
        path,
        &IdentityFile {
            identity: identity.private_identity(),
        },
    )?;
    println!("created identity {}", identity.peer_id());
    Ok(())
}

fn identity_public(private_path: &str, public_path: &str) -> Result<()> {
    let identity = load_identity(private_path)?;
    write_json(
        public_path,
        &PublicIdentityFile {
            identity: identity.public_identity(),
        },
    )?;
    println!("wrote public identity {}", identity.peer_id());
    Ok(())
}

fn send(args: &[String]) -> Result<()> {
    let server = option_value(args, "--server").unwrap_or(DEFAULT_SERVER);
    let from_path = required_option(args, "--from")?;
    let to_path = required_option(args, "--to")?;
    let text = required_option(args, "--text")?;
    let sender = load_identity(from_path)?;
    let recipient = load_public_identity(to_path)?;
    let session = authenticate(server, &sender)?;
    let envelope = sender.encrypt_for(&recipient, text.as_bytes())?;
    let response: SubmitEnvelopeResponse = http_json(
        server,
        "POST",
        "/v1/relay/envelopes",
        Some(&session),
        &SubmitEnvelopeRequest { envelope },
    )?;

    println!(
        "submitted message {} accepted={}",
        response.message_id, response.accepted
    );
    Ok(())
}

fn receive(args: &[String]) -> Result<()> {
    let server = option_value(args, "--server").unwrap_or(DEFAULT_SERVER);
    let identity_path = required_option(args, "--identity")?;
    let from_path = required_option(args, "--from")?;
    let recipient = load_identity(identity_path)?;
    let sender = load_public_identity(from_path)?;
    let session = authenticate(server, &recipient)?;
    let pending: PendingEnvelopesResponse =
        http_empty(server, "GET", "/v1/relay/envelopes/pending", Some(&session))?;

    if pending.envelopes.is_empty() {
        println!("no pending messages");
        return Ok(());
    }

    for envelope in pending.envelopes {
        print_message(&recipient, &sender, &envelope)?;
        let path = format!("/v1/relay/envelopes/{}/delivered", envelope.message_id);
        let delivered: MarkDeliveredResponse = http_empty(server, "POST", &path, Some(&session))?;
        println!(
            "marked {} delivered removed={}",
            envelope.message_id, delivered.removed
        );
    }

    Ok(())
}

fn authenticate(server: &str, identity: &IdentityKeypair) -> Result<String> {
    let challenge: AuthChallenge = http_json(
        server,
        "POST",
        "/v1/auth/challenge",
        None,
        &AuthChallengeRequest {
            peer_id: identity.peer_id(),
        },
    )?;
    let response: AuthVerifyResponse = http_json(
        server,
        "POST",
        "/v1/auth/verify",
        None,
        &AuthVerifyRequest {
            identity: public_identity_document(identity),
            challenge_id: challenge.challenge_id.clone(),
            signature: identity.sign_auth_challenge(&challenge),
        },
    )?;

    Ok(response.session_token)
}

fn public_identity_document(identity: &IdentityKeypair) -> PublicIdentityDocument {
    let public = identity.public_identity();
    PublicIdentityDocument {
        peer_id: public.peer_id,
        signing_key: public.signing_key,
        agreement_key: public.agreement_key,
    }
}

fn print_message(
    recipient: &IdentityKeypair,
    sender: &PublicIdentity,
    envelope: &Envelope,
) -> Result<()> {
    let plaintext = recipient.decrypt_from(sender, envelope)?;
    println!(
        "{}: {}",
        envelope.message_id,
        String::from_utf8_lossy(&plaintext)
    );
    Ok(())
}

fn load_identity(path: &str) -> Result<IdentityKeypair> {
    let file: IdentityFile = read_json(path)?;
    Ok(IdentityKeypair::from_private_identity(file.identity))
}

fn load_public_identity(path: &str) -> Result<PublicIdentity> {
    let file: PublicIdentityFile = read_json(path)?;
    Ok(file.identity)
}

fn read_json<T: DeserializeOwned>(path: &str) -> Result<T> {
    Ok(serde_json::from_slice(&fs::read(path)?)?)
}

fn write_json<T: Serialize>(path: &str, value: &T) -> Result<()> {
    if let Some(parent) = Path::new(path).parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    fs::write(path, serde_json::to_vec_pretty(value)?)?;
    Ok(())
}

fn http_json<TRequest, TResponse>(
    server: &str,
    method: &str,
    path: &str,
    bearer_token: Option<&str>,
    payload: &TRequest,
) -> Result<TResponse>
where
    TRequest: Serialize,
    TResponse: DeserializeOwned,
{
    http_request(
        server,
        method,
        path,
        bearer_token,
        Some(serde_json::to_vec(payload)?),
    )
}

fn http_empty<TResponse>(
    server: &str,
    method: &str,
    path: &str,
    bearer_token: Option<&str>,
) -> Result<TResponse>
where
    TResponse: DeserializeOwned,
{
    http_request(server, method, path, bearer_token, None)
}

fn http_request<TResponse>(
    server: &str,
    method: &str,
    path: &str,
    bearer_token: Option<&str>,
    body: Option<Vec<u8>>,
) -> Result<TResponse>
where
    TResponse: DeserializeOwned,
{
    let endpoint = parse_http_url(server)?;
    let body = body.unwrap_or_default();
    let mut stream = TcpStream::connect((&endpoint.host[..], endpoint.port))?;
    let mut request = format!(
        "{method} {path} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\nContent-Length: {}\r\n",
        endpoint.host,
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
        return Err(CliError::Http(format!(
            "server returned HTTP {status}: {}",
            String::from_utf8_lossy(body)
        )));
    }

    Ok(serde_json::from_slice(body)?)
}

#[derive(Debug)]
struct HttpEndpoint {
    host: String,
    port: u16,
}

fn parse_http_url(server: &str) -> Result<HttpEndpoint> {
    let without_scheme = server
        .strip_prefix("http://")
        .ok_or_else(|| CliError::Usage("only http:// relay URLs are supported".to_owned()))?;
    let host_port = without_scheme.trim_end_matches('/');
    let (host, port) = match host_port.rsplit_once(':') {
        Some((host, port)) => (
            host.to_owned(),
            port.parse::<u16>()
                .map_err(|_| CliError::Usage("invalid relay port".to_owned()))?,
        ),
        None => (host_port.to_owned(), 80),
    };
    Ok(HttpEndpoint { host, port })
}

fn parse_http_response(response: &[u8]) -> Result<(u16, &[u8])> {
    let separator = b"\r\n\r\n";
    let header_end = response
        .windows(separator.len())
        .position(|window| window == separator)
        .ok_or_else(|| CliError::Http("malformed HTTP response".to_owned()))?;
    let headers = &response[..header_end];
    let body = &response[header_end + separator.len()..];
    let status_line_end = headers
        .windows(2)
        .position(|window| window == b"\r\n")
        .unwrap_or(headers.len());
    let status_line = std::str::from_utf8(&headers[..status_line_end])
        .map_err(|_| CliError::Http("invalid HTTP status line".to_owned()))?;
    let status = status_line
        .split_whitespace()
        .nth(1)
        .ok_or_else(|| CliError::Http("missing HTTP status".to_owned()))?
        .parse::<u16>()
        .map_err(|_| CliError::Http("invalid HTTP status".to_owned()))?;
    Ok((status, body))
}

fn option_value<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    args.windows(2)
        .find(|window| window[0] == name)
        .map(|window| window[1].as_str())
}

fn required_option<'a>(args: &'a [String], name: &str) -> Result<&'a str> {
    option_value(args, name).ok_or_else(|| CliError::Usage(format!("missing {name}\n{}", usage())))
}

fn usage() -> String {
    [
        "messenger-dev identity new <identity.json>",
        "messenger-dev identity public <identity.json> <public.json>",
        "messenger-dev send --server <url> --from <identity.json> --to <public.json> --text <message>",
        "messenger-dev receive --server <url> --identity <identity.json> --from <public.json>",
    ]
    .join("\n")
}
