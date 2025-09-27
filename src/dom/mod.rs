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
	/// If `target` is an element, attach the new attributes to it, except
	/// when they'd collide with existing entries.
	///
	/// Note: this isn't usually used.
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

	/// # Append Based on Parent Node.
	///
	/// Insert `child` before `sibling` if `sibling` has a parent, otherwise
	/// append it to `last_parent`.
	///
	/// Note: this isn't usually used.
	fn append_based_on_parent_node(
		&self,
		sibling: &Handle,
		last_parent: &Handle,
		child: NodeOrText<Self::Handle>,
	) {
		if self.find_node_parent_and_index(sibling).is_some() {
			self.append_before_sibling(sibling, child);
		}
		else { self.append(last_parent, child); }
	}

	/// # Append Before Sibling.
	///
	/// Note: this isn't usually used.
	fn append_before_sibling(&self, sibling: &Handle, child: NodeOrText<Handle>) {
		let Some((parent, pos)) = self.find_node_parent_and_index(sibling) else {
			self.error.borrow_mut().replace(HtminlError::Parse);
			return;
		};

		// Unwrap the children.
		let children: &mut Vec<_> = &mut parent.children.borrow_mut();
		if children.len() <= pos {
			self.error.borrow_mut().replace(HtminlError::Parse);
			return;
		}

		match child {
			// Text nodes can always be added.
			NodeOrText::AppendText(v) =>
				// If the previous node was text, merge them.
				if
					pos != 0 &&
					let NodeInner::Text { ref contents } = children[pos - 1].inner
				{
					contents.borrow_mut().push_tendril(&v);
				}
				// Otherwise add it anew.
				else {
					children.insert(pos, Node::new(NodeInner::Text {
						contents: RefCell::new(v)
					}));
				},

			// Among the other possible node types, we're only actually
			// interested in elements.
			NodeOrText::AppendNode(v) => if matches!(v.inner, NodeInner::Element { .. }) {
				children.insert(pos - 1, v);
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

	/// # Remove From Parent.
	///
	/// Note: this isn't usually used.
	fn remove_from_parent(&self, target: &Handle) {
		if let Some((parent, pos)) = self.find_node_parent_and_index(target) {
			let children: &mut Vec<_> = &mut parent.children.borrow_mut();
			if pos < children.len() { children.remove(pos); }
		}
	}

	/// # Reparent Children.
	///
	/// Drain and append all children from `old_parent` onto `new_parent`.
	///
	/// Note: this isn't usually used.
	fn reparent_children(&self, old_parent: &Handle, new_parent: &Handle) {
		let old_children: &mut Vec<_> = &mut old_parent.children.borrow_mut();
		let new_children: &mut Vec<_> = &mut new_parent.children.borrow_mut();
		new_children.append(old_children);
	}

	/// # Same Node?
	fn same_node(&self, x: &Handle, y: &Handle) -> bool { Rc::ptr_eq(x, y) }

	/// # Append Doctype to Document.
	fn append_doctype_to_document(
		&self,
		_name: StrTendril,
		_public_id: StrTendril,
		_system_id: StrTendril,
	) {
		// Noop.
	}

	/// # Is Mathml?
	///
	/// We don't support mathml, so always return false.
	fn is_mathml_annotation_xml_integration_point(&self, _node: &Handle) -> bool {
		false
	}

	/// # Set Parsing Error.
	fn parse_error(&self, _msg: Cow<'static, str>) {
		// Noop.
	}

	/// # Set Quirks Mode.
	fn set_quirks_mode(&self, _mode: QuirksMode) {
		// Noop.
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

	#[must_use]
	/// # Find Node.
	///
	/// Search the tree for `target`, returning its parent and position in
	/// `parent.children` if found.
	///
	/// Note the struct is _not_ optimized for this sort of operation, but
	/// it doesn't usually — or ever? — come up during parsing, and we don't
	/// use it ourselves.
	fn find_node_parent_and_index(&self, target: &Handle) -> Option<(Handle, usize)> {
		/// # Find and Delete.
		///
		/// Note that our `Tree` structure is not optimized for this, but it
		/// isn't normally called.
		fn walk(handle: &Handle, target: &Handle) -> Option<(Handle, usize)> {
			let children: &mut Vec<_> = &mut handle.children.borrow_mut();
			if let Some(pos) = children.iter().position(|v| Rc::ptr_eq(v, target)) {
				return Some((Rc::clone(handle), pos));
			}

			// Recurse.
			for child in children {
				if let Some(out) = walk(child, target) { return Some(out); }
			}

			None
		}

		walk(&self.root, target)
	}

	/// # Post Processing.
	///
	/// (Lightly) clean the tree before returning it.
	///
	/// Specifically, this ensures that void HTML elements really have no
	/// children, and fixes `<template>` child element associations.
	fn post_process(&self) {
		/// # Patch Tree.
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



#[cfg(test)]
mod tests {
	use super::*;

	/// # Predictable Test Tree.
	const HTML: &[u8] = b"\
	<html>\
		<head></head>\
		<body>\
			<div>\
				<span></span>\
			</div>\
		</body>\
	</html>";

	#[test]
	fn t_remove_from_parent() {
		// Parse a simple document.
		let tree = Tree::parse(HTML).expect("Tree parse failed.");

		// Find the span.
		let target = Rc::clone(
			&tree.root.children.borrow()[0]
				.children.borrow()[1]
				.children.borrow()[0]
				.children.borrow()[0]
		);
		let NodeInner::Element { ref name, .. } = target.inner else {
			panic!("Wrong element.");
		};
		assert_eq!(name.local, local_name!("span"));

		// Remove the span from the tree.
		tree.remove_from_parent(&target);

		// The div should have no children now.
		assert!(
			tree.root.children.borrow()[0]
				.children.borrow()[1]
				.children.borrow()[0]
				.children.borrow().is_empty()
		);
	}

	#[test]
	fn t_append_before_sibling() {
		// Parse a simple document.
		let tree = Tree::parse(HTML).expect("Tree parse failed.");

		// Find the span.
		let target = Rc::clone(
			&tree.root.children.borrow()[0]
				.children.borrow()[1]
				.children.borrow()[0]
				.children.borrow()[0]
		);
		let NodeInner::Element { ref name, .. } = target.inner else {
			panic!("Wrong element.");
		};
		assert_eq!(name.local, local_name!("span"));

		// Let's add a text element before it.
		let new = NodeOrText::AppendText(StrTendril::from("Hello World"));
		tree.append_before_sibling(&target, new);

		// The div should have no children now.
		assert!(matches!(
			tree.root.children.borrow()[0]
				.children.borrow()[1]
				.children.borrow()[0]
				.children.borrow()[0].inner,
			NodeInner::Text { .. },
		));
	}
}
