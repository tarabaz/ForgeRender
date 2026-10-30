//! Controllo di ForgeRender da un assistente IA (Claude Code, Claude Desktop…) con il
//! Model Context Protocol. L'utente apre ForgeRender e accende "Controllo IA" nella barra:
//! la finestra ascolta su `127.0.0.1` e Claude ci arriva tramite il ponte `forgerender-mcp`
//! (crate `forge_mcp_bridge`), che è il programma che Claude avvia e chiude. La finestra resta
//! dell'utente: quando Claude si scollega rimane aperta con il lavoro. Guida: `docs/MCP.md`.
//!
//! Ogni collegamento ha un thread che legge le richieste; l'esecuzione passa alla finestra con
//! un canale e avviene all'inizio del fotogramma ([`process`]), come un clic sui pulsanti:
//! niente accessi concorrenti alla scena. Import, apertura dei progetti e render finiscono nei
//! fotogrammi successivi.

pub mod ops;
pub mod tools;

use crate::app::ForgeApp;
use eframe::egui;
use forge_mcp::link;
use forge_mcp::rpc::ToolOutput;
use serde_json::Value;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Chiamata di uno strumento in attesa della finestra.
pub struct Call {
    name: String,
    args: Value,
    /// La finestra l'ha presa in carico (per distinguere "lenta" da "ferma").
    picked: Arc<AtomicBool>,
    reply: Sender<ToolOutput>,
}

/// "Controllo IA" acceso: ascolto locale e chiamate in arrivo.
pub struct Server {
    rx: Receiver<Call>,
    waiting: Option<(Sender<ToolOutput>, tools::Waiter)>,
    stop: Arc<AtomicBool>,
    token: String,
    /// Collegamenti aperti (per chiuderli quando l'utente spegne il controllo).
    streams: Arc<Mutex<Vec<TcpStream>>>,
    /// Ponti collegati in questo momento ("Controllato da IA" nella barra).
    pub clients: Arc<AtomicUsize>,
    /// Strumenti eseguiti e ultimo nome (suggerimento dell'indicatore).
    pub calls: u64,
    pub last: Option<String>,
}

/// Se la finestra non prende la chiamata entro questo tempo, si risponde con un errore.
const PICKUP_TIMEOUT: Duration = Duration::from_secs(20);

/// Accende il controllo: ascolto su una porta locale scelta dal sistema e file di contatto
/// per il ponte. `ctx` serve a svegliare la finestra a ogni richiesta.
pub fn start(ctx: egui::Context) -> Result<Server, String> {
    let listener = TcpListener::bind(("127.0.0.1", 0)).map_err(|e| format!("controllo IA non attivato: {e}"))?;
    let port = listener.local_addr().map_err(|e| e.to_string())?.port();
    listener.set_nonblocking(true).map_err(|e| e.to_string())?;
    let token = link::new_token();
    let contact = link::Contact {
        port,
        token: token.clone(),
        pid: std::process::id(),
        version: env!("CARGO_PKG_VERSION").into(),
    };
    let path = link::write_contact(&contact).map_err(|e| format!("controllo IA non attivato: file di contatto non scritto ({e})"))?;
    let (tx, rx) = channel::<Call>();
    let stop = Arc::new(AtomicBool::new(false));
    let clients = Arc::new(AtomicUsize::new(0));
    let streams: Arc<Mutex<Vec<TcpStream>>> = Arc::default();
    {
        let (stop, clients, streams, token) = (stop.clone(), clients.clone(), streams.clone(), token.clone());
        std::thread::Builder::new()
            .name("forge-ia-ascolto".into())
            .spawn(move || {
                while !stop.load(Ordering::Acquire) {
                    match listener.accept() {
                        Ok((stream, _)) => {
                            let (tx, ctx, stop, clients, token) = (tx.clone(), ctx.clone(), stop.clone(), clients.clone(), token.clone());
                            if let Ok(s) = stream.try_clone() {
                                streams.lock().expect("collegamenti").push(s);
                            }
                            let _ = std::thread::Builder::new()
                                .name("forge-ia-collegamento".into())
                                .spawn(move || serve(stream, &token, &tx, &ctx, &stop, &clients));
                        }
                        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => std::thread::sleep(Duration::from_millis(100)),
                        Err(e) => {
                            tracing::warn!("controllo IA: {e}");
                            std::thread::sleep(Duration::from_millis(500));
                        }
                    }
                }
            })
            .map_err(|e| e.to_string())?;
    }
    tracing::info!("controllo IA acceso: 127.0.0.1:{port}, contatto in {}", path.display());
    Ok(Server {
        rx,
        waiting: None,
        stop,
        token,
        streams,
        clients,
        calls: 0,
        last: None,
    })
}

impl Server {
    /// Spegne il controllo: niente più collegamenti, quelli aperti vengono chiusi, il comando
    /// in corso riceve un errore.
    pub fn shutdown(mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some((reply, _)) = self.waiting.take() {
            let _ = reply.send(ToolOutput::error("l'utente ha spento \"Controllo IA\""));
        }
        for s in self.streams.lock().expect("collegamenti").drain(..) {
            let _ = s.shutdown(std::net::Shutdown::Both);
        }
        link::remove_contact(&self.token);
        tracing::info!("controllo IA spento");
    }

    pub fn connected(&self) -> bool {
        self.clients.load(Ordering::Acquire) > 0
    }
}

/// Un collegamento del ponte: saluto con la chiave, poi richieste e risposte una per riga.
fn serve(stream: TcpStream, token: &str, tx: &Sender<Call>, ctx: &egui::Context, stop: &AtomicBool, clients: &AtomicUsize) {
    let _ = stream.set_nonblocking(false);
    let _ = stream.set_nodelay(true);
    let Ok(read) = stream.try_clone() else { return };
    let mut reader = BufReader::new(read);
    let mut writer = stream;
    let mut line = String::new();
    // Il saluto deve arrivare subito; senza chiave giusta si chiude.
    let _ = writer.set_read_timeout(Some(Duration::from_secs(5)));
    let hello = reader
        .read_line(&mut line)
        .ok()
        .filter(|&n| n > 0)
        .map(|_| link::check_hello(&line, token));
    let ok = matches!(hello, Some(Ok(())));
    let answer = if ok {
        serde_json::json!({"ok": true, "version": env!("CARGO_PKG_VERSION")})
    } else {
        serde_json::json!({"ok": false, "error": "chiave non valida"})
    };
    if writeln!(writer, "{answer}").is_err() || !ok {
        return;
    }
    let _ = writer.set_read_timeout(None);
    clients.fetch_add(1, Ordering::AcqRel);
    tracing::info!("controllo IA: assistente collegato");
    ctx.request_repaint();
    loop {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) | Err(_) => break,
            Ok(_) => {}
        }
        if stop.load(Ordering::Acquire) {
            break;
        }
        let result = match link::parse_request(&line) {
            Ok((name, args)) => forward(tx, ctx, &name, &args),
            Err(e) => ToolOutput::error(e),
        };
        if writeln!(writer, "{}", result.to_json()).and_then(|_| writer.flush()).is_err() {
            break;
        }
    }
    clients.fetch_sub(1, Ordering::AcqRel);
    tracing::info!("controllo IA: assistente scollegato");
    ctx.request_repaint();
}

/// Passa la chiamata alla finestra e ne aspetta il risultato.
fn forward(tx: &Sender<Call>, ctx: &egui::Context, name: &str, args: &Value) -> ToolOutput {
    let (rtx, rrx) = channel();
    let picked = Arc::new(AtomicBool::new(false));
    let call = Call {
        name: name.to_string(),
        args: args.clone(),
        picked: picked.clone(),
        reply: rtx,
    };
    if tx.send(call).is_err() {
        return ToolOutput::error("l'utente ha spento \"Controllo IA\"");
    }
    ctx.request_repaint();
    let start = Instant::now();
    loop {
        match rrx.recv_timeout(Duration::from_millis(500)) {
            Ok(out) => return out,
            Err(RecvTimeoutError::Timeout) => {
                if !picked.load(Ordering::Acquire) && start.elapsed() > PICKUP_TIMEOUT {
                    return ToolOutput::error(
                        "ForgeRender non risponde: la finestra è ridotta a icona o occupata da una finestra di dialogo. \
                         Chiedi all'utente di riportarla in primo piano e riprova.",
                    );
                }
                ctx.request_repaint();
            }
            Err(RecvTimeoutError::Disconnected) => return ToolOutput::error("l'utente ha spento \"Controllo IA\""),
        }
    }
}

/// Esegue le chiamate arrivate e fa avanzare quella in attesa (a ogni fotogramma).
pub fn process(app: &mut ForgeApp, ctx: &egui::Context) {
    tools::advance_render_job(app);
    let Some(mut b) = app.mcp.take() else { return };
    if let Some((reply, mut w)) = b.waiting.take() {
        match (w.poll)(app) {
            Some(out) => {
                let _ = reply.send(out);
            }
            None if Instant::now() > w.deadline => {
                let _ = reply.send(ToolOutput::error("tempo scaduto: l'operazione è ancora in corso nella finestra"));
            }
            None => b.waiting = Some((reply, w)),
        }
    }
    while b.waiting.is_none() {
        let Ok(call) = b.rx.try_recv() else { break };
        call.picked.store(true, Ordering::Release);
        tracing::info!("controllo IA: {}", call.name);
        b.calls += 1;
        b.last = Some(call.name.clone());
        match tools::call(app, &call.name, &call.args) {
            tools::Step::Done(out) => {
                let _ = call.reply.send(out);
            }
            tools::Step::Wait(w) => b.waiting = Some((call.reply, w)),
        }
    }
    if b.waiting.is_some() {
        ctx.request_repaint_after(Duration::from_millis(100));
    }
    app.mcp = Some(b);
}

/// Accende o spegne "Controllo IA" (pulsante nella barra, `--controllo-ia`).
pub fn set_enabled(app: &mut ForgeApp, ctx: &egui::Context, on: bool) {
    match (on, app.mcp.take()) {
        (true, Some(s)) => app.mcp = Some(s),
        (true, None) => match start(ctx.clone()) {
            Ok(s) => app.mcp = Some(s),
            Err(e) => app.error = Some(e),
        },
        (false, Some(s)) => s.shutdown(),
        (false, None) => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Giro completo senza finestra: il ponte si collega con la chiave del file di contatto,
    /// manda un comando, la "finestra" (qui il test) risponde. Chiave sbagliata: rifiutato.
    /// Spegnendo, il file sparisce e il ponte riceve "non collegato".
    #[test]
    fn bridge_talks_to_the_window() {
        let file = std::env::temp_dir().join(format!("forge-ia-{}.json", std::process::id()));
        // SAFETY: unico test che usa questa variabile; nessun altro thread la legge in parallelo.
        std::env::set_var(link::ENV_CONTACT, &file);
        let ctx = egui::Context::default();
        let server = start(ctx).unwrap();
        assert!(file.exists());
        let contact: link::Contact = serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
        let rx = &server.rx;
        let bridge = std::thread::spawn(|| {
            let mut c = link::Client::connect().expect("collegamento");
            assert!(c.is_alive());
            c.call("scene_info", &serde_json::json!({"max_components": 1})).unwrap()
        });
        let call = rx.recv_timeout(Duration::from_secs(10)).expect("chiamata");
        assert_eq!(call.name, "scene_info");
        assert_eq!(call.args["max_components"], 1);
        call.reply.send(ToolOutput::ok(serde_json::json!({"ok": 1}))).unwrap();
        let result = bridge.join().unwrap();
        assert_eq!(result["isError"], false);
        assert!(result["content"][0]["text"].as_str().unwrap().contains("ok"));
        assert!(server.connected() || server.clients.load(Ordering::Acquire) == 0);
        // Chiave sbagliata.
        let mut s = TcpStream::connect(("127.0.0.1", contact.port)).unwrap();
        writeln!(s, "{}", serde_json::json!({"token": "sbagliata"})).unwrap();
        let mut answer = String::new();
        BufReader::new(s).read_line(&mut answer).unwrap();
        assert!(answer.contains("\"ok\":false"), "{answer}");
        server.shutdown();
        assert!(!file.exists(), "file di contatto tolto");
        let err = link::Client::connect().err().expect("spento");
        assert!(err.contains("Controllo IA"), "{err}");
        std::env::remove_var(link::ENV_CONTACT);
    }
}
