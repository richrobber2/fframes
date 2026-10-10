//! Live rendering for runtimes that keep the frame on screen (the browser): instead of a whole SVG string per frame, send the browser only what changed so it
//! keeps the same DOM nodes. Persistent nodes are what make the frame interactive (event handlers,
//! focus, hover styles) and they skip re-parsing and re-creating the unchanged parts of the frame.
//!
//! The frame is parsed into a light tree and compared with the previous one. Elements are addressed
//! by their path of element-child indexes from the root `<svg>`. When the children of an element
//! change shape (count or tags), that element is replaced as a whole with its markup.

use crate::roxmltree;

/// Sets an attribute: `[OP_SET, path, name, value]`.
pub const OP_SET: u32 = 0;
/// Removes an attribute: `[OP_REMOVE, path, name]`.
pub const OP_REMOVE: u32 = 1;
/// Replaces the text of an element that holds only text: `[OP_TEXT, path, text]`.
pub const OP_TEXT: u32 = 2;
/// Replaces an element with markup: `[OP_REPLACE, path, markup]`. An empty path mounts the root.
pub const OP_REPLACE: u32 = 3;

#[derive(Debug, PartialEq)]
enum Children {
    Elements(Vec<Node>),
    Text(String),
    /// Text mixed with elements. Compared as markup and replaced whole when it differs.
    Mixed,
}

#[derive(Debug, PartialEq)]
struct Node {
    tag: String,
    attrs: Vec<(String, String)>,
    children: Children,
    /// The element's markup in the frame source, kept to replace it whole.
    markup: String,
}

#[derive(Debug, PartialEq)]
pub enum Op {
    Set {
        path: Vec<u32>,
        name: String,
        value: String,
    },
    Remove {
        path: Vec<u32>,
        name: String,
    },
    Text {
        path: Vec<u32>,
        text: String,
    },
    Replace {
        path: Vec<u32>,
        markup: String,
    },
}

const XLINK_NS: &str = "http://www.w3.org/1999/xlink";
const XML_NS: &str = "http://www.w3.org/XML/1998/namespace";

fn attr_name(attr: &roxmltree::Attribute) -> String {
    match attr.namespace() {
        Some(XLINK_NS) => format!("xlink:{}", attr.name()),
        Some(XML_NS) => format!("xml:{}", attr.name()),
        _ => attr.name().to_string(),
    }
}

fn build(node: roxmltree::Node, source: &str) -> Node {
    let mut elements = Vec::new();
    let mut text = String::new();
    let mut has_text = false;
    for child in node.children() {
        if child.is_element() {
            elements.push(build(child, source));
        } else if let Some(value) = child.text() {
            if !value.trim().is_empty() {
                has_text = true;
            }
            text.push_str(value);
        }
    }

    let children = match (elements.is_empty(), has_text) {
        (true, _) => Children::Text(text),
        (false, false) => Children::Elements(elements),
        (false, true) => Children::Mixed,
    };

    Node {
        tag: node.tag_name().name().to_string(),
        attrs: node
            .attributes()
            .map(|attr| (attr_name(&attr), attr.value().to_string()))
            .collect(),
        children,
        markup: source[node.range()].to_string(),
    }
}

fn diff(old: &Node, new: &Node, path: &mut Vec<u32>, ops: &mut Vec<Op>) {
    let same_shape = old.tag == new.tag
        && match (&old.children, &new.children) {
            (Children::Elements(a), Children::Elements(b)) => a.len() == b.len(),
            (Children::Text(_), Children::Text(_)) => true,
            (Children::Mixed, Children::Mixed) => old.markup == new.markup,
            _ => false,
        };
    if !same_shape {
        ops.push(Op::Replace {
            path: path.clone(),
            markup: new.markup.clone(),
        });
        return;
    }
    if old.markup == new.markup {
        return;
    }

    for (name, value) in &new.attrs {
        let unchanged = old.attrs.iter().any(|(n, v)| n == name && v == value);
        if !unchanged {
            ops.push(Op::Set {
                path: path.clone(),
                name: name.clone(),
                value: value.clone(),
            });
        }
    }
    for (name, _) in &old.attrs {
        if !new.attrs.iter().any(|(n, _)| n == name) {
            ops.push(Op::Remove {
                path: path.clone(),
                name: name.clone(),
            });
        }
    }

    match (&old.children, &new.children) {
        (Children::Text(a), Children::Text(b)) if a != b => ops.push(Op::Text {
            path: path.clone(),
            text: b.clone(),
        }),
        (Children::Elements(a), Children::Elements(b)) => {
            for (index, (old_child, new_child)) in a.iter().zip(b).enumerate() {
                path.push(index as u32);
                diff(old_child, new_child, path, ops);
                path.pop();
            }
        }
        _ => {}
    }
}

/// Remembers the last frame and turns the next one into DOM operations.
#[derive(Default)]
pub struct LiveDiff {
    previous: Option<Node>,
}

impl LiveDiff {
    /// Forgets the mounted frame, so the next one is sent whole.
    pub fn reset(&mut self) {
        self.previous = None;
    }

    pub fn next(&mut self, svg: &str) -> Result<Vec<Op>, String> {
        let document = roxmltree::Document::parse(svg).map_err(|err| err.to_string())?;
        let next = build(document.root_element(), svg);
        let mut ops = Vec::new();
        match &self.previous {
            Some(previous) => diff(previous, &next, &mut Vec::new(), &mut ops),
            None => ops.push(Op::Replace {
                path: Vec::new(),
                markup: next.markup.clone(),
            }),
        }
        self.previous = Some(next);
        Ok(ops)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10">"#;

    fn frame(body: &str) -> String {
        format!("{SVG}{body}</svg>")
    }

    #[test]
    fn first_frame_mounts_whole() {
        let mut live = LiveDiff::default();
        let svg = frame(r#"<rect x="1"/>"#);
        assert_eq!(
            live.next(&svg).unwrap(),
            vec![Op::Replace {
                path: vec![],
                markup: svg
            }]
        );
    }

    #[test]
    fn unchanged_frame_sends_nothing() {
        let mut live = LiveDiff::default();
        live.next(&frame(r#"<rect x="1"/>"#)).unwrap();
        assert!(live.next(&frame(r#"<rect x="1"/>"#)).unwrap().is_empty());
    }

    #[test]
    fn attributes_and_text_are_patched_in_place() {
        let mut live = LiveDiff::default();
        live.next(&frame(r#"<g><rect x="1" y="2"/><text>a</text></g>"#))
            .unwrap();
        let ops = live
            .next(&frame(r#"<g><rect x="5" fill="red"/><text>b</text></g>"#))
            .unwrap();
        assert_eq!(
            ops,
            vec![
                Op::Set {
                    path: vec![0, 0],
                    name: "x".into(),
                    value: "5".into()
                },
                Op::Set {
                    path: vec![0, 0],
                    name: "fill".into(),
                    value: "red".into()
                },
                Op::Remove {
                    path: vec![0, 0],
                    name: "y".into()
                },
                Op::Text {
                    path: vec![0, 1],
                    text: "b".into()
                },
            ]
        );
    }

    #[test]
    fn changed_children_replace_the_parent() {
        let mut live = LiveDiff::default();
        live.next(&frame(r#"<g id="a"><rect/></g>"#)).unwrap();
        let ops = live
            .next(&frame(r#"<g id="a"><rect/><circle r="1"/></g>"#))
            .unwrap();
        assert_eq!(
            ops,
            vec![Op::Replace {
                path: vec![0],
                markup: r#"<g id="a"><rect/><circle r="1"/></g>"#.into()
            }]
        );
    }

    #[test]
    fn mixed_text_is_replaced_whole() {
        let mut live = LiveDiff::default();
        live.next(&frame("<text>a<tspan>b</tspan></text>")).unwrap();
        let ops = live.next(&frame("<text>c<tspan>b</tspan></text>")).unwrap();
        assert_eq!(
            ops,
            vec![Op::Replace {
                path: vec![0],
                markup: "<text>c<tspan>b</tspan></text>".into()
            }]
        );
    }

    #[test]
    fn xlink_attributes_keep_their_prefix() {
        let mut live = LiveDiff::default();
        let image = |href: &str| {
            format!(
                r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink"><image xlink:href="{href}"/></svg>"#
            )
        };
        live.next(&image("a.png")).unwrap();
        assert_eq!(
            live.next(&image("b.png")).unwrap(),
            vec![Op::Set {
                path: vec![0],
                name: "xlink:href".into(),
                value: "b.png".into()
            }]
        );
    }
}
