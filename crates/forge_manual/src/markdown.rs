//! Markdown → blocchi semplici da disegnare (titoli, paragrafi, elenchi, tabelle, immagini,
//! codice). Si usa solo il parser di `pulldown-cmark`: il disegno è nell'app.

use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};

/// Pezzo di testo con il suo stile.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Span {
    pub text: String,
    pub bold: bool,
    pub italic: bool,
    pub code: bool,
    /// Destinazione del collegamento (id di pagina o indirizzo web).
    pub link: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Block {
    Heading {
        level: u8,
        spans: Vec<Span>,
    },
    Paragraph(Vec<Span>),
    /// Elenco: `start` = numero del primo elemento se numerato.
    List {
        start: Option<u64>,
        items: Vec<Vec<Span>>,
    },
    Quote(Vec<Span>),
    Code(String),
    Table {
        header: Vec<Vec<Span>>,
        rows: Vec<Vec<Vec<Span>>>,
    },
    Image {
        src: String,
        caption: String,
    },
    Rule,
}

impl Block {
    /// Chiama `f` per ogni collegamento del blocco.
    pub fn visit_links(&self, f: &mut impl FnMut(&str)) {
        let mut spans = |v: &[Span]| {
            for s in v {
                if let Some(l) = &s.link {
                    f(l);
                }
            }
        };
        match self {
            Block::Heading { spans: s, .. } | Block::Paragraph(s) | Block::Quote(s) => spans(s),
            Block::List { items, .. } => items.iter().for_each(|i| spans(i)),
            Block::Table { header, rows } => {
                header.iter().for_each(|c| spans(c));
                rows.iter().flatten().for_each(|c| spans(c));
            }
            _ => {}
        }
    }

    /// Testo semplice del blocco (per la ricerca).
    pub fn plain_text(&self) -> String {
        let join = |v: &[Span]| v.iter().map(|s| s.text.as_str()).collect::<String>();
        match self {
            Block::Heading { spans, .. } | Block::Paragraph(spans) | Block::Quote(spans) => join(spans),
            Block::List { items, .. } => items.iter().map(|i| join(i)).collect::<Vec<_>>().join(" "),
            Block::Table { header, rows } => header
                .iter()
                .chain(rows.iter().flatten())
                .map(|c| join(c))
                .collect::<Vec<_>>()
                .join(" "),
            Block::Image { caption, .. } => caption.clone(),
            Block::Code(c) => c.clone(),
            Block::Rule => String::new(),
        }
    }
}

/// Tabella in costruzione: intestazione, righe, "si sta leggendo l'intestazione".
type TableAcc = (Vec<Vec<Span>>, Vec<Vec<Vec<Span>>>, bool);

#[derive(Default)]
struct Style {
    bold: u32,
    italic: u32,
    link: Option<String>,
}

pub fn parse_blocks(md: &str) -> Vec<Block> {
    let mut out = Vec::new();
    let mut spans: Vec<Span> = Vec::new();
    let mut style = Style::default();
    let mut heading: Option<u8> = None;
    let mut list: Option<(Option<u64>, Vec<Vec<Span>>)> = None;
    let mut in_item = false;
    let mut quote = false;
    let mut code: Option<String> = None;
    let mut image: Option<(String, String)> = None;
    let mut table: Option<TableAcc> = None;
    let mut row: Vec<Vec<Span>> = Vec::new();

    let push_text = |spans: &mut Vec<Span>, style: &Style, text: &str, code: bool| {
        spans.push(Span {
            text: text.to_string(),
            bold: style.bold > 0,
            italic: style.italic > 0,
            code,
            link: style.link.clone(),
        });
    };

    for ev in Parser::new_ext(md, Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH) {
        match ev {
            Event::Start(Tag::Heading { level, .. }) => {
                heading = Some(match level {
                    HeadingLevel::H1 => 1,
                    HeadingLevel::H2 => 2,
                    HeadingLevel::H3 => 3,
                    _ => 4,
                })
            }
            Event::End(TagEnd::Heading(_)) => out.push(Block::Heading {
                level: heading.take().unwrap_or(2),
                spans: std::mem::take(&mut spans),
            }),
            Event::Start(Tag::Paragraph) => {}
            Event::End(TagEnd::Paragraph) => {
                if in_item {
                    // Paragrafi dentro un elemento d'elenco: restano nell'elemento.
                    spans.push(Span {
                        text: " ".into(),
                        ..Default::default()
                    });
                } else if let Some((src, caption)) = image.take() {
                    out.push(Block::Image { src, caption });
                    spans.clear();
                } else if !spans.is_empty() {
                    let s = std::mem::take(&mut spans);
                    out.push(if quote { Block::Quote(s) } else { Block::Paragraph(s) });
                }
            }
            Event::Start(Tag::BlockQuote(_)) => quote = true,
            Event::End(TagEnd::BlockQuote(_)) => quote = false,
            Event::Start(Tag::List(start)) => list = Some((start, Vec::new())),
            Event::End(TagEnd::List(_)) => {
                if let Some((start, items)) = list.take() {
                    out.push(Block::List { start, items });
                }
            }
            Event::Start(Tag::Item) => in_item = true,
            Event::End(TagEnd::Item) => {
                in_item = false;
                let mut s = std::mem::take(&mut spans);
                while s.last().is_some_and(|x| x.text.trim().is_empty()) {
                    s.pop();
                }
                if let Some((_, items)) = &mut list {
                    items.push(s);
                }
            }
            Event::Start(Tag::CodeBlock(_)) => code = Some(String::new()),
            Event::End(TagEnd::CodeBlock) => out.push(Block::Code(code.take().unwrap_or_default().trim_end().to_string())),
            Event::Start(Tag::Strong) => style.bold += 1,
            Event::End(TagEnd::Strong) => style.bold = style.bold.saturating_sub(1),
            Event::Start(Tag::Emphasis) => style.italic += 1,
            Event::End(TagEnd::Emphasis) => style.italic = style.italic.saturating_sub(1),
            Event::Start(Tag::Link { dest_url, .. }) => style.link = Some(dest_url.to_string()),
            Event::End(TagEnd::Link) => style.link = None,
            Event::Start(Tag::Image { dest_url, .. }) => image = Some((dest_url.to_string(), String::new())),
            Event::End(TagEnd::Image) => {}
            Event::Start(Tag::Table(_)) => table = Some((Vec::new(), Vec::new(), true)),
            Event::End(TagEnd::Table) => {
                if let Some((header, rows, _)) = table.take() {
                    out.push(Block::Table { header, rows });
                }
            }
            Event::Start(Tag::TableHead) => row.clear(),
            Event::End(TagEnd::TableHead) => {
                if let Some(t) = &mut table {
                    t.0 = std::mem::take(&mut row);
                    t.2 = false;
                }
            }
            Event::Start(Tag::TableRow) => row.clear(),
            Event::End(TagEnd::TableRow) => {
                if let Some(t) = &mut table {
                    t.1.push(std::mem::take(&mut row));
                }
            }
            Event::End(TagEnd::TableCell) => row.push(std::mem::take(&mut spans)),
            Event::Text(t) => {
                if let Some(c) = &mut code {
                    c.push_str(&t);
                } else if let Some((_, caption)) = &mut image {
                    caption.push_str(&t);
                } else {
                    push_text(&mut spans, &style, &t, false);
                }
            }
            Event::Code(t) => push_text(&mut spans, &style, &t, true),
            Event::SoftBreak => push_text(&mut spans, &style, " ", false),
            Event::HardBreak => push_text(&mut spans, &style, "\n", false),
            Event::Rule => out.push(Block::Rule),
            _ => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_from_markdown() {
        let md = "# Titolo\n\nTesto **forte** e [IOR](mat.ior).\n\n![Didascalia](img/comune/a.jpg)\n\n1. uno\n2. due\n\n| A | B |\n|---|---|\n| 1 | 2 |\n";
        let b = parse_blocks(md);
        assert!(matches!(&b[0], Block::Heading { level: 1, .. }));
        let Block::Paragraph(p) = &b[1] else { panic!("{b:?}") };
        assert!(p.iter().any(|s| s.bold && s.text == "forte"));
        assert!(p.iter().any(|s| s.link.as_deref() == Some("mat.ior")));
        assert_eq!(
            b[2],
            Block::Image {
                src: "img/comune/a.jpg".into(),
                caption: "Didascalia".into()
            }
        );
        let Block::List { start: Some(1), items } = &b[3] else {
            panic!("{b:?}")
        };
        assert_eq!(items.len(), 2);
        let Block::Table { header, rows } = &b[4] else { panic!("{b:?}") };
        assert_eq!((header.len(), rows.len()), (2, 1));
    }
}
