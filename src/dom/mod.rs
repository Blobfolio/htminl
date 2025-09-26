/*!
# HTMinL: DOM.
*/

pub(super) mod node;

use crate::{
	Handle,
	HtminlError,
	Node,
	NodeInner,
	spec::{
		WhiteSpace,
		self,
	},
};
use html5ever::{
	Attribute,
	expanded_name,
	ns,
	local_name,
	ExpandedName,
	interface::{
		ElementFlags,
		NodeOrText,
		QuirksMode,
		TreeSink,
	},
	ParseOpts,
	QualName,
	tendril::{
		StrTendril,
		TendrilSink,
	},
	tree_builder::TreeBuilderOpts,
};
use indexmap::IndexMap;
use std::{
	borrow::Cow,
	io::Cursor,
	rc::Rc,
	cell::RefCell,
};



/// # Invalid Expanded Name.
///
/// This is used to avoid unfriendly panics in inapplicable `TreeSink` member
/// methods.
static NOOP_NAME: ExpandedName = expanded_name!("", "abbr");



#[derive(Debug, Clone)]
/// # HTML DOM Tree.
///
/// This struct mostly only exists as a place to chuck the ugly `TreeSink`
/// trait on. `Node` (or `Handle`) is self-referential, so is more or less
/// its own tree. Haha.
pub(crate) struct Tree {
	/// # Document Root.
	root: Handle,

	/// # Error.
	error: RefCell<Option<HtminlError>>,
}

impl Default for Tree {
	#[inline]
	/// # Default: Empty Root Document.
	fn default() -> Self {
		Self {
			root: Node::new(NodeInner::Document),
			error: RefCell::new(None),
		}
	}
}

impl TreeSink for Tree {
	type Handle = Handle;
	type Output = Self;
	type ElemName<'a> = ExpandedName<'a>
	where Self: 'a;

	/// # Add Attributes if Missing.
	///
	/// It's unclear if this is used by the tree builder or not, but if it is,
	/// it attaches the `new` attributes to `target`, except when previously
	/// defined.
	fn add_attrs_if_missing(&self, target: &Handle, new: Vec<Attribute>) {
		use indexmap::map::Entry;

		if let NodeInner::Element { ref attrs, .. } = target.inner {
			let attrs: &mut IndexMap<_, _> = &mut attrs.borrow_mut();

			for Attribute { name, value } in new {
				if let Entry::Vacant(e) = attrs.entry(name) { e.insert(value); }
			}
		}
	}

	/// # Append Node.
	///
	/// Attach a text or element child node to an existing (parent) node.
	///
	/// Other node types are ignored.
	fn append(&self, parent: &Handle, child: NodeOrText<Handle>) {
		match child {
			// Text nodes can always be added.
			NodeOrText::AppendText(v) =>
				// If the last node was text, merge them.
				if
					let Some(last) = parent.children.borrow().last() &&
					let NodeInner::Text { ref contents } = last.inner
				{
					contents.borrow_mut().push_tendril(&v);
				}
				// Otherwise add it anew.
				else {
					parent.children.borrow_mut().push(Node::new(NodeInner::Text {
						contents: RefCell::new(v)
					}));
				},

			// Among the other possible node types, we're only actually
			// interested in elements.
			NodeOrText::AppendNode(v) => if matches!(v.inner, NodeInner::Element { .. }) {
				parent.children.borrow_mut().push(v);
			},
		}
	}

	/// # Create Comment.
	///
	/// Return a generic placeholder node that will be ignored if appended.
	fn create_comment(&self, _text: StrTendril) -> Handle {
		Node::new(NodeInner::Ignored)
	}

	/// # Create Element.
	///
	/// Create and return a new element node.
	fn create_element(&self, name: QualName, attrs: Vec<Attribute>, flags: ElementFlags)
	-> Handle {
		let inner = NodeInner::Element {
			name,
			attrs: RefCell::new(attrs.into_iter().map(|v| (v.name, v.value)).collect())
		};

		// Fucking templates. Haha.
		let children = RefCell::new(
			if flags.template {
				vec![Node::new(NodeInner::Document)]
			}
			else { Vec::new() }
		);

		Rc::new(Node { inner, children })
	}

	/// # Create Processing Instruction.
	///
	/// Return a generic placeholder node that will be ignored if appended.
	fn create_pi(&self, _target: StrTendril, _data: StrTendril) -> Handle {
		Node::new(NodeInner::Ignored)
	}

	/// # Element Name.
	///
	/// Return an element node's fully qualified name. It is unclear if this
	/// is used by the tree builder or not.
	fn elem_name<'a>(&self, target: &'a Handle) -> ExpandedName<'a> {
		if let NodeInner::Element { ref name, .. } = target.inner {
			name.expanded()
		}
		else {
			debug_assert!(false, "BUG: elem_name called on non-element node.");
			self.error.borrow_mut().replace(HtminlError::Parse);
			NOOP_NAME
		}
	}

	#[inline]
	/// # Finish Parsing.
	///
	/// Hopefully the compiler will optimize this nonsense away. Haha.
	fn finish(self) -> Self { self }

	/// # Get Document Root.
	///
	/// Clone and return the root document.
	fn get_document(&self) -> Handle { Rc::clone(&self.root) }

	/// # Get Template Contents.
	///
	/// For whatever reason, `<template>` child nodes are treated as a new
	/// document instead of being attached as children.
	///
	/// This method returns a handle for them.
	fn get_template_contents(&self, target: &Handle) -> Handle {
		if
			let NodeInner::Element { ref name, .. } = target.inner &&
			matches!(name.ns, ns!(html)) &&
			matches!(name.local, local_name!("template")) &&
			let Some(out) = target.children.borrow().first()
		{
			return Rc::clone(out);
		}

		debug_assert!(false, "BUG: elem_name called on non-element node.");
		self.error.borrow_mut().replace(HtminlError::Parse);
		Node::new(NodeInner::Ignored)
	}

	/// # Set Parsing Error.
	fn parse_error(&self, _msg: Cow<'static, str>) {
		// There's no way to tell useful errors from stupid ones, so we
		// don't track them. Haha.
	}

	/// # Same Node?
	fn same_node(&self, x: &Handle, y: &Handle) -> bool { Rc::ptr_eq(x, y) }

	/// # Append Before Sibling.
	fn append_before_sibling(&self, _sibling: &Handle, _child: NodeOrText<Handle>) {
		debug_assert!(false, "BUG: append_before_sibling is unimplemented.");
		self.error.borrow_mut().replace(HtminlError::Parse);
	}

	/// # Append Based on Parent Node.
	fn append_based_on_parent_node(
		&self,
		_element: &Self::Handle,
		_prev_element: &Self::Handle,
		_child: NodeOrText<Self::Handle>,
	) {
		debug_assert!(false, "BUG: append_based_on_parent_node is unimplemented.");
		self.error.borrow_mut().replace(HtminlError::Parse);
	}

	/// # Append Doctype to Document.
	fn append_doctype_to_document(
		&self,
		_name: StrTendril,
		_public_id: StrTendril,
		_system_id: StrTendril,
	) {
		// We don't support doctype.
		debug_assert!(false, "BUG: append_doctype_to_document is unimplemented.");
	}

	/// # Is Mathml?
	fn is_mathml_annotation_xml_integration_point(&self, _target: &Handle) -> bool {
		debug_assert!(false, "BUG: is_mathml_annotation_xml_integration_point is unimplemented.");
		false
	}

	/// # Remove From Parent.
	fn remove_from_parent(&self, _target: &Handle) {
		debug_assert!(false, "BUG: remove_from_parent is unimplemented.");
		self.error.borrow_mut().replace(HtminlError::Parse);
	}

	/// # Reparent Children.
	fn reparent_children(&self, _node: &Handle, _new_parent: &Handle) {
		debug_assert!(false, "BUG: reparent_children is unimplemented.");
		self.error.borrow_mut().replace(HtminlError::Parse);
	}

	/// # Set Quirks Mode.
	fn set_quirks_mode(&self, _mode: QuirksMode) {
		debug_assert!(false, "BUG: set_quirks_mode is unimplemented.");
	}
}

impl Tree {
	/// # Parse Document.
	///
	/// Parse RAW HTML (as bytes) into a proper (minified) tree, returning it
	/// unless there's a show-stopping error of some kind.
	pub(crate) fn parse(raw: &[u8]) -> Result<Self, HtminlError> {
		// Since we aren't expecting anything other than HTML, we can skip the
		// doctype and save a tiny bit of overhead.
		let opts = ParseOpts {
			tree_builder: TreeBuilderOpts {
				drop_doctype: true,
				..TreeBuilderOpts::default()
			},
			..ParseOpts::default()
		};

		// Try to parse with our parser.
		let dom = html5ever::parse_document(Self::default(), opts)
			.from_utf8()
			.read_from(&mut Cursor::new(raw))
			.map_err(|_| HtminlError::Parse)?;

		if let Some(e) = dom.error.borrow_mut().take() {
			return Err(e);
		}

		dom.post_process();
		dom.minify();
		Ok(dom)
	}

	/// # Serialize Document.
	///
	/// Convert the tree back into an HTML string, returning it unless there
	/// are any show-stopping errors.
	pub(crate) fn serialize(&self, size_hint: Option<usize>)
	-> Result<String, HtminlError> {
		use std::fmt::Write;

		let size_hint = size_hint.unwrap_or(256);
		let mut out = String::with_capacity(size_hint);
		write!(&mut out, "{}", node::NodeDisplay::new(&self.root, None))
			.map_err(|_| HtminlError::Save)
			.map(|()| out)
	}

	/// # Post Processing.
	fn post_process(&self) {
		/// # Patch Tree.
		///
		/// (Lightly) clean the tree before returning it.
		fn walk(handle: &Handle) {
			if let NodeInner::Element { ref name, .. } = handle.inner {
				// Ensure void HTML elements are actually childless.
				if spec::is_void_html_tag(name) {
					handle.children.borrow_mut().truncate(0);
					return; // No children, no recursion. Bail early!
				}

				// The tree builder parses <template> content as a separate
				// document instead of regular children. Let's remove that
				// indirection as it isn't relevant or helpful for our
				// purposes.
				if
					matches!(name.ns, ns!(html)) &&
					matches!(name.local, local_name!("template"))
				{
					let children: &mut Vec<_> = &mut handle.children.borrow_mut();
					if
						let Some(first) = children.pop() &&
						matches!(first.inner, NodeInner::Document)
					{
						std::mem::swap(children, &mut first.children.borrow_mut());
					}
					// This shouldn't be reachable.
					else { children.truncate(0); }
				}
			}

			// Do the same for the children of the children.
			for child in handle.children.borrow().iter() { walk(child); }
		}

		walk(&self.root);
	}

	/// # Minify Text Nodes.
	fn minify(&self) {
		/// # Minify Node by Node.
		fn walk(handle: &Handle, ws: WhiteSpace) {
			// Maybe trim first/last text child.
			let try_trim = match handle.inner {
				NodeInner::Document => true,
				NodeInner::Element { ref name, .. } => spec::can_trim(name),
				_ => false,
			};
			if try_trim {
				let children: &mut Vec<_> = &mut handle.children.borrow_mut();
				if ! children.is_empty() {
					// Trim leading whitespace from first text child.
					if let NodeInner::Text { ref contents } = children[0].inner {
						let contents: &mut StrTendril = &mut contents.borrow_mut();
						let new: &str = contents.as_ref().trim_start();
						if new != contents.as_ref() {
							*contents = StrTendril::from(new);
						}
					}

					// Trim trailing whitespace from last text child.
					if let NodeInner::Text { ref contents } = children[children.len() - 1].inner {
						let contents: &mut StrTendril = &mut contents.borrow_mut();
						let new: &str = contents.as_ref().trim_end();
						if new != contents.as_ref() {
							*contents = StrTendril::from(new);
						}
					}
				}
			}

			// Strip unwanted children.
			handle.children.borrow_mut().retain(|v| match v.inner {
				// Keep and/or replace the text if non-empty, otherwise drop it.
				NodeInner::Text { ref contents } => {
					let mut contents = contents.borrow_mut();
					! contents.is_empty() && ws.process(contents.as_bytes()).is_none_or(|new|
						if new.is_empty() { false }
						else {
							*contents = new;
							true
						}
					)
				},

				// Recurse the children of elements.
				NodeInner::Element { ref name, ref attrs, .. } => {
					// Don't mess with scripts/styles that have a nonce.
					if
						matches!(name.local, local_name!("script") | local_name!("style")) &&
						attrs.borrow().contains_key(&QualName::new(
							None,
							ns!(html),
							local_name!("nonce"),
						))
					{
						return true;
					}

					// Recurse to strip their children.
					walk(v, WhiteSpace::from_element(name));
					true
				},

				// This shouldn't be reachable, but if for some reason it hits, recurse
				// same as if it were an element.
				NodeInner::Document => {
					walk(v, WhiteSpace::ROOT);
					true
				},

				// Ignored elements don't count.
				NodeInner::Ignored => false,
			});
		}

		walk(&self.root, WhiteSpace::ROOT);
	}
}



