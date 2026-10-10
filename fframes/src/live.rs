//! Live rendering for runtimes that keep the frame on screen (the browser): instead of a whole SVG
//! string per frame, send only what changed so the page keeps the same DOM nodes. Persistent nodes
//! are what make the frame interactive (event handlers, focus, hover styles), and the browser skips
//! re-parsing and re-creating the unchanged parts of the frame.
//!
//! Each frame is parsed and compared with the previous one. Elements are addressed by their path of
//! element-child indexes from the root `<svg>`. When the children of an element change shape
//! (count or tags), that element is replaced whole with its markup.
//!
//! The comparison keeps the previous frame's text and a flat table of byte ranges into it, so
//! unchanged parts cost a byte comparison and nothing is allocated for them. Identical subtrees are
//! skipped with a single comparison of their markup.

use std::borrow::Cow;
use std::ops::Range;

use crate::roxmltree;

/// Sets an attribute. Strings: name, value.
pub const OP_SET: u32 = 0;
/// Removes an attribute. Strings: name.
pub const OP_REMOVE: u32 = 1;
/// Replaces the text of an element that holds only text. Strings: text.
pub const OP_TEXT: u32 = 2;
/// Replaces an element with markup; an empty path mounts the root. Strings: markup.
pub const OP_REPLACE: u32 = 3;

/// One operation, decoded from [`LiveOps`].
#[derive(Debug, PartialEq, Eq)]
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

/// The operations of one frame in the wire format of the JS runtime: `numbers` holds
/// `[op, path length, ...path]` for every operation and `strings` holds the operation's string
/// arguments in order, each followed by `\0` (XML text cannot contain it).
#[derive(Debug, Default)]
pub struct LiveOps {
    pub numbers: Vec<u32>,
    pub strings: String,
    count: usize,
}

impl LiveOps {
    /// Number of operations.
    pub fn len(&self) -> usize {
        self.count
    }

    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    fn clear(&mut self) {
        self.numbers.clear();
        self.strings.clear();
        self.count = 0;
    }

    fn push(&mut self, op: u32, path: &[u32], strings: &[&str]) {
        self.numbers.push(op);
        self.numbers.push(path.len() as u32);
        self.numbers.extend_from_slice(path);
        for string in strings {
            self.strings.push_str(string);
            self.strings.push('\0');
        }
        self.count += 1;
    }

    /// Decodes the operations, for tests and native runtimes.
    pub fn to_ops(&self) -> Vec<Op> {
        let mut strings = self.strings.split('\0').map(str::to_string);
        let mut next = || strings.next().unwrap_or_default();
        let mut ops = Vec::with_capacity(self.count);
        let mut i = 0;
        while i < self.numbers.len() {
            let (op, len) = (self.numbers[i], self.numbers[i + 1] as usize);
            let path = self.numbers[i + 2..i + 2 + len].to_vec();
            i += 2 + len;
            ops.push(match op {
                OP_SET => Op::Set {
                    path,
                    name: next(),
                    value: next(),
                },
                OP_REMOVE => Op::Remove { path, name: next() },
                OP_TEXT => Op::Text { path, text: next() },
                _ => Op::Replace {
                    path,
                    markup: next(),
                },
            });
        }
        ops
    }
}

#[derive(Debug, Clone)]
enum Kids {
    /// Element children, as a range of [`Tree::children`].
    Elements(Range<usize>),
    /// Only text (possibly none): its raw range in the source.
    Text(Range<usize>),
    /// Text mixed with elements: replaced whole when it changes.
    Mixed,
}

#[derive(Debug, Clone)]
struct FlatNode {
    tag: Range<usize>,
    /// The element's markup.
    markup: Range<usize>,
    /// Range of [`Tree::attrs`].
    attrs: Range<usize>,
    kids: Kids,
}

#[derive(Debug, Clone)]
struct Attr {
    /// The qualified name as written, e.g. `xlink:href`.
    name: Range<usize>,
    /// The raw value, entities not decoded.
    value: Range<usize>,
}

/// A parsed frame: its text and byte ranges into it.
#[derive(Debug, Default)]
struct Tree {
    source: String,
    nodes: Vec<FlatNode>,
    attrs: Vec<Attr>,
    children: Vec<usize>,
}

impl Tree {
    fn text(&self, range: &Range<usize>) -> &str {
        &self.source[range.clone()]
    }

    fn clear(&mut self) {
        self.nodes.clear();
        self.attrs.clear();
        self.children.clear();
    }

    /// Parses `source` and makes it this tree. Returns the root element's index.
    fn rebuild(&mut self, source: String) -> Result<usize, String> {
        self.clear();
        let root = if let Some(root) = Scanner::new(&source).root(self) {
            root
        } else {
            self.clear();
            let document = roxmltree::Document::parse(&source).map_err(|err| err.to_string())?;
            self.add(document.root_element(), &source)
        };
        self.source = source;
        Ok(root)
    }

    fn add(&mut self, node: roxmltree::Node, source: &str) -> usize {
        let markup = node.range();
        let tag_len = source[markup.start + 1..markup.end]
            .find(|c: char| c.is_ascii_whitespace() || c == '/' || c == '>')
            .unwrap_or(0);
        let tag = markup.start + 1..markup.start + 1 + tag_len;

        let attrs_start = self.attrs.len();
        self.attrs.extend(node.attributes().map(|attr| Attr {
            name: attr.range_qname(),
            value: attr.range_value(),
        }));
        let attrs = attrs_start..self.attrs.len();

        let index = self.nodes.len();
        self.nodes.push(FlatNode {
            tag,
            markup,
            attrs,
            kids: Kids::Text(0..0),
        });

        let mut elements: Vec<usize> = Vec::new();
        let mut text: Option<Range<usize>> = None;
        let mut has_text = false;
        for child in node.children() {
            if child.is_element() {
                elements.push(self.add(child, source));
            } else if child.is_text() {
                let range = child.range();
                has_text |= !source[range.clone()].trim().is_empty();
                text = Some(match text {
                    Some(previous) => previous.start..range.end,
                    None => range,
                });
            }
        }

        self.nodes[index].kids = match (elements.is_empty(), has_text) {
            (true, _) => Kids::Text(text.unwrap_or(0..0)),
            (false, false) => {
                let start = self.children.len();
                self.children.extend(elements);
                Kids::Elements(start..self.children.len())
            }
            (false, true) => Kids::Mixed,
        };
        index
    }
}

/// A fast reader for the plain XML that `svgr!` prints: elements, double- or single-quoted
/// attributes and text. Anything else (comments, CDATA, processing instructions, malformed
/// markup) makes it give up, and the tree is built with `roxmltree` instead.
struct Scanner<'s> {
    bytes: &'s [u8],
    at: usize,
}

impl<'s> Scanner<'s> {
    fn new(source: &'s str) -> Self {
        Self {
            bytes: source.as_bytes(),
            at: 0,
        }
    }

    fn skip_space(&mut self) {
        while self.bytes.get(self.at).is_some_and(u8::is_ascii_whitespace) {
            self.at += 1;
        }
    }

    fn name(&mut self) -> Option<Range<usize>> {
        let start = self.at;
        while self.bytes.get(self.at).is_some_and(|&b| {
            !b.is_ascii_whitespace() && !matches!(b, b'/' | b'>' | b'=' | b'<' | b'"' | b'\'')
        }) {
            self.at += 1;
        }
        (self.at > start).then_some(start..self.at)
    }

    fn root(mut self, tree: &mut Tree) -> Option<usize> {
        self.skip_space();
        let root = self.element(tree)?;
        self.skip_space();
        (self.at == self.bytes.len()).then_some(root)
    }

    fn element(&mut self, tree: &mut Tree) -> Option<usize> {
        let start = self.at;
        if self.bytes.get(self.at) != Some(&b'<') {
            return None;
        }
        self.at += 1;
        let tag = self.name()?;

        let attrs_start = tree.attrs.len();
        let self_closing = loop {
            self.skip_space();
            match self.bytes.get(self.at)? {
                b'/' => {
                    if self.bytes.get(self.at + 1) != Some(&b'>') {
                        return None;
                    }
                    self.at += 2;
                    break true;
                }
                b'>' => {
                    self.at += 1;
                    break false;
                }
                _ => {
                    let name = self.name()?;
                    self.skip_space();
                    if self.bytes.get(self.at) != Some(&b'=') {
                        return None;
                    }
                    self.at += 1;
                    self.skip_space();
                    let quote = *self.bytes.get(self.at)?;
                    if quote != b'"' && quote != b'\'' {
                        return None;
                    }
                    self.at += 1;
                    let value_start = self.at;
                    while *self.bytes.get(self.at)? != quote {
                        if self.bytes[self.at] == b'<' {
                            return None;
                        }
                        self.at += 1;
                    }
                    // Namespace declarations are not attributes (roxmltree leaves them out too).
                    let qualified = &self.bytes[name.clone()];
                    if qualified != b"xmlns" && !qualified.starts_with(b"xmlns:") {
                        tree.attrs.push(Attr {
                            name,
                            value: value_start..self.at,
                        });
                    }
                    self.at += 1;
                }
            }
        };
        let attrs = attrs_start..tree.attrs.len();

        let index = tree.nodes.len();
        tree.nodes.push(FlatNode {
            tag: tag.clone(),
            markup: start..start,
            attrs,
            kids: Kids::Text(0..0),
        });

        let mut elements: Vec<usize> = Vec::new();
        let mut text: Option<Range<usize>> = None;
        let mut has_text = false;
        if !self_closing {
            loop {
                match self.bytes.get(self.at)? {
                    b'<' if self.bytes.get(self.at + 1) == Some(&b'/') => {
                        self.at += 2;
                        let closing = self.name()?;
                        if self.bytes[closing] != self.bytes[tag.clone()] {
                            return None;
                        }
                        self.skip_space();
                        if self.bytes.get(self.at) != Some(&b'>') {
                            return None;
                        }
                        self.at += 1;
                        break;
                    }
                    b'<' if matches!(self.bytes.get(self.at + 1), Some(b'!' | b'?')) => {
                        return None;
                    }
                    b'<' => elements.push(self.element(tree)?),
                    _ => {
                        let text_start = self.at;
                        while self.bytes.get(self.at).is_some_and(|&b| b != b'<') {
                            self.at += 1;
                        }
                        has_text |= self.bytes[text_start..self.at]
                            .iter()
                            .any(|b| !b.is_ascii_whitespace());
                        text = Some(match text {
                            Some(previous) => previous.start..self.at,
                            None => text_start..self.at,
                        });
                    }
                }
            }
        }

        tree.nodes[index].markup = start..self.at;
        tree.nodes[index].kids = match (elements.is_empty(), has_text) {
            (true, _) => Kids::Text(text.unwrap_or(0..0)),
            (false, false) => {
                let first = tree.children.len();
                tree.children.extend(elements);
                Kids::Elements(first..tree.children.len())
            }
            (false, true) => Kids::Mixed,
        };
        Some(index)
    }
}

/// Decodes the five predefined XML entities and character references.
fn unescape(raw: &str) -> Cow<'_, str> {
    if !raw.contains('&') {
        return Cow::Borrowed(raw);
    }
    let mut out = String::with_capacity(raw.len());
    let mut rest = raw;
    while let Some(start) = rest.find('&') {
        out.push_str(&rest[..start]);
        rest = &rest[start..];
        let Some(end) = rest.find(';') else { break };
        let entity = &rest[1..end];
        let decoded = match entity {
            "amp" => Some('&'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            _ => entity
                .strip_prefix("#x")
                .and_then(|hex| u32::from_str_radix(hex, 16).ok())
                .or_else(|| entity.strip_prefix('#').and_then(|dec| dec.parse().ok()))
                .and_then(char::from_u32),
        };
        if let Some(c) = decoded {
            out.push(c);
            rest = &rest[end + 1..];
        } else {
            out.push('&');
            rest = &rest[1..];
        }
    }
    out.push_str(rest);
    Cow::Owned(out)
}

fn diff(old: &Tree, new: &Tree, o: usize, n: usize, path: &mut Vec<u32>, ops: &mut LiveOps) {
    let (old_node, new_node) = (&old.nodes[o], &new.nodes[n]);
    if old.text(&old_node.markup) == new.text(&new_node.markup) {
        return;
    }

    let same_shape = old.text(&old_node.tag) == new.text(&new_node.tag)
        && match (&old_node.kids, &new_node.kids) {
            (Kids::Elements(a), Kids::Elements(b)) => {
                a.len() == b.len()
                    && old.children[a.clone()]
                        .iter()
                        .zip(&new.children[b.clone()])
                        .all(|(&a, &b)| old.text(&old.nodes[a].tag) == new.text(&new.nodes[b].tag))
            }
            (Kids::Text(_), Kids::Text(_)) => true,
            _ => false,
        };
    if !same_shape {
        ops.push(OP_REPLACE, path, &[new.text(&new_node.markup)]);
        return;
    }

    let old_attrs = &old.attrs[old_node.attrs.clone()];
    for attr in &new.attrs[new_node.attrs.clone()] {
        let name = new.text(&attr.name);
        let value = new.text(&attr.value);
        let unchanged = old_attrs
            .iter()
            .any(|old_attr| old.text(&old_attr.name) == name && old.text(&old_attr.value) == value);
        if !unchanged {
            ops.push(OP_SET, path, &[name, &unescape(value)]);
        }
    }
    for old_attr in old_attrs {
        let name = old.text(&old_attr.name);
        let kept = new.attrs[new_node.attrs.clone()]
            .iter()
            .any(|attr| new.text(&attr.name) == name);
        if !kept {
            ops.push(OP_REMOVE, path, &[name]);
        }
    }

    match (&old_node.kids, &new_node.kids) {
        (Kids::Text(a), Kids::Text(b)) => {
            if old.text(a) != new.text(b) {
                ops.push(OP_TEXT, path, &[&unescape(new.text(b))]);
            }
        }
        (Kids::Elements(a), Kids::Elements(b)) => {
            for (index, (&a, &b)) in old.children[a.clone()]
                .iter()
                .zip(&new.children[b.clone()])
                .enumerate()
            {
                path.push(index as u32);
                diff(old, new, a, b, path, ops);
                path.pop();
            }
        }
        _ => {}
    }
}

/// Remembers the last frame and turns the next one into DOM operations.
#[derive(Debug, Default)]
pub struct LiveDiff {
    previous: Tree,
    current: Tree,
    /// Root element of `previous`, `None` until a frame is mounted.
    previous_root: Option<usize>,
    ops: LiveOps,
    path: Vec<u32>,
}

impl LiveDiff {
    /// Forgets the mounted frame, so the next one is sent whole.
    pub fn reset(&mut self) {
        self.previous_root = None;
    }

    /// Compares `svg` with the previous frame and returns the operations that turn one into the
    /// other. The first frame after creation or [`LiveDiff::reset`] mounts the root whole.
    pub fn next(&mut self, svg: String) -> Result<&LiveOps, String> {
        let root = self.current.rebuild(svg)?;
        self.ops.clear();
        if let Some(previous_root) = self.previous_root {
            self.path.clear();
            diff(
                &self.previous,
                &self.current,
                previous_root,
                root,
                &mut self.path,
                &mut self.ops,
            );
        } else {
            let markup = self.current.text(&self.current.nodes[root].markup);
            self.ops.push(OP_REPLACE, &[], &[markup]);
        }
        std::mem::swap(&mut self.previous, &mut self.current);
        self.previous_root = Some(root);
        Ok(&self.ops)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10">"#;

    fn frame(body: &str) -> String {
        format!("{SVG}{body}</svg>")
    }

    fn ops(live: &mut LiveDiff, svg: &str) -> Vec<Op> {
        live.next(svg.to_string()).unwrap().to_ops()
    }

    #[test]
    fn first_frame_mounts_whole() {
        let mut live = LiveDiff::default();
        let svg = frame(r#"<rect x="1"/>"#);
        assert_eq!(
            ops(&mut live, &svg),
            vec![Op::Replace {
                path: vec![],
                markup: svg
            }]
        );
    }

    #[test]
    fn unchanged_frame_sends_nothing() {
        let mut live = LiveDiff::default();
        ops(&mut live, &frame(r#"<rect x="1"/>"#));
        assert!(ops(&mut live, &frame(r#"<rect x="1"/>"#)).is_empty());
    }

    #[test]
    fn attributes_and_text_are_patched_in_place() {
        let mut live = LiveDiff::default();
        ops(
            &mut live,
            &frame(r#"<g><rect x="1" y="2"/><text>a</text></g>"#),
        );
        let result = ops(
            &mut live,
            &frame(r#"<g><rect x="5" fill="red"/><text>b</text></g>"#),
        );
        assert_eq!(
            result,
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
        ops(&mut live, &frame(r#"<g id="a"><rect/></g>"#));
        let result = ops(&mut live, &frame(r#"<g id="a"><rect/><circle r="1"/></g>"#));
        assert_eq!(
            result,
            vec![Op::Replace {
                path: vec![0],
                markup: r#"<g id="a"><rect/><circle r="1"/></g>"#.into()
            }]
        );
    }

    #[test]
    fn mixed_text_is_replaced_whole() {
        let mut live = LiveDiff::default();
        ops(&mut live, &frame("<text>a<tspan>b</tspan></text>"));
        let result = ops(&mut live, &frame("<text>c<tspan>b</tspan></text>"));
        assert_eq!(
            result,
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
        ops(&mut live, &image("a.png"));
        assert_eq!(
            ops(&mut live, &image("b.png")),
            vec![Op::Set {
                path: vec![0],
                name: "xlink:href".into(),
                value: "b.png".into()
            }]
        );
    }

    #[test]
    fn entities_are_decoded_in_values_and_text() {
        let mut live = LiveDiff::default();
        ops(
            &mut live,
            r#"<svg xmlns="http://www.w3.org/2000/svg"><text x="1">a</text></svg>"#,
        );
        assert_eq!(
            ops(
                &mut live,
                r#"<svg xmlns="http://www.w3.org/2000/svg"><text x="&quot;2&quot;">a &amp; b &#x41;</text></svg>"#
            ),
            vec![
                Op::Set {
                    path: vec![0],
                    name: "x".into(),
                    value: "\"2\"".into()
                },
                Op::Text {
                    path: vec![0],
                    text: "a & b A".into()
                },
            ]
        );
    }

    #[test]
    fn reset_mounts_the_next_frame_whole() {
        let mut live = LiveDiff::default();
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg"><rect/></svg>"#;
        ops(&mut live, svg);
        live.reset();
        assert_eq!(
            ops(&mut live, svg),
            vec![Op::Replace {
                path: vec![],
                markup: svg.into()
            }]
        );
    }

    #[test]
    fn changed_child_tag_replaces_the_parent() {
        let mut live = LiveDiff::default();
        ops(
            &mut live,
            r#"<svg xmlns="http://www.w3.org/2000/svg"><g><rect/></g></svg>"#,
        );
        assert_eq!(
            ops(
                &mut live,
                r#"<svg xmlns="http://www.w3.org/2000/svg"><g><circle/></g></svg>"#
            ),
            vec![Op::Replace {
                path: vec![0],
                markup: "<g><circle/></g>".into()
            }]
        );
    }

    fn shape(tree: &Tree, index: usize) -> String {
        let node = &tree.nodes[index];
        let attrs: Vec<String> = tree.attrs[node.attrs.clone()]
            .iter()
            .map(|a| format!("{}={}", tree.text(&a.name), tree.text(&a.value)))
            .collect();
        let kids = match &node.kids {
            Kids::Elements(range) => tree.children[range.clone()]
                .iter()
                .map(|&child| shape(tree, child))
                .collect::<Vec<_>>()
                .join(","),
            Kids::Text(range) => format!("'{}'", tree.text(range)),
            Kids::Mixed => "mixed".into(),
        };
        format!(
            "{}[{}]({})<{}>",
            tree.text(&node.tag),
            attrs.join(" "),
            kids,
            tree.text(&node.markup).len()
        )
    }

    #[test]
    fn scanner_reads_svgr_markup_like_roxmltree() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><g transform="translate(1 2)"><circle cx="1" cy='2' r="3"/><text x="1" font-family="DM Sans">Hello &amp; bye</text><text>a<tspan>b</tspan></text><g></g></g></svg>"#;
        let mut fast = Tree::default();
        let fast_root = Scanner::new(svg)
            .root(&mut fast)
            .expect("scanner reads svgr output");
        fast.source = svg.to_string();

        let mut slow = Tree::default();
        let document = roxmltree::Document::parse(svg).unwrap();
        let slow_root = slow.add(document.root_element(), svg);
        slow.source = svg.to_string();

        assert_eq!(shape(&fast, fast_root), shape(&slow, slow_root));
    }

    #[test]
    fn scanner_gives_up_on_comments_and_falls_back() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg"><!-- note --><rect x="1"/></svg>"#;
        assert!(Scanner::new(svg).root(&mut Tree::default()).is_none());
        let mut live = LiveDiff::default();
        ops(&mut live, svg);
        assert_eq!(
            ops(
                &mut live,
                r#"<svg xmlns="http://www.w3.org/2000/svg"><!-- note --><rect x="2"/></svg>"#
            ),
            vec![Op::Set {
                path: vec![0],
                name: "x".into(),
                value: "2".into()
            }]
        );
    }
}
