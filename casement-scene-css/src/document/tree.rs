//! Building the document's nodes: `<html><body>` and then `<div class style>` boxes.

use super::style::Props;
use blitz_dom::{Attribute, BaseDocument, DocumentMutator, LocalName, Namespace, NodeId, QualName};

/// An open mutation of a document: boxes are added under its `<body>`.
pub(super) struct Tree<'doc> {
    mutator: DocumentMutator<'doc>,
    body: NodeId,
}

fn name(local: &str) -> QualName {
    QualName::new(None, Namespace::from(""), LocalName::from(local))
}

fn attribute(local: &str, value: String) -> Attribute {
    Attribute {
        name: name(local),
        value,
    }
}

impl<'doc> Tree<'doc> {
    /// `dom` with an `<html><body>` in it, ready for boxes.
    pub(super) fn open(dom: &'doc mut BaseDocument) -> Tree<'doc> {
        let mut mutator = dom.mutate();
        let root = mutator.doc.root_node().id;
        let html = mutator.create_element(name("html"), Vec::new());
        mutator.append_children(root, &[html]);
        let body = mutator.create_element(name("body"), Vec::new());
        mutator.append_children(html, &[body]);
        Tree { mutator, body }
    }

    /// The `<body>`: where the scene's boxes go.
    pub(super) fn body(&self) -> NodeId {
        self.body
    }

    /// A `<div class="{class}" style="{props}">` appended under `parent`.
    pub(super) fn div(&mut self, parent: NodeId, class: &str, props: Props) -> NodeId {
        let attrs = vec![
            attribute("class", class.to_owned()),
            attribute("style", props.text()),
        ];
        let div = self.mutator.create_element(name("div"), attrs);
        self.mutator.append_children(parent, &[div]);
        div
    }
}
