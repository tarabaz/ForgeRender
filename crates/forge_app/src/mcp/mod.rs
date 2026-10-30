//! Controllo di ForgeRender da un assistente IA (Claude Code, Claude Desktop…) con il
//! Model Context Protocol. `forgerender --mcp` apre la finestra normale e in più legge le
//! richieste JSON-RPC da stdin e risponde su stdout. Guida: `docs/MCP.md`.
//!
//! Un thread legge stdin e risponde da solo a inizializzazione ed elenco degli strumenti;
//! le chiamate agli strumenti passano alla finestra con un canale e si eseguono all'inizio
//! del fotogramma ([`process`]), come un clic sui pulsanti: niente accessi concorrenti alla
//! scena. Import, apertura dei progetti e render finiscono nei fotogrammi successivi.

pub mod ops;
pub mod rpc;
pub mod tools;

use crate::app::ForgeApp;
use eframe::egui;
use rpc::ToolOutput;
use serde_json::Value;
use std::io::{BufRead, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError, Sender};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Chiamata di uno strumento in attesa della finestra.
pub struct Call {
    name: String,
    args: Value,
    /// La finestra l'ha presa in carico (per distinguere "lenta" da "ferma").
    picked: Arc<AtomicBool>,
    reply: Sender<ToolOutput>,
}

/// Lato finestra del collegamento.
pub struct Bridge {
    rx: Receiver<Call>,
    waiting: Option<(Sender<ToolOutput>, tools::Waiter)>,
    /// Stdin ancora aperto (l'assistente è collegato).
    pub connected: Arc<AtomicBool>,
    /// Strumenti eseguiti e ultimo nome (per l'indicatore nella barra).
    pub calls: u64,
    pub last: Option<String>,
}

/// Se la finestra non prende la chiamata entro questo tempo, si risponde con un errore.
const PICKUP_TIMEOUT: Duration = Duration::from_secs(20);

/// Avvia il thread che legge stdin. `ctx` serve a svegliare la finestra a ogni richiesta.
pub fn start(ctx: egui::Context) -> Bridge {
    let (tx, rx) = channel::<Call>();
    let connected = Arc::new(AtomicBool::new(true));
    let conn = connected.clone();
    std::thread::Builder::new()
        .name("forge-mcp".into())
        .spawn(move || {
            let tools = tools::list();
            let stdin = std::io::stdin();
            let mut stdout = std::io::stdout();
            for line in stdin.lock().lines() {
                let Ok(line) = line else { break };
                if line.trim().is_empty() {
                    continue;
                }
                let mut call = |name: &str, args: &Value| forward(&tx, &ctx, name, args);
                if let Some(resp) = rpc::handle_line(&line, &tools, &mut call) {
                    let ok = writeln!(stdout, "{resp}").and_then(|_| stdout.flush());
                    if ok.is_err() {
                        break;
                    }
                }
            }
            conn.store(false, Ordering::Release);
            tracing::info!("MCP: assistente scollegato (stdin chiuso)");
            ctx.request_repaint();
        })
        .expect("thread MCP");
    tracing::info!("MCP attivo: ForgeRender riceve comandi su stdin/stdout");
    Bridge {
        rx,
        waiting: None,
        connected,
        calls: 0,
        last: None,
    }
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
        return ToolOutput::error("la finestra di ForgeRender è stata chiusa");
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
                         Riportala in primo piano e riprova.",
                    );
                }
                ctx.request_repaint();
            }
            Err(RecvTimeoutError::Disconnected) => return ToolOutput::error("la finestra di ForgeRender è stata chiusa"),
        }
    }
}

/// Esegue le chiamate arrivate e fa avanzare quella in attesa (a ogni fotogramma).
pub fn process(app: &mut ForgeApp, ctx: &egui::Context) {
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
        tracing::info!("MCP: {}", call.name);
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
    // Scollegato senza aver mai ricevuto comandi (per esempio la verifica di
    // `claude mcp list`): la finestra si chiude da sola. Dopo dei comandi resta aperta.
    if b.calls == 0 && !b.connected.load(Ordering::Acquire) {
        tracing::info!("MCP: solo verifica del collegamento, chiusura della finestra");
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }
    app.mcp = Some(b);
}
