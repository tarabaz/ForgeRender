//! `forgerender-mcp`: il programma che Claude Code / Claude Desktop avviano come server MCP.
//!
//! Non apre ForgeRender e non contiene il renderer: risponde da solo a inizializzazione ed
//! elenco degli strumenti, e inoltra ogni comando alla finestra di ForgeRender che l'utente ha
//! aperto con "Controllo IA" acceso. Quando Claude chiude il ponte, la finestra resta aperta.
//! Se la finestra non c'è, i comandi rispondono con un errore che spiega cosa fare.
//!
//! All'inizializzazione il ponte si presenta subito alla finestra (se il controllo è acceso)
//! con il nome dell'assistente (`clientInfo.name`): la finestra mostra chi è collegato.

use forge_mcp::link::Client;
use forge_mcp::rpc::{self, ToolOutput};
use serde_json::Value;
use std::io::{BufRead, Write};

fn main() {
    if std::env::args().any(|a| a == "--version" || a == "-V") {
        println!("forgerender-mcp {}", env!("CARGO_PKG_VERSION"));
        return;
    }
    let tools = forge_mcp::tools::list();
    let mut client: Option<Client> = None;
    let mut assistant: Option<String> = None;
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        let initialize = rpc::client_name(&line);
        if let Some(name) = &initialize {
            assistant = Some(name.clone());
        }
        let mut call = |name: &str, args: &Value| call_window(&mut client, assistant.as_deref(), name, args);
        if let Some(resp) = rpc::handle_line(&line, &tools, &mut call) {
            if writeln!(stdout, "{resp}").and_then(|_| stdout.flush()).is_err() {
                break;
            }
        }
        // Dopo la risposta all'inizializzazione: collegamento subito, se la finestra c'è.
        if initialize.is_some() && client.is_none() {
            if let Ok(c) = Client::connect(assistant.as_deref()) {
                eprintln!("forgerender-mcp: collegato a ForgeRender {}", c.version);
                client = Some(c);
            }
        }
    }
}

/// Inoltra il comando alla finestra, collegandosi (o ricollegandosi) se serve. Un comando già
/// consegnato non viene mai rimandato: se il collegamento cade a metà, si risponde con l'errore.
fn call_window(client: &mut Option<Client>, assistant: Option<&str>, name: &str, args: &Value) -> Value {
    if client.as_ref().is_some_and(|c| !c.is_alive()) {
        *client = None;
    }
    if client.is_none() {
        match Client::connect(assistant) {
            Ok(c) => {
                eprintln!("forgerender-mcp: collegato a ForgeRender {}", c.version);
                *client = Some(c);
            }
            Err(e) => return ToolOutput::error(e).to_json(),
        }
    }
    let c = client.as_mut().expect("collegato");
    match c.call(name, args) {
        Ok(v) => v,
        Err(e) => {
            *client = None;
            ToolOutput::error(e).to_json()
        }
    }
}
