//! Protocollo MCP (Model Context Protocol) su stdin/stdout: JSON-RPC 2.0, un messaggio per
//! riga. Qui solo il protocollo; gli strumenti li esegue la finestra (vedi `mcp::tools`).

use serde_json::{json, Value};

/// Versioni del protocollo che il server accetta (la prima è quella proposta).
const VERSIONS: [&str; 3] = ["2025-06-18", "2025-03-26", "2024-11-05"];

/// Risultato di uno strumento: testo (di solito JSON) e, se c'è, un'immagine PNG.
#[derive(Clone, Debug, Default)]
pub struct ToolOutput {
    pub text: String,
    pub png: Option<Vec<u8>>,
    pub is_error: bool,
}

impl ToolOutput {
    pub fn ok(mut v: Value) -> Self {
        round_floats(&mut v);
        Self {
            text: serde_json::to_string_pretty(&v).unwrap_or_default(),
            ..Default::default()
        }
    }

    pub fn error(msg: impl Into<String>) -> Self {
        Self {
            text: msg.into(),
            is_error: true,
            ..Default::default()
        }
    }

    fn to_json(&self) -> Value {
        let mut content = vec![json!({"type": "text", "text": self.text})];
        if let Some(png) = &self.png {
            content.push(json!({"type": "image", "data": base64(png), "mimeType": "image/png"}));
        }
        json!({"content": content, "isError": self.is_error})
    }
}

/// Numeri decimali a 3 cifre (gli f32 convertiti mostrerebbero 0.3499999940395355).
fn round_floats(v: &mut Value) {
    match v {
        Value::Number(n) if n.is_f64() => {
            let x = (n.as_f64().unwrap_or(0.0) * 1000.0).round() / 1000.0;
            if let Some(r) = serde_json::Number::from_f64(x) {
                *n = r;
            }
        }
        Value::Array(a) => a.iter_mut().for_each(round_floats),
        Value::Object(o) => o.values_mut().for_each(round_floats),
        _ => {}
    }
}

/// Istruzioni per l'assistente, inviate all'inizializzazione.
const INSTRUCTIONS: &str = "ForgeRender è un renderer fotorealistico per foto di prodotto. \
Prima di modificare chiama scene_info: misure in metri, y verso l'alto. \
Flusso tipico: import_model → prepare_studio (aspetto 'studio') → add_light / edit_light → \
apply_material o create_material → set_camera → set_render_settings → render (restituisce \
l'immagine: guardala e correggi). \"Destra\", \"sinistra\", \"davanti\" sono rispetto \
all'inquadratura attuale. Ogni modifica si vede subito nella finestra di ForgeRender. \
Non sovrascrivere file esistenti senza che l'utente lo chieda (overwrite).";

/// Gestisce una riga ricevuta. Restituisce la risposta da scrivere (nessuna per le notifiche).
/// `call` esegue uno strumento.
pub fn handle_line(line: &str, tools: &Value, call: &mut dyn FnMut(&str, &Value) -> ToolOutput) -> Option<String> {
    let msg: Value = match serde_json::from_str(line) {
        Ok(v) => v,
        Err(e) => return Some(error_response(Value::Null, -32700, &format!("JSON non valido: {e}"))),
    };
    let id = msg.get("id").cloned();
    let method = msg.get("method").and_then(Value::as_str).unwrap_or("");
    let params = msg.get("params").cloned().unwrap_or(Value::Null);
    // Notifiche (senza id): nessuna risposta.
    let id = id?;
    let result = match method {
        "initialize" => {
            let asked = params.get("protocolVersion").and_then(Value::as_str).unwrap_or(VERSIONS[0]);
            let version = VERSIONS.iter().find(|v| **v == asked).copied().unwrap_or(VERSIONS[0]);
            json!({
                "protocolVersion": version,
                "capabilities": {"tools": {"listChanged": false}},
                "serverInfo": {"name": "forgerender", "title": "ForgeRender", "version": env!("CARGO_PKG_VERSION")},
                "instructions": INSTRUCTIONS,
            })
        }
        "ping" => json!({}),
        "tools/list" => json!({"tools": tools}),
        "resources/list" => json!({"resources": []}),
        "prompts/list" => json!({"prompts": []}),
        "tools/call" => {
            let Some(name) = params.get("name").and_then(Value::as_str) else {
                return Some(error_response(id, -32602, "manca il nome dello strumento"));
            };
            let known = tools.as_array().is_some_and(|a| a.iter().any(|t| t["name"] == name));
            if !known {
                return Some(error_response(id, -32602, &format!("strumento sconosciuto: {name}")));
            }
            let args = params.get("arguments").cloned().unwrap_or_else(|| json!({}));
            call(name, &args).to_json()
        }
        _ => return Some(error_response(id, -32601, &format!("metodo non supportato: {method}"))),
    };
    Some(json!({"jsonrpc": "2.0", "id": id, "result": result}).to_string())
}

fn error_response(id: Value, code: i64, message: &str) -> String {
    json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}}).to_string()
}

/// Base64 standard (per le immagini nelle risposte).
pub fn base64(data: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for c in data.chunks(3) {
        let n = (c[0] as u32) << 16 | (*c.get(1).unwrap_or(&0) as u32) << 8 | *c.get(2).unwrap_or(&0) as u32;
        for k in 0..4 {
            if k <= c.len() {
                out.push(T[(n >> (18 - 6 * k) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tools() -> Value {
        json!([{"name": "scene_info", "description": "x", "inputSchema": {"type": "object"}}])
    }

    #[test]
    fn handshake_list_and_call() {
        let t = tools();
        let mut calls = Vec::new();
        let mut call = |n: &str, a: &Value| {
            calls.push((n.to_string(), a.clone()));
            ToolOutput::ok(json!({"ok": true}))
        };
        let r = handle_line(
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{}}}"#,
            &t,
            &mut call,
        )
        .unwrap();
        let v: Value = serde_json::from_str(&r).unwrap();
        assert_eq!(v["id"], 1);
        assert_eq!(v["result"]["protocolVersion"], "2025-03-26");
        assert!(v["result"]["capabilities"]["tools"].is_object());
        // Notifica: nessuna risposta.
        assert!(handle_line(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#, &t, &mut call).is_none());
        let r = handle_line(r#"{"jsonrpc":"2.0","id":"a","method":"tools/list"}"#, &t, &mut call).unwrap();
        assert!(r.contains("scene_info"));
        let r = handle_line(
            r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"scene_info","arguments":{"x":1}}}"#,
            &t,
            &mut call,
        )
        .unwrap();
        let v: Value = serde_json::from_str(&r).unwrap();
        assert_eq!(v["result"]["isError"], false);
        assert!(v["result"]["content"][0]["text"].as_str().unwrap().contains("ok"));
        assert_eq!(calls, vec![("scene_info".to_string(), json!({"x": 1}))]);
    }

    #[test]
    fn errors_are_reported() {
        let t = tools();
        let mut call = |_: &str, _: &Value| ToolOutput::error("no");
        let v: Value = serde_json::from_str(&handle_line("{rotto", &t, &mut call).unwrap()).unwrap();
        assert_eq!(v["error"]["code"], -32700);
        let v: Value = serde_json::from_str(&handle_line(r#"{"jsonrpc":"2.0","id":3,"method":"boh"}"#, &t, &mut call).unwrap()).unwrap();
        assert_eq!(v["error"]["code"], -32601);
        let r = handle_line(
            r#"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"inesistente"}}"#,
            &t,
            &mut call,
        )
        .unwrap();
        assert!(r.contains("-32602"));
        let r = handle_line(
            r#"{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"scene_info"}}"#,
            &t,
            &mut call,
        )
        .unwrap();
        let v: Value = serde_json::from_str(&r).unwrap();
        assert_eq!(v["result"]["isError"], true);
    }

    #[test]
    fn base64_matches_reference() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
        assert_eq!(base64(&[0xff, 0xfe, 0x00]), "//4A");
        let out = ToolOutput {
            png: Some(vec![1, 2, 3]),
            ..ToolOutput::ok(json!({}))
        };
        assert_eq!(out.to_json()["content"][1]["type"], "image");
        let t = ToolOutput::ok(json!({"r": 0.35f32, "v": [1.23456789, 2], "n": 7}));
        assert!(
            t.text.contains("0.35") && !t.text.contains("0.349") && t.text.contains("1.235"),
            "{}",
            t.text
        );
    }
}
