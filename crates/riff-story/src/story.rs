//! Stories: the cells a component is shown in, shared with the reference
//! generator (`tools/reference/build.py`), and the reference pictures it made.
//!
//! A story is `stories/<slug>.json`: sections of rows of cells, each cell
//! plain-data props. Rows may name a template (`"cells": "@matrix"`) and give
//! props every cell of the row inherits. Both sides expand it the same way,
//! so a cell is known by the same `section/row/cell` key on both.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

/// A pointer or keyboard state a cell is pinned in.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PinnedState {
    Hover,
    Active,
    Focus,
}

/// One cell, expanded: its key, pinned state and merged props.
#[derive(Clone, Debug)]
pub struct Cell {
    pub key: String,
    pub id: String,
    pub state: Option<PinnedState>,
    pub props: Props,
}

#[derive(Clone, Debug)]
pub struct Row {
    pub title: String,
    pub cells: Vec<Cell>,
}

#[derive(Clone, Debug)]
pub struct Section {
    pub title: String,
    pub rows: Vec<Row>,
}

#[derive(Clone, Debug)]
pub struct Story {
    pub slug: String,
    pub name: String,
    pub category: String,
    pub description: String,
    /// Where looping animations are held, so live cells match the pictures.
    pub freeze_ms: Option<u64>,
    pub sections: Vec<Section>,
    /// The hash the generator records, to tell a current picture from a stale one.
    pub hash: String,
}

impl Story {
    pub fn cells(&self) -> impl Iterator<Item = &Cell> {
        self.sections
            .iter()
            .flat_map(|s| &s.rows)
            .flat_map(|r| &r.cells)
    }
}

// --- the file format ---------------------------------------------------------

#[derive(Deserialize)]
struct RawStory {
    slug: String,
    name: String,
    category: String,
    #[serde(default)]
    description: String,
    freeze_ms: Option<u64>,
    sections: Vec<RawSection>,
    #[serde(default)]
    templates: BTreeMap<String, Vec<RawCell>>,
}

#[derive(Deserialize)]
struct RawSection {
    id: String,
    title: String,
    rows: Vec<RawRow>,
}

#[derive(Deserialize)]
struct RawRow {
    id: String,
    title: String,
    #[serde(default)]
    props: Map<String, Value>,
    cells: RawCells,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum RawCells {
    Template(String),
    List(Vec<RawCell>),
}

#[derive(Clone, Deserialize)]
struct RawCell {
    id: String,
    state: Option<PinnedState>,
    #[serde(default)]
    props: Map<String, Value>,
}

/// Parses and expands a story. `source` is the file's text, hashed as the
/// generator hashes it.
pub fn parse(source: &str) -> Result<Story, String> {
    let raw: RawStory = serde_json::from_str(source).map_err(|e| e.to_string())?;
    let mut sections = Vec::new();
    for section in raw.sections {
        let mut rows = Vec::new();
        for row in section.rows {
            let cells = match row.cells {
                RawCells::List(cells) => cells,
                RawCells::Template(name) => {
                    let name = name.trim_start_matches('@');
                    raw.templates
                        .get(name)
                        .cloned()
                        .ok_or_else(|| format!("row {}: unknown template @{name}", row.id))?
                }
            };
            let cells = cells
                .into_iter()
                .map(|cell| {
                    let mut props = row.props.clone();
                    props.extend(cell.props);
                    Cell {
                        key: format!("{}/{}/{}", section.id, row.id, cell.id),
                        id: cell.id,
                        state: cell.state,
                        props: Props(props),
                    }
                })
                .collect();
            rows.push(Row {
                title: row.title,
                cells,
            });
        }
        sections.push(Section {
            title: section.title,
            rows,
        });
    }
    let story = Story {
        slug: raw.slug,
        name: raw.name,
        category: raw.category,
        description: raw.description,
        freeze_ms: raw.freeze_ms,
        sections,
        hash: hash(source),
    };
    let mut keys: Vec<_> = story.cells().map(|c| &c.key).collect();
    keys.sort();
    if let Some(pair) = keys.windows(2).find(|pair| pair[0] == pair[1]) {
        return Err(format!("duplicate cell {}", pair[0]));
    }
    Ok(story)
}

/// sha256 of the story file, first 16 hex digits: `story_hash` in build.py.
fn hash(source: &str) -> String {
    let digest = Sha256::digest(source.as_bytes());
    digest[..8].iter().map(|b| format!("{b:02x}")).collect()
}

/// A cell's props: plain JSON, read by each component's `cell` function.
#[derive(Clone, Debug, Default)]
pub struct Props(Map<String, Value>);

impl Props {
    pub fn str(&self, key: &str) -> Option<&str> {
        self.0.get(key).and_then(Value::as_str)
    }

    pub fn bool(&self, key: &str) -> bool {
        self.0.get(key).and_then(Value::as_bool).unwrap_or(false)
    }

    pub fn bool_or(&self, key: &str, default: bool) -> bool {
        self.0.get(key).and_then(Value::as_bool).unwrap_or(default)
    }

    pub fn i64(&self, key: &str) -> Option<i64> {
        self.0.get(key).and_then(Value::as_i64)
    }

    pub fn f32(&self, key: &str) -> Option<f32> {
        self.0
            .get(key)
            .and_then(Value::as_f64)
            .map(|value| value as f32)
    }

    /// Reads either a JSON string (single item) or an array of strings.
    pub fn strings(&self, key: &str) -> Option<Vec<&str>> {
        match self.0.get(key)? {
            Value::String(s) => Some(vec![s.as_str()]),
            Value::Array(items) => items.iter().map(Value::as_str).collect::<Option<Vec<_>>>(),
            _ => None,
        }
    }

    /// Reads an array of JSON objects as nested [`Props`].
    pub fn objects(&self, key: &str) -> Option<Vec<Props>> {
        match self.0.get(key)? {
            Value::Array(items) => items
                .iter()
                .map(|v| v.as_object().cloned().map(Props))
                .collect(),
            _ => None,
        }
    }

    /// Every key, so a component can reject props it does not understand
    /// instead of silently drawing something else.
    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.0.keys().map(String::as_str)
    }
}

// --- reference pictures ------------------------------------------------------

/// The reference pictures of one story in one mode.
#[derive(Clone, Debug)]
pub struct Reference {
    pub dir: PathBuf,
    /// Pixels past the component's box on each side of every picture.
    pub margin: f32,
    pub cells: BTreeMap<String, (f32, f32)>,
    /// Whether the pictures were made from the current story file.
    pub current: bool,
}

impl Reference {
    pub fn picture(&self, key: &str) -> PathBuf {
        self.dir.join(format!("{key}.png"))
    }
}

#[derive(Deserialize)]
struct Manifest {
    margin: f32,
    story: String,
    cells: BTreeMap<String, ManifestCell>,
}

#[derive(Deserialize)]
struct ManifestCell {
    width: f32,
    height: f32,
}

/// Where `tools/reference/build.py` writes.
pub fn reference_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/reference")
}

/// Loads the pictures of `story` for `mode` (`"light"` or `"dark"`), if made.
pub fn load_reference(story: &Story, mode: &str) -> Option<Reference> {
    let dir = reference_root().join(&story.slug).join(mode);
    let text = std::fs::read_to_string(dir.join("manifest.json")).ok()?;
    let manifest: Manifest = serde_json::from_str(&text).ok()?;
    Some(Reference {
        dir,
        margin: manifest.margin,
        current: manifest.story == story.hash,
        cells: manifest
            .cells
            .into_iter()
            .map(|(key, cell)| (key, (cell.width, cell.height)))
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const STORY: &str = r#"{
        "slug": "demo", "name": "Demo", "category": "Test",
        "sections": [{
            "id": "s", "title": "S",
            "rows": [
                { "id": "a", "title": "a", "props": { "variant": "x" }, "cells": "@t" },
                { "id": "b", "title": "b", "cells": [{ "id": "one", "state": "hover", "props": { "label": "L" } }] }
            ]
        }],
        "templates": { "t": [{ "id": "one", "props": { "label": "A" } }, { "id": "two", "props": { "variant": "y" } }] }
    }"#;

    #[test]
    fn expands_templates_and_row_props() {
        let story = parse(STORY).unwrap();
        let cells: Vec<_> = story.cells().collect();
        assert_eq!(
            cells.iter().map(|c| c.key.as_str()).collect::<Vec<_>>(),
            ["s/a/one", "s/a/two", "s/b/one"]
        );
        assert_eq!(cells[0].props.str("variant"), Some("x"));
        assert_eq!(cells[0].props.str("label"), Some("A"));
        // A cell's own props win over its row's.
        assert_eq!(cells[1].props.str("variant"), Some("y"));
        assert_eq!(cells[2].state, Some(PinnedState::Hover));
    }

    #[test]
    fn rejects_unknown_templates_and_duplicates() {
        let unknown = STORY.replace("\"@t\"", "\"@nope\"");
        assert!(parse(&unknown).unwrap_err().contains("unknown template"));
        let duplicate = STORY.replace("\"id\": \"two\"", "\"id\": \"one\"");
        assert!(parse(&duplicate).unwrap_err().contains("duplicate"));
    }

    #[test]
    fn hash_matches_the_generator() {
        // `hashlib.sha256(b"abc").hexdigest()[:16]` in build.py.
        assert_eq!(hash("abc"), "ba7816bf8f01cfea");
    }
}
