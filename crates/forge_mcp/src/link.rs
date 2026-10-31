//! Collegamento tra il ponte `forgerender-mcp` (avviato da Claude) e la finestra di
//! ForgeRender aperta dall'utente.
//!
//! - Quando l'utente accende "Controllo IA", ForgeRender ascolta su `127.0.0.1` (porta scelta
//!   dal sistema) e scrive porta e chiave segreta nel file di contatto ([`contact_file`]),
//!   leggibile solo dall'utente. Spegnendo, il file viene tolto.
//! - Il ponte legge il file, si collega e manda la chiave. Poi, una riga JSON per messaggio:
//!   richiesta `{"name": …, "args": …}`, risposta = risultato MCP di `tools/call`.
//!
//! Il collegamento non esce dal PC (solo l'interfaccia locale) e senza la chiave viene chiuso.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::PathBuf;
use std::time::Duration;

/// Variabile d'ambiente che sostituisce il percorso del file di contatto (prove automatiche).
pub const ENV_CONTACT: &str = "FORGE_MCP_FILE";

/// Messaggio per l'assistente quando la finestra non accetta comandi.
pub const NOT_CONNECTED: &str = "ForgeRender non è collegato: l'utente deve aprire ForgeRender e accendere \
\"Controllo IA\" nella barra in alto. Chiediglielo e poi riprova.";

/// Contenuto del file di contatto.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Contact {
    pub port: u16,
    pub token: String,
    pub pid: u32,
    pub version: String,
}

/// File di contatto: `%LOCALAPPDATA%\ForgeRender\controllo-ia.json` su Windows,
/// `$XDG_RUNTIME_DIR/forgerender-controllo-ia.json` o `~/.local/state/forgerender/` altrove.
/// Vale anche per la versione portable (è uno stato temporaneo dell'utente, non un'impostazione).
pub fn contact_file() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os(ENV_CONTACT) {
        return Some(PathBuf::from(p));
    }
    if cfg!(windows) {
        return std::env::var_os("LOCALAPPDATA").map(|d| PathBuf::from(d).join("ForgeRender").join("controllo-ia.json"));
    }
    if let Some(d) = std::env::var_os("XDG_RUNTIME_DIR") {
        return Some(PathBuf::from(d).join("forgerender-controllo-ia.json"));
    }
    std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/state/forgerender/controllo-ia.json"))
}

/// Chiave casuale (32 cifre esadecimali) dagli hasher casuali della libreria standard, dal tempo
/// e dal processo: basta a impedire ad altri programmi di indovinarla.
pub fn new_token() -> String {
    use std::hash::{BuildHasher, Hasher};
    let mut out = String::new();
    for k in 0..2u64 {
        let mut h = std::collections::hash_map::RandomState::new().build_hasher();
        h.write_u64(k);
        h.write_u32(std::process::id());
        h.write_u128(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0),
        );
        out.push_str(&format!("{:016x}", h.finish()));
    }
    out
}

/// Scrive il file di contatto (solo l'utente può leggerlo dove il sistema lo permette).
pub fn write_contact(c: &Contact) -> std::io::Result<PathBuf> {
    let path = contact_file().ok_or_else(|| std::io::Error::other("cartella dell'utente non trovata"))?;
    if let Some(d) = path.parent() {
        std::fs::create_dir_all(d)?;
    }
    std::fs::write(&path, serde_json::to_vec_pretty(c).map_err(std::io::Error::other)?)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
    }
    Ok(path)
}

/// Toglie il file di contatto se è ancora quello con questa chiave (un'altra finestra può
/// averlo sostituito).
pub fn remove_contact(token: &str) {
    if let Some(path) = contact_file() {
        if read_contact_at(&path).is_ok_and(|c| c.token == token) {
            let _ = std::fs::remove_file(path);
        }
    }
}

fn read_contact_at(path: &std::path::Path) -> Result<Contact, String> {
    let text = std::fs::read_to_string(path).map_err(|_| NOT_CONNECTED.to_string())?;
    serde_json::from_str(&text).map_err(|e| format!("file di contatto {} non valido: {e}", path.display()))
}

/// Ponte → finestra: collegamento autenticato.
pub struct Client {
    writer: TcpStream,
    reader: BufReader<TcpStream>,
    pub version: String,
}

impl Client {
    /// Si collega alla finestra indicata dal file di contatto.
    pub fn connect(assistant: Option<&str>) -> Result<Client, String> {
        let path = contact_file().ok_or(NOT_CONNECTED)?;
        let c = read_contact_at(&path)?;
        let addr = SocketAddr::from(([127, 0, 0, 1], c.port));
        let stream = TcpStream::connect_timeout(&addr, Duration::from_secs(3)).map_err(|_| NOT_CONNECTED.to_string())?;
        let _ = stream.set_nodelay(true);
        let mut client = Client {
            reader: BufReader::new(stream.try_clone().map_err(|e| e.to_string())?),
            writer: stream,
            version: c.version.clone(),
        };
        // Saluto con la chiave, con un tempo massimo: una finestra bloccata non blocca il ponte.
        client.writer.set_read_timeout(Some(Duration::from_secs(10))).ok();
        client.send(&json!({
            "token": c.token,
            "client": "forgerender-mcp",
            "version": env!("CARGO_PKG_VERSION"),
            "assistant": assistant,
        }))?;
        let hello = client.recv()?;
        if hello["ok"] != true {
            return Err(hello["error"].as_str().unwrap_or(NOT_CONNECTED).to_string());
        }
        client.writer.set_read_timeout(None).ok();
        Ok(client)
    }

    fn send(&mut self, v: &Value) -> Result<(), String> {
        let line = format!("{v}\n");
        self.writer
            .write_all(line.as_bytes())
            .and_then(|_| self.writer.flush())
            .map_err(|_| "collegamento con ForgeRender interrotto".to_string())
    }

    fn recv(&mut self) -> Result<Value, String> {
        let mut line = String::new();
        match self.reader.read_line(&mut line) {
            Ok(0) | Err(_) => Err("ForgeRender si è chiuso, oppure l'utente ha spento \"Controllo IA\"".into()),
            Ok(_) => serde_json::from_str(&line).map_err(|e| format!("risposta non valida da ForgeRender: {e}")),
        }
    }

    /// Il collegamento è ancora aperto? (La finestra può averlo chiuso: controllo spento,
    /// programma chiuso.) Controllato prima di mandare un comando, mai dopo.
    pub fn is_alive(&self) -> bool {
        if self.writer.set_nonblocking(true).is_err() {
            return false;
        }
        let mut b = [0u8; 1];
        let alive = match self.writer.peek(&mut b) {
            Ok(0) => false,
            Ok(_) => true,
            Err(e) => e.kind() == std::io::ErrorKind::WouldBlock,
        };
        let _ = self.writer.set_nonblocking(false);
        alive
    }

    /// Esegue uno strumento nella finestra; restituisce il risultato MCP.
    pub fn call(&mut self, name: &str, args: &Value) -> Result<Value, String> {
        self.send(&json!({"name": name, "args": args}))?;
        self.recv()
    }
}

/// Lato finestra: legge il saluto del ponte e controlla la chiave. Restituisce il nome con cui
/// l'assistente si è presentato al ponte (`clientInfo.name` dell'MCP), se lo conosce.
pub fn check_hello(line: &str, token: &str) -> Result<Option<String>, String> {
    let v: Value = serde_json::from_str(line).map_err(|_| "saluto non valido".to_string())?;
    if v["token"].as_str() == Some(token) {
        Ok(v["assistant"].as_str().filter(|s| !s.is_empty()).map(str::to_string))
    } else {
        Err("chiave non valida".into())
    }
}

/// Lato finestra: legge una richiesta `{"name", "args"}`.
pub fn parse_request(line: &str) -> Result<(String, Value), String> {
    let v: Value = serde_json::from_str(line).map_err(|e| format!("richiesta non valida: {e}"))?;
    let name = v["name"].as_str().ok_or("richiesta senza nome")?.to_string();
    Ok((name, v.get("args").cloned().unwrap_or_else(|| json!({}))))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_are_random_and_long() {
        let a = new_token();
        let b = new_token();
        assert_eq!(a.len(), 32);
        assert_ne!(a, b);
        assert!(check_hello(&json!({"token": a}).to_string(), &a).is_ok());
        assert!(check_hello(&json!({"token": b}).to_string(), &a).is_err());
        assert!(check_hello("rotto", &a).is_err());
    }

    #[test]
    fn requests_parse() {
        let (n, a) = parse_request(r#"{"name":"render","args":{"samples":8}}"#).unwrap();
        assert_eq!(n, "render");
        assert_eq!(a["samples"], 8);
        assert_eq!(parse_request(r#"{"name":"scene_info"}"#).unwrap().1, json!({}));
        assert!(parse_request(r#"{"args":{}}"#).is_err());
    }
}
