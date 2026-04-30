use messenger_crypto::{IdentityKeypair, PrivateIdentity, PublicIdentity};
use messenger_transport::{RelayHttpClient, TransportError};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{env, fs, path::Path};
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
    #[error("transport error: {0}")]
    Transport(#[from] TransportError),
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
    let client = RelayHttpClient::new(server)?;
    let session = client.authenticate(&sender)?;
    let envelope = sender.encrypt_for(&recipient, text.as_bytes())?;
    let response = client.submit(&session, envelope)?;

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
    let client = RelayHttpClient::new(server)?;
    let session = client.authenticate(&recipient)?;
    let pending = client.pending(&session)?;

    if pending.is_empty() {
        println!("no pending messages");
        return Ok(());
    }

    for envelope in pending {
        print_message(&recipient, &sender, &envelope)?;
        let delivered = client.mark_delivered(&session, envelope.message_id)?;
        println!(
            "marked {} delivered removed={}",
            envelope.message_id, delivered.removed
        );
    }

    Ok(())
}

fn print_message(
    recipient: &IdentityKeypair,
    sender: &PublicIdentity,
    envelope: &messenger_protocol::Envelope,
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
