use messenger_client::{ClientError, MessengerClient};
use messenger_client_store::{ClientStoreError, MessageDirection};
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
    #[error("client error: {0}")]
    Client(#[from] ClientError),
    #[error("client store error: {0}")]
    ClientStore(#[from] ClientStoreError),
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
        [command, subcommand, rest @ ..] if command == "db" && subcommand == "init" => {
            db_init(rest)
        }
        [command, subcommand, rest @ ..] if command == "db" && subcommand == "public" => {
            db_public(rest)
        }
        [command, subcommand, rest @ ..] if command == "contact" && subcommand == "add" => {
            contact_add(rest)
        }
        [command, subcommand, rest @ ..] if command == "contact" && subcommand == "list" => {
            contact_list(rest)
        }
        [command, subcommand, rest @ ..] if command == "message" && subcommand == "send" => {
            message_send(rest)
        }
        [command, rest @ ..] if command == "sync" => sync(rest),
        [command, subcommand, rest @ ..] if command == "messages" && subcommand == "list" => {
            messages_list(rest)
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

fn db_init(args: &[String]) -> Result<()> {
    let database = required_option(args, "--db")?;
    let server = option_value(args, "--server").unwrap_or(DEFAULT_SERVER);
    let client = MessengerClient::open(database, server)?;
    let peer_id = client.init_identity()?;
    println!("initialized client db for {peer_id}");
    Ok(())
}

fn db_public(args: &[String]) -> Result<()> {
    let database = required_option(args, "--db")?;
    let output = required_option(args, "--out")?;
    let server = option_value(args, "--server").unwrap_or(DEFAULT_SERVER);
    let client = MessengerClient::open(database, server)?;
    let identity = client.identity()?;
    write_json(
        output,
        &PublicIdentityFile {
            identity: identity.public_identity(),
        },
    )?;
    println!("wrote public identity {}", identity.peer_id());
    Ok(())
}

fn contact_add(args: &[String]) -> Result<()> {
    let database = required_option(args, "--db")?;
    let name = required_option(args, "--name")?;
    let public_path = required_option(args, "--public")?;
    let server = option_value(args, "--server").unwrap_or(DEFAULT_SERVER);
    let client = MessengerClient::open(database, server)?;
    let public_identity = load_public_identity(public_path)?;
    client.add_contact(name, &public_identity)?;
    println!("added contact {name} ({})", public_identity.peer_id);
    Ok(())
}

fn contact_list(args: &[String]) -> Result<()> {
    let database = required_option(args, "--db")?;
    let server = option_value(args, "--server").unwrap_or(DEFAULT_SERVER);
    let client = MessengerClient::open(database, server)?;
    let contacts = client.store().contacts()?;
    if contacts.is_empty() {
        println!("no contacts");
        return Ok(());
    }

    for contact in contacts {
        println!("{} {}", contact.display_name, contact.peer_id);
    }
    Ok(())
}

fn message_send(args: &[String]) -> Result<()> {
    let database = required_option(args, "--db")?;
    let to = required_option(args, "--to")?;
    let text = required_option(args, "--text")?;
    let server = option_value(args, "--server").unwrap_or(DEFAULT_SERVER);
    let client = MessengerClient::open(database, server)?;
    let sent = client.send_message(to, text)?;
    println!(
        "sent message {} accepted={}",
        sent.message_id, sent.accepted
    );
    Ok(())
}

fn sync(args: &[String]) -> Result<()> {
    let database = required_option(args, "--db")?;
    let server = option_value(args, "--server").unwrap_or(DEFAULT_SERVER);
    let client = MessengerClient::open(database, server)?;
    let messages = client.sync_pending()?;
    if messages.is_empty() {
        println!("no pending messages");
        return Ok(());
    }

    for message in messages {
        println!(
            "{} from {}: {}",
            message.message_id, message.sender, message.body
        );
    }
    Ok(())
}

fn messages_list(args: &[String]) -> Result<()> {
    let database = required_option(args, "--db")?;
    let contact = required_option(args, "--contact")?;
    let server = option_value(args, "--server").unwrap_or(DEFAULT_SERVER);
    let client = MessengerClient::open(database, server)?;
    let contact = client
        .store()
        .contact_by_name(contact)?
        .ok_or_else(|| CliError::Usage("unknown contact".to_owned()))?;
    let messages = client.store().messages_for_peer(&contact.peer_id)?;
    if messages.is_empty() {
        println!("no messages");
        return Ok(());
    }

    for message in messages {
        let direction = match message.direction {
            MessageDirection::Inbound => "in",
            MessageDirection::Outbound => "out",
        };
        println!("[{direction}] {}: {}", message.message_id, message.body);
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
        "messenger-dev db init --db <client.db> [--server <url>]",
        "messenger-dev db public --db <client.db> --out <public.json> [--server <url>]",
        "messenger-dev contact add --db <client.db> --name <name> --public <public.json> [--server <url>]",
        "messenger-dev contact list --db <client.db> [--server <url>]",
        "messenger-dev message send --db <client.db> --to <name> --text <message> [--server <url>]",
        "messenger-dev sync --db <client.db> [--server <url>]",
        "messenger-dev messages list --db <client.db> --contact <name> [--server <url>]",
        "messenger-dev send --server <url> --from <identity.json> --to <public.json> --text <message>",
        "messenger-dev receive --server <url> --identity <identity.json> --from <public.json>",
    ]
    .join("\n")
}
