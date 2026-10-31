//! Collegamento degli assistenti IA a ForgeRender con un clic (finestra "Integrazione AI").
//!
//! - Il ponte `forgerender-mcp.exe` viene copiato in una cartella fissa
//!   (`%LOCALAPPDATA%\ForgeRender\mcp\`): le registrazioni puntano sempre lì e restano valide
//!   quando si aggiorna o si sposta ForgeRender.
//! - Per ogni assistente si legge dove tiene la configurazione e se ForgeRender c'è già:
//!   - Claude Desktop: `claude_desktop_config.json` (installazione normale, Microsoft Store e
//!     la cartella `Claude-Data` delle versioni recenti; le cartelle lasciate hanno un file
//!     `.user-data-left…`);
//!   - Claude Code (terminale e app desktop, stessa registrazione utente): il comando
//!     `claude mcp add --scope user`, oppure, senza il comando, `~/.claude.json`;
//!   - Codex (OpenAI): `~/.codex/config.toml`;
//!   - ChatGPT: collega solo server remoti su internet, non programmi del PC: non supportato.
//! - Prima di modificare un file se ne fa una copia (`.backup-forgerender`), la scrittura è
//!   atomica e si tocca solo la voce `forgerender`; un file rovinato non si modifica.

use serde_json::{json, Value};
use std::path::{Path, PathBuf};

/// Nome del server nelle configurazioni degli assistenti.
pub const SERVER: &str = "forgerender";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Assistant {
    ClaudeDesktop,
    ClaudeCode,
    Codex,
    ChatGpt,
}

impl Assistant {
    pub const ALL: [Assistant; 4] = [
        Assistant::ClaudeDesktop,
        Assistant::ClaudeCode,
        Assistant::Codex,
        Assistant::ChatGpt,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Assistant::ClaudeDesktop => "Claude Desktop",
            Assistant::ClaudeCode => "Claude Code",
            Assistant::Codex => "Codex (OpenAI)",
            Assistant::ChatGpt => "ChatGPT",
        }
    }

    /// Assistente dal nome con cui si presenta al ponte (`clientInfo.name` dell'MCP).
    pub fn from_client_name(name: &str) -> Option<Assistant> {
        let n = name.to_lowercase();
        if n.contains("claude-code") || n.contains("claude code") {
            Some(Assistant::ClaudeCode)
        } else if n.contains("claude") {
            Some(Assistant::ClaudeDesktop)
        } else if n.contains("codex") {
            Some(Assistant::Codex)
        } else if n.contains("chatgpt") || n.contains("openai") {
            Some(Assistant::ChatGpt)
        } else {
            None
        }
    }
}

/// Stato della configurazione di un assistente.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum State {
    /// L'assistente non è sul PC.
    NotInstalled,
    NotConfigured,
    /// Registrato, ma con un ponte diverso da quello della cartella fissa (es. una portable vecchia).
    Outdated(String),
    Configured,
    /// L'assistente non può collegare programmi del PC.
    Unsupported,
    /// File di configurazione illeggibile: non lo si tocca.
    Broken(String),
}

/// Cartelle dell'utente (sostituibili nei test).
#[derive(Clone, Debug)]
pub struct Env {
    pub home: PathBuf,
    pub appdata: PathBuf,
    pub local: PathBuf,
}

impl Env {
    pub fn current() -> Option<Env> {
        let var = |k: &str| std::env::var_os(k).map(PathBuf::from);
        let home = var("USERPROFILE").or_else(|| var("HOME"))?;
        Some(Env {
            appdata: var("APPDATA").unwrap_or_else(|| home.join("AppData").join("Roaming")),
            local: var("LOCALAPPDATA").unwrap_or_else(|| home.join("AppData").join("Local")),
            home,
        })
    }

    /// Ponte nella cartella fissa.
    pub fn bridge(&self) -> PathBuf {
        self.local.join("ForgeRender").join("mcp").join(bridge_file_name())
    }
}

fn bridge_file_name() -> &'static str {
    if cfg!(windows) {
        "forgerender-mcp.exe"
    } else {
        "forgerender-mcp"
    }
}

/// Ponte accanto al programma in esecuzione.
pub fn bridge_source() -> Option<PathBuf> {
    let p = std::env::current_exe().ok()?.parent()?.join(bridge_file_name());
    p.is_file().then_some(p)
}

/// Copia il ponte nella cartella fissa se manca o è diverso. Se è in uso (un assistente è
/// collegato) resta quello vecchio: si riprova al prossimo avvio. Restituisce il percorso fisso.
pub fn install_bridge(env: &Env, source: &Path) -> Result<PathBuf, String> {
    let target = env.bridge();
    let new = std::fs::read(source).map_err(|e| format!("ponte non leggibile ({}): {e}", source.display()))?;
    if std::fs::read(&target).is_ok_and(|old| old == new) {
        return Ok(target);
    }
    if let Some(d) = target.parent() {
        std::fs::create_dir_all(d).map_err(|e| format!("cartella {} non creata: {e}", d.display()))?;
    }
    let tmp = target.with_extension("nuovo");
    std::fs::write(&tmp, &new).map_err(|e| format!("ponte non copiato: {e}"))?;
    match std::fs::rename(&tmp, &target) {
        Ok(()) => Ok(target),
        Err(e) => {
            let _ = std::fs::remove_file(&tmp);
            if target.is_file() {
                Err(format!(
                    "il ponte è in uso da un assistente collegato: resta la versione precedente finché non si chiude ({e})"
                ))
            } else {
                Err(format!("ponte non copiato: {e}"))
            }
        }
    }
}

fn same_path(a: &str, b: &Path) -> bool {
    let norm = |s: &str| s.trim().trim_matches('"').replace('/', "\\").to_lowercase();
    norm(a) == norm(&b.to_string_lossy())
}

fn state_from(registered: Option<String>, bridge: &Path) -> State {
    match registered {
        None => State::NotConfigured,
        Some(c) if same_path(&c, bridge) => State::Configured,
        Some(c) => State::Outdated(c),
    }
}

// ---------------------------------------------------------------- file JSON

/// Legge un file JSON di configurazione: assente = oggetto vuoto; non valido = errore (il file
/// non si tocca).
fn read_json(path: &Path) -> Result<Value, String> {
    match std::fs::read_to_string(path) {
        Ok(t) if t.trim().is_empty() => Ok(json!({})),
        Ok(t) => {
            let v: Value =
                serde_json::from_str(&t).map_err(|e| format!("{} non è un JSON valido ({e}): non lo modifico", path.display()))?;
            if v.is_object() {
                Ok(v)
            } else {
                Err(format!("{} non contiene un oggetto JSON: non lo modifico", path.display()))
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(json!({})),
        Err(e) => Err(format!("{} non leggibile: {e}", path.display())),
    }
}

/// Scrittura sicura: copia del file precedente, file temporaneo, poi rinomina.
fn write_text(path: &Path, text: &str) -> Result<(), String> {
    if let Some(d) = path.parent() {
        std::fs::create_dir_all(d).map_err(|e| format!("cartella {} non creata: {e}", d.display()))?;
    }
    if path.is_file() {
        let backup = PathBuf::from(format!("{}.backup-forgerender", path.display()));
        std::fs::copy(path, &backup).map_err(|e| format!("copia di sicurezza non riuscita: {e}"))?;
    }
    let tmp = PathBuf::from(format!("{}.forgerender-tmp", path.display()));
    std::fs::write(&tmp, text).map_err(|e| format!("{} non scritto: {e}", path.display()))?;
    std::fs::rename(&tmp, path).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        format!("{} non scritto: {e}", path.display())
    })
}

fn json_registered(v: &Value) -> Option<String> {
    v.get("mcpServers")?.get(SERVER)?.get("command")?.as_str().map(str::to_string)
}

fn json_set(path: &Path, bridge: &Path) -> Result<(), String> {
    let mut v = read_json(path)?;
    let obj = v.as_object_mut().expect("oggetto");
    let servers = obj.entry("mcpServers").or_insert_with(|| json!({}));
    let Some(servers) = servers.as_object_mut() else {
        return Err(format!("{}: \"mcpServers\" non è un oggetto, non lo modifico", path.display()));
    };
    servers.insert(SERVER.into(), json!({"command": bridge.to_string_lossy(), "args": []}));
    write_text(path, &(serde_json::to_string_pretty(&v).map_err(|e| e.to_string())? + "\n"))
}

fn json_remove(path: &Path) -> Result<(), String> {
    if !path.is_file() {
        return Ok(());
    }
    let mut v = read_json(path)?;
    let removed = v
        .get_mut("mcpServers")
        .and_then(|s| s.as_object_mut())
        .is_some_and(|s| s.remove(SERVER).is_some());
    if removed {
        write_text(path, &(serde_json::to_string_pretty(&v).map_err(|e| e.to_string())? + "\n"))?;
    }
    Ok(())
}

// ---------------------------------------------------------------- Claude Desktop

fn subdirs(dir: &Path, prefix: &str) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir() && p.file_name().is_some_and(|n| n.to_string_lossy().starts_with(prefix)))
        .collect();
    v.sort();
    v
}

/// Cartella lasciata da Claude Desktop quando ha spostato i dati (file `.user-data-left…`).
fn left_behind(dir: &Path) -> bool {
    std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .any(|e| e.file_name().to_string_lossy().starts_with(".user-data-left"))
}

/// File di configurazione di Claude Desktop in uso (anche se il file non c'è ancora).
pub fn desktop_configs(env: &Env) -> Vec<PathBuf> {
    let mut dirs = subdirs(&env.local.join("Claude-Data"), "claude_");
    for p in subdirs(&env.local.join("Packages"), "Claude_") {
        dirs.push(p.join("LocalCache").join("Roaming").join("Claude"));
    }
    dirs.push(env.appdata.join("Claude"));
    dirs.into_iter()
        .filter(|d| d.is_dir() && !left_behind(d))
        .map(|d| d.join("claude_desktop_config.json"))
        .collect()
}

fn desktop_installed(env: &Env) -> bool {
    !desktop_configs(env).is_empty()
        || env.local.join("AnthropicClaude").is_dir()
        || !subdirs(&env.local.join("Packages"), "Claude_").is_empty()
}

// ---------------------------------------------------------------- Claude Code

/// Il comando `claude` (Claude Code), dal PATH o dalla cartella d'installazione.
pub fn claude_cli(env: &Env) -> Option<PathBuf> {
    let names: &[&str] = if cfg!(windows) {
        &["claude.exe", "claude.cmd"]
    } else {
        &["claude"]
    };
    std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).collect::<Vec<_>>())
        .unwrap_or_default()
        .into_iter()
        .chain([env.home.join(".local").join("bin")])
        .flat_map(|d| names.iter().map(move |n| d.join(n)))
        .find(|p| p.is_file())
}

fn code_config(env: &Env) -> PathBuf {
    env.home.join(".claude.json")
}

fn code_installed(env: &Env) -> bool {
    claude_cli(env).is_some() || code_config(env).is_file() || env.home.join(".claude").is_dir()
}

/// Esegue un programma senza aprire finestre (su Windows niente console).
fn run_hidden(exe: &Path, args: &[&str]) -> Result<std::process::Output, String> {
    let mut cmd = std::process::Command::new(exe);
    cmd.args(args).stdin(std::process::Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd.output().map_err(|e| format!("{} non avviato: {e}", exe.display()))
}

// ---------------------------------------------------------------- Codex

fn codex_config(env: &Env) -> PathBuf {
    env.home.join(".codex").join("config.toml")
}

fn codex_installed(env: &Env) -> bool {
    env.home.join(".codex").is_dir()
}

/// La sezione `[mcp_servers.forgerender]` (e le sue sotto-tabelle) del file TOML.
fn toml_is_ours(header: &str) -> bool {
    let h = header.trim().trim_start_matches('[').trim_end_matches(']').trim();
    let ours = format!("mcp_servers.{SERVER}");
    h == ours || h.starts_with(&format!("{ours}.")) || h == format!("mcp_servers.\"{SERVER}\"")
}

/// Toglie la nostra sezione dal testo TOML, lasciando il resto com'è.
fn toml_without(text: &str) -> String {
    let mut out = Vec::new();
    let mut skipping = false;
    for line in text.lines() {
        let t = line.trim_start();
        if t.starts_with('[') {
            skipping = toml_is_ours(t.split('#').next().unwrap_or(t));
        }
        if !skipping {
            out.push(line);
        }
    }
    let mut s = out.join("\n");
    while s.ends_with("\n\n") {
        s.pop();
    }
    if !s.is_empty() && !s.ends_with('\n') {
        s.push('\n');
    }
    s
}

fn toml_registered(text: &str) -> Option<String> {
    let mut inside = false;
    for line in text.lines() {
        let t = line.trim();
        if t.starts_with('[') {
            inside = t == format!("[mcp_servers.{SERVER}]");
            continue;
        }
        if inside {
            if let Some(v) = t.strip_prefix("command").map(str::trim_start).and_then(|r| r.strip_prefix('=')) {
                let v = v.trim();
                let unq = if v.starts_with('\'') {
                    v.trim_matches('\'').to_string()
                } else {
                    v.trim_matches('"').replace("\\\\", "\\")
                };
                return Some(unq);
            }
        }
    }
    None
}

fn toml_with(text: &str, bridge: &Path) -> String {
    let mut s = toml_without(text);
    if !s.is_empty() {
        s.push('\n');
    }
    // Stringa letterale TOML (apici singoli): nessun carattere di escape nei percorsi Windows.
    s.push_str(&format!(
        "[mcp_servers.{SERVER}]\ncommand = '{}'\nargs = []\n",
        bridge.to_string_lossy()
    ));
    s
}

// ---------------------------------------------------------------- operazioni

/// Stato attuale dell'assistente.
pub fn state(env: &Env, a: Assistant) -> State {
    let bridge = env.bridge();
    match a {
        Assistant::ChatGpt => State::Unsupported,
        Assistant::ClaudeDesktop => {
            if !desktop_installed(env) {
                return State::NotInstalled;
            }
            let configs = desktop_configs(env);
            let mut found = None;
            for c in &configs {
                match read_json(c) {
                    Ok(v) => {
                        if let Some(r) = json_registered(&v) {
                            found = Some(r);
                        }
                    }
                    Err(e) => return State::Broken(e),
                }
            }
            state_from(found, &bridge)
        }
        Assistant::ClaudeCode => {
            if !code_installed(env) {
                return State::NotInstalled;
            }
            match read_json(&code_config(env)) {
                Ok(v) => state_from(json_registered(&v), &bridge),
                Err(e) => State::Broken(e),
            }
        }
        Assistant::Codex => {
            if !codex_installed(env) {
                return State::NotInstalled;
            }
            let text = std::fs::read_to_string(codex_config(env)).unwrap_or_default();
            state_from(toml_registered(&text), &bridge)
        }
    }
}

/// Collega l'assistente al ponte della cartella fissa. Restituisce il passo successivo da
/// dire all'utente (riavvio, sessione nuova).
pub fn connect(env: &Env, a: Assistant) -> Result<String, String> {
    let bridge = env.bridge();
    if !bridge.is_file() {
        return Err("ponte non installato nella cartella fissa: riapri ForgeRender".into());
    }
    match a {
        Assistant::ChatGpt => Err("ChatGPT collega solo server remoti su internet, non programmi del PC".into()),
        Assistant::ClaudeDesktop => {
            let configs = desktop_configs(env);
            if configs.is_empty() {
                return Err("cartella delle impostazioni di Claude Desktop non trovata: aprilo una volta e riprova".into());
            }
            for c in &configs {
                json_set(c, &bridge)?;
            }
            Ok(crate::i18n::tk("Riavvia Claude Desktop (serve solo questa volta): poi ogni chat vede ForgeRender.").into())
        }
        Assistant::ClaudeCode => {
            match claude_cli(env) {
                Some(cli) => {
                    let _ = run_hidden(&cli, &["mcp", "remove", "--scope", "user", SERVER]);
                    let bridge_s = bridge.to_string_lossy().to_string();
                    let out = run_hidden(&cli, &["mcp", "add", "--scope", "user", SERVER, "--", &bridge_s])?;
                    if !out.status.success() {
                        return Err(format!(
                            "claude mcp add non riuscito: {}",
                            String::from_utf8_lossy(&out.stderr).trim()
                        ));
                    }
                }
                None => json_set(&code_config(env), &bridge)?,
            }
            if state(env, a) != State::Configured {
                return Err("la registrazione non risulta: riprova con Claude Code chiuso".into());
            }
            Ok(crate::i18n::tk("Apri una sessione nuova di Claude Code (quelle già aperte non lo vedono).").into())
        }
        Assistant::Codex => {
            let path = codex_config(env);
            let text = std::fs::read_to_string(&path).unwrap_or_default();
            write_text(&path, &toml_with(&text, &bridge))?;
            Ok(crate::i18n::tk("Apri una sessione nuova di Codex.").into())
        }
    }
}

/// Toglie ForgeRender dall'assistente (le altre voci restano).
pub fn disconnect(env: &Env, a: Assistant) -> Result<(), String> {
    match a {
        Assistant::ChatGpt => Ok(()),
        Assistant::ClaudeDesktop => desktop_configs(env).iter().try_for_each(|c| json_remove(c)),
        Assistant::ClaudeCode => match claude_cli(env) {
            Some(cli) => {
                let _ = run_hidden(&cli, &["mcp", "remove", "--scope", "user", SERVER])?;
                json_remove(&code_config(env))
            }
            None => json_remove(&code_config(env)),
        },
        Assistant::Codex => {
            let path = codex_config(env);
            match std::fs::read_to_string(&path) {
                Ok(text) if toml_registered(&text).is_some() => write_text(&path, &toml_without(&text)),
                _ => Ok(()),
            }
        }
    }
}

/// Claude Desktop è aperto? (Processi dell'app, non quelli di Claude Code.)
#[cfg(windows)]
pub fn claude_desktop_running() -> bool {
    let ps = "Get-Process claude -ErrorAction SilentlyContinue | Where-Object { $_.Path -like '*\\WindowsApps\\Claude_*' -or $_.Path -like '*\\AnthropicClaude\\*' } | Select-Object -First 1 | ForEach-Object { 'si' }";
    run_hidden(Path::new("powershell"), &["-NoProfile", "-Command", ps])
        .map(|o| String::from_utf8_lossy(&o.stdout).contains("si"))
        .unwrap_or(false)
}

#[cfg(not(windows))]
pub fn claude_desktop_running() -> bool {
    false
}

/// Chiude Claude Desktop (solo l'app, non i Claude Code del terminale) e lo riapre.
#[cfg(windows)]
pub fn restart_claude_desktop() -> Result<(), String> {
    let ps = "$p = Get-Process claude -ErrorAction SilentlyContinue | Where-Object { $_.Path -like '*\\WindowsApps\\Claude_*' -or $_.Path -like '*\\AnthropicClaude\\*' }; \
              $p | Stop-Process -Force -ErrorAction SilentlyContinue; Start-Sleep -Milliseconds 1500; \
              $app = Get-StartApps | Where-Object { $_.Name -eq 'Claude' } | Select-Object -First 1; \
              if ($app) { Start-Process ('shell:AppsFolder\\' + $app.AppID) } \
              elseif (Test-Path \"$env:LOCALAPPDATA\\AnthropicClaude\\claude.exe\") { Start-Process \"$env:LOCALAPPDATA\\AnthropicClaude\\claude.exe\" } \
              else { exit 3 }";
    let out = run_hidden(Path::new("powershell"), &["-NoProfile", "-Command", ps])?;
    if out.status.success() {
        Ok(())
    } else {
        Err("Claude Desktop chiuso: riaprilo dal menu Start".into())
    }
}

#[cfg(not(windows))]
pub fn restart_claude_desktop() -> Result<(), String> {
    Err("riavvia Claude Desktop a mano".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_env(tag: &str) -> Env {
        let root = std::env::temp_dir().join(format!("forge_ai_setup_{tag}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let env = Env {
            home: root.join("home"),
            appdata: root.join("roaming"),
            local: root.join("local"),
        };
        std::fs::create_dir_all(&env.home).unwrap();
        std::fs::create_dir_all(&env.appdata).unwrap();
        std::fs::create_dir_all(&env.local).unwrap();
        env
    }

    fn fake_bridge(env: &Env) -> PathBuf {
        let src = env.home.join("portable").join(bridge_file_name());
        std::fs::create_dir_all(src.parent().unwrap()).unwrap();
        std::fs::write(&src, b"ponte v1").unwrap();
        install_bridge(env, &src).unwrap()
    }

    /// Il ponte va nella cartella fissa; se cambia si aggiorna, se è uguale non si riscrive.
    #[test]
    fn bridge_is_copied_to_the_fixed_folder() {
        let env = fake_env("ponte");
        let p = fake_bridge(&env);
        assert_eq!(p, env.bridge());
        assert_eq!(std::fs::read(&p).unwrap(), b"ponte v1");
        let src = env.home.join("nuova").join(bridge_file_name());
        std::fs::create_dir_all(src.parent().unwrap()).unwrap();
        std::fs::write(&src, b"ponte v2").unwrap();
        install_bridge(&env, &src).unwrap();
        assert_eq!(std::fs::read(&p).unwrap(), b"ponte v2");
    }

    /// Claude Desktop della versione recente (Claude-Data) con la cartella vecchia lasciata:
    /// si scrive solo nel file in uso, si conservano le altre voci, si fa la copia di sicurezza,
    /// e "Scollega" toglie solo la nostra voce.
    #[test]
    fn claude_desktop_config_is_merged_and_backed_up() {
        let env = fake_env("desktop");
        assert_eq!(state(&env, Assistant::ClaudeDesktop), State::NotInstalled);
        let data = env.local.join("Claude-Data").join("claude_abc123");
        std::fs::create_dir_all(&data).unwrap();
        let old = env.appdata.join("Claude");
        std::fs::create_dir_all(&old).unwrap();
        std::fs::write(old.join(".user-data-left.claude_abc123"), "").unwrap();
        let cfg = data.join("claude_desktop_config.json");
        std::fs::write(&cfg, r#"{"preferences":{"x":1},"mcpServers":{"altro":{"command":"a.exe"}}}"#).unwrap();
        assert_eq!(desktop_configs(&env), vec![cfg.clone()]);
        let bridge = fake_bridge(&env);
        assert_eq!(state(&env, Assistant::ClaudeDesktop), State::NotConfigured);
        connect(&env, Assistant::ClaudeDesktop).unwrap();
        assert_eq!(state(&env, Assistant::ClaudeDesktop), State::Configured);
        let v: Value = serde_json::from_str(&std::fs::read_to_string(&cfg).unwrap()).unwrap();
        assert_eq!(v["preferences"]["x"], 1);
        assert_eq!(v["mcpServers"]["altro"]["command"], "a.exe");
        assert_eq!(v["mcpServers"]["forgerender"]["command"], bridge.to_string_lossy().as_ref());
        assert!(PathBuf::from(format!("{}.backup-forgerender", cfg.display())).is_file());
        assert!(!old.join("claude_desktop_config.json").exists(), "cartella lasciata intatta");
        disconnect(&env, Assistant::ClaudeDesktop).unwrap();
        let v: Value = serde_json::from_str(&std::fs::read_to_string(&cfg).unwrap()).unwrap();
        assert!(v["mcpServers"].get("forgerender").is_none());
        assert_eq!(v["mcpServers"]["altro"]["command"], "a.exe");
    }

    /// Un file rovinato non si modifica; un percorso vecchio risulta "da aggiornare".
    #[test]
    fn broken_and_outdated_configurations() {
        let env = fake_env("rotti");
        let dir = env.appdata.join("Claude");
        std::fs::create_dir_all(&dir).unwrap();
        let cfg = dir.join("claude_desktop_config.json");
        std::fs::write(&cfg, "{ rotto").unwrap();
        fake_bridge(&env);
        assert!(matches!(state(&env, Assistant::ClaudeDesktop), State::Broken(_)));
        assert!(connect(&env, Assistant::ClaudeDesktop).is_err());
        assert_eq!(std::fs::read_to_string(&cfg).unwrap(), "{ rotto");
        std::fs::write(
            &cfg,
            r#"{"mcpServers":{"forgerender":{"command":"C:\\vecchia\\forgerender-mcp.exe"}}}"#,
        )
        .unwrap();
        assert_eq!(
            state(&env, Assistant::ClaudeDesktop),
            State::Outdated("C:\\vecchia\\forgerender-mcp.exe".into())
        );
    }

    /// Claude Code senza il comando `claude` sul PC: si scrive `~/.claude.json` conservando il
    /// resto (qui il PATH dei test non ha `claude` nella cartella finta).
    #[test]
    fn claude_code_without_cli_uses_the_user_config() {
        let env = fake_env("code");
        std::fs::write(
            env.home.join(".claude.json"),
            r#"{"numStartups":3,"mcpServers":{"rea":{"command":"r.exe"}}}"#,
        )
        .unwrap();
        fake_bridge(&env);
        if claude_cli(&env).is_some() {
            // Su un PC con Claude Code installato il test userebbe il comando vero: si salta.
            return;
        }
        assert_eq!(state(&env, Assistant::ClaudeCode), State::NotConfigured);
        connect(&env, Assistant::ClaudeCode).unwrap();
        let v: Value = serde_json::from_str(&std::fs::read_to_string(env.home.join(".claude.json")).unwrap()).unwrap();
        assert_eq!(v["numStartups"], 3);
        assert_eq!(v["mcpServers"]["rea"]["command"], "r.exe");
        assert_eq!(state(&env, Assistant::ClaudeCode), State::Configured);
    }

    /// Codex: la sezione TOML si aggiunge, si ritrova e si toglie lasciando intatte le altre
    /// (comprese le sotto-tabelle degli altri server).
    #[test]
    fn codex_toml_section() {
        let env = fake_env("codex");
        let dir = env.home.join(".codex");
        std::fs::create_dir_all(&dir).unwrap();
        let original = "model = \"gpt-5\"\n\n[mcp_servers.node_repl]\ncommand = \"node\"\n\n[mcp_servers.node_repl.env]\nA = \"1\"\n";
        std::fs::write(dir.join("config.toml"), original).unwrap();
        let bridge = fake_bridge(&env);
        assert_eq!(state(&env, Assistant::Codex), State::NotConfigured);
        connect(&env, Assistant::Codex).unwrap();
        let text = std::fs::read_to_string(dir.join("config.toml")).unwrap();
        assert!(text.starts_with(original.trim_end()), "{text}");
        assert_eq!(toml_registered(&text).as_deref(), Some(bridge.to_string_lossy().as_ref()));
        assert_eq!(state(&env, Assistant::Codex), State::Configured);
        // Collegare di nuovo non duplica la sezione.
        connect(&env, Assistant::Codex).unwrap();
        let text = std::fs::read_to_string(dir.join("config.toml")).unwrap();
        assert_eq!(text.matches("[mcp_servers.forgerender]").count(), 1);
        disconnect(&env, Assistant::Codex).unwrap();
        let text = std::fs::read_to_string(dir.join("config.toml")).unwrap();
        assert_eq!(text, original);
    }

    #[test]
    fn assistants_from_client_names() {
        assert_eq!(Assistant::from_client_name("claude-code"), Some(Assistant::ClaudeCode));
        assert_eq!(Assistant::from_client_name("claude-ai"), Some(Assistant::ClaudeDesktop));
        assert_eq!(Assistant::from_client_name("codex-mcp-client"), Some(Assistant::Codex));
        assert_eq!(Assistant::from_client_name("qualcos'altro"), None);
        assert_eq!(state(&fake_env("chatgpt"), Assistant::ChatGpt), State::Unsupported);
    }
}
