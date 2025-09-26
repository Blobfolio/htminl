/*!
# HTMinL: DOM Node.
*/

use html5ever::{
	QualName,
	tendril::StrTendril,
};
use indexmap::IndexMap;
use std::{
	cell::RefCell,
	rc::Rc,
};



/// # Refcounted Node.
///
/// Nodes are self-referential, so generally need to be wrapped in an `Rc`.
pub(crate) type Handle = Rc<Node>;



#[derive(Debug)]
/// # DOM Node.
///
/// This struct holds tag/attribute/content details for a node and its
/// children. At the root level, it's the whole damn tree.
///
/// In practice, most references hold `Handle` instead, which is an `Rc`-
/// wrapped version.
pub(crate) struct Node {
	/// # Node Kind/Data.
	pub(crate) inner: NodeInner,

	/// # Child Node(s).
	pub(crate) children: RefCell<Vec<Handle>>,
}

impl Node {
	#[must_use]
	/// # New Element.
	pub(crate) fn new(inner: NodeInner) -> Handle {
		Rc::new(Self {
			inner,
			children: RefCell::new(Vec::new()),
		})
	}
}



#[derive(Debug, Clone)]
/// # Node Kind/Data.
pub(crate) enum NodeInner {
	/// # The Root Node.
	Document,

	/// # HTML Element.
	Element {
		/// # Tag Name.
		name: QualName,

		/// # Tag Attributes.
		attrs: RefCell<IndexMap<QualName, StrTendril>>,
	},

	/// # Text.
	Text {
		/// # Content.
		contents: RefCell<StrTendril>
	},

	/// # Comments, Doctypes, Processing Instructions.
	///
	/// We don't support these node types, but the `TreeSink` API requires we
	/// "create" them anyway.
	Ignored,
}
