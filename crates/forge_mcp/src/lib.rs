//! Controllo di ForgeRender da un assistente IA con il Model Context Protocol (MCP).
//!
//! - [`rpc`]: protocollo MCP (JSON-RPC 2.0 su stdin/stdout), usato dal ponte `forgerender-mcp`.
//! - [`tools`]: elenco e schema degli strumenti (li esegue la finestra, `forge_app::mcp`).
//! - [`link`]: collegamento locale tra il ponte e la finestra aperta dall'utente.
//!
//! Guida: `docs/MCP.md`.

pub mod link;
pub mod rpc;
pub mod tools;
