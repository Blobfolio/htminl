/*!
# HTMinL: DOM.
*/

pub(super) mod node;

use crate::{
	Handle,
	HtminlError,
	Node,
	NodeInner,
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
	/// Note: `Tree` is not optimized for this sort of thing, but the trait
	/// methods referencing this don't seem to be called often — or ever?! —
	/// so it's no big deal.
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
				if is_void_html_tag(name) {
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
		fn walk(handle: &Handle, ws: TextNormalization) {
			// Maybe trim first/last text child.
			let try_trim = match handle.inner {
				NodeInner::Document => true,
				NodeInner::Element { ref name, .. } => can_trim_first_last_text(name),
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
					! contents.is_empty() && ws.normalize(contents.as_bytes()).is_none_or(|new|
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
					walk(v, TextNormalization::new(name));
					true
				},

				// This shouldn't be reachable, but if for some reason it hits, recurse
				// same as if it were an element.
				NodeInner::Document => {
					walk(v, TextNormalization::Both);
					true
				},

				// Ignored elements don't count.
				NodeInner::Ignored => false,
			});
		}

		walk(&self.root, TextNormalization::Both);
	}
}



#[derive(Clone, Copy, Eq, PartialEq)]
/// # Whitespace Normalization.
enum TextNormalization {
	/// # Collapse Whitespace.
	Collapse,

	/// # Drop Whitespace-Only Text.
	Drop,

	/// # Collapse and Drop.
	Both,

	/// # Neither.
	None,
}

impl TextNormalization {
	#[must_use]
	/// # New.
	const fn new(tag: &QualName) -> Self {
		match tag.ns {
			// HTML is the main game, obviously.
			ns!(html) => {
				let collapse = can_collapse_whitespace(tag);
				let drop = can_drop_whitespace_text(tag);
				if collapse && drop { Self::Both }
				else if collapse { Self::Collapse }
				else if drop { Self::Drop }
				else { Self::None }
			},
			// We can do a _little_ bit of cleanup for SVGs.
			ns!(svg) => match tag.local {
				local_name!("defs") |
				local_name!("g") |
				local_name!("svg") |
				local_name!("symbol") => Self::Drop,
				_ => Self::None,
			},
			_ => Self::None,
		}
	}

	#[must_use]
	/// # Normalize Text.
	///
	/// Crunch the text according to the enabled options, returning a new
	/// value if different.
	fn normalize(self, raw: &[u8]) -> Option<StrTendril> {
		// Drop it if droppable!
		if self.drop() && is_whitespace(raw) {
			Some(StrTendril::new())
		}

		// Collapse if collapseable!
		else if
			self.collapse() &&
			let Some(new) = collapse(raw) &&
			new != raw &&
			let Ok(new) = String::from_utf8(new)
		{
			Some(StrTendril::from(new))
		}

		// Leave it be.
		else { None }
	}

	#[must_use]
	/// # Drop?
	const fn drop(self) -> bool { matches!(self, Self::Drop | Self::Both) }

	#[must_use]
	/// # Collapse?
	const fn collapse(self) -> bool { matches!(self, Self::Collapse | Self::Both) }
}



#[expect(clippy::too_many_lines, reason = "There are a lot of tags.")]
#[must_use]
/// # Can Collapse Child Text Whitespace?
///
/// Contiguous whitespace in these elements has no effect, so we can collapse
/// long strings to a single whitespace.
///
/// Note: this includes the union of the [MDN element list](https://developer.mozilla.org/en-US/docs/Web/HTML/Reference/Elements)
/// and `html5ever`'s [named "atoms"](https://github.com/servo/html5ever/blob/main/web_atoms/local_names.txt),
/// minus void elements, `code`, `plaintext`, `pre`, `script`, `style`,
/// and `textarea`.
///
/// Testing the negative would be a lot easier, but we specifically want to
/// exclude unrecognized/custom elements, since there's no telling how they're
/// meant to work.
const fn can_collapse_whitespace(tag: &QualName) -> bool {
	matches!(tag.ns, ns!(html)) &&
	matches!(
		tag.local,
		local_name!("a") |
		local_name!("abbr") |
		local_name!("acronym") |
		local_name!("address") |
		local_name!("applet") |
		local_name!("article") |
		local_name!("aside") |
		local_name!("audio") |
		local_name!("b") |
		local_name!("bdi") |
		local_name!("bdo") |
		local_name!("big") |
		local_name!("blink") |
		local_name!("blockquote") |
		local_name!("body") |
		local_name!("button") |
		local_name!("canvas") |
		local_name!("caption") |
		local_name!("center") |
		local_name!("cite") |
		local_name!("colgroup") |
		local_name!("content") |
		local_name!("data") |
		local_name!("datalist") |
		local_name!("dd") |
		local_name!("del") |
		local_name!("details") |
		local_name!("dfn") |
		local_name!("dialog") |
		local_name!("dir") |
		local_name!("div") |
		local_name!("dl") |
		local_name!("dt") |
		local_name!("em") |
		local_name!("fieldset") |
		local_name!("figcaption") |
		local_name!("figure") |
		local_name!("font") |
		local_name!("footer") |
		local_name!("form") |
		local_name!("frameset") |
		local_name!("h1") |
		local_name!("h2") |
		local_name!("h3") |
		local_name!("h4") |
		local_name!("h5") |
		local_name!("h6") |
		local_name!("head") |
		local_name!("header") |
		local_name!("hgroup") |
		local_name!("html") |
		local_name!("i") |
		local_name!("ins") |
		local_name!("isindex") |
		local_name!("kbd") |
		local_name!("label") |
		local_name!("legend") |
		local_name!("li") |
		local_name!("listing") |
		local_name!("main") |
		local_name!("map") |
		local_name!("mark") |
		local_name!("marquee") |
		local_name!("menu") |
		local_name!("menuitem") |
		local_name!("meter") |
		local_name!("nav") |
		local_name!("nobr") |
		local_name!("noembed") |
		local_name!("noframes") |
		local_name!("noscript") |
		local_name!("object") |
		local_name!("ol") |
		local_name!("optgroup") |
		local_name!("option") |
		local_name!("output") |
		local_name!("p") |
		local_name!("picture") |
		local_name!("progress") |
		local_name!("q") |
		local_name!("rb") |
		local_name!("rp") |
		local_name!("rt") |
		local_name!("rtc") |
		local_name!("ruby") |
		local_name!("s") |
		local_name!("samp") |
		local_name!("search") |
		local_name!("section") |
		local_name!("select") |
		local_name!("slot") |
		local_name!("small") |
		local_name!("span") |
		local_name!("strike") |
		local_name!("strong") |
		local_name!("sub") |
		local_name!("summary") |
		local_name!("sup") |
		local_name!("table") |
		local_name!("tbody") |
		local_name!("td") |
		local_name!("template") |
		local_name!("tfoot") |
		local_name!("th") |
		local_name!("thead") |
		local_name!("time") |
		local_name!("title") |
		local_name!("tr") |
		local_name!("tt") |
		local_name!("u") |
		local_name!("ul") |
		local_name!("var") |
		local_name!("video") |
		local_name!("xmp")
	)
}

#[must_use]
/// # Can Drop Text Nodes?
///
/// Whitespace-only text nodes in these elements serve no purpose and
/// can be safely removed.
const fn can_drop_whitespace_text(tag: &QualName) -> bool {
	matches!(tag.ns, ns!(html)) &&
	matches!(
		tag.local,
		local_name!("audio") |
		local_name!("body") |
		local_name!("head") |
		local_name!("html") |
		local_name!("optgroup") |
		local_name!("option") |
		local_name!("picture") |
		local_name!("select") |
		local_name!("table") |
		local_name!("tbody") |
		local_name!("template") |
		local_name!("tfoot") |
		local_name!("tr") |
		local_name!("video")
	)
}

#[must_use]
/// # Can Trim Child Text?
///
/// Returns `true` if it is safe to trim leading whitespace from the first
/// child, and trailing whitespace from the last, assuming either or both are
/// text nodes.
const fn can_trim_first_last_text(tag: &QualName) -> bool {
	can_drop_whitespace_text(tag) ||
	match tag.ns {
		ns!(html) => matches!(
			tag.local,
			local_name!("script") |
			local_name!("style") |
			local_name!("title")
		),
		ns!(svg) => matches!(
			tag.local,
			local_name!("desc") |
			local_name!("script") |
			local_name!("style") |
			local_name!("title")
		),
		_ => false,
	}
}

#[must_use]
/// Collapse Whitespace.
///
/// HTML rendering largely ignores whitespace, and at any rate treats all
/// types (other than the no-break space `\xA0`) the same way.
///
/// There is some nuance, but for most elements, we can safely convert all
/// contiguous sequences of (ASCII) whitespace to a single horizontal space
/// character.
fn collapse(txt: &[u8]) -> Option<Vec<u8>> {
	// Edge case: single whitespace.
	if txt.len() == 1 && matches!(txt[0], b'\t' | b'\n' | b'\x0C') {
		return Some(vec![b' ']);
	}

	// Find the first non-space whitespace, or pair of (any) whitespaces.
	let pos = txt.windows(2).position(|pair|
		matches!(pair[0], b'\t' | b'\n' | b'\x0C') ||
		(pair[0].is_ascii_whitespace() && pair[1].is_ascii_whitespace())
	)?;

	// Split at that location and start building up a replacement.
	let (a, rest) = txt.split_at(pos);
	let mut new = Vec::with_capacity(txt.len());
	new.extend_from_slice(a);

	let mut in_ws = false;
	for &b in rest {
		match b {
			b'\t' | b'\n' | b'\x0C' | b' ' => if ! in_ws {
				in_ws = true;
				new.push(b' ');
			},
			_ => {
				in_ws = false;
				new.push(b);
			},
		}
	}

	Some(new)
}

#[must_use]
/// # Is Void HTML Element?
const fn is_void_html_tag(tag: &QualName) -> bool {
	matches!(tag.ns, ns!(html)) &&
	matches!(
		tag.local,
		local_name!("area") |
		local_name!("base") |
		local_name!("basefont") |
		local_name!("bgsound") |
		local_name!("br") |
		local_name!("col") |
		local_name!("embed") |
		local_name!("frame") |
		local_name!("hr") |
		local_name!("iframe") |
		local_name!("img") |
		local_name!("input") |
		local_name!("keygen") |
		local_name!("link") |
		local_name!("meta") |
		local_name!("param") |
		local_name!("source") |
		local_name!("track") |
		local_name!("wbr")
	)
}

#[must_use]
/// Is (Only) Whitespace?
///
/// Returns `true` if the node is empty or contains only whitespace.
///
/// Note that CR is replaced with LF prior to parsing, so there's no need
/// to include `b'\r'` in the matchset.
const fn is_whitespace(mut txt: &[u8]) -> bool {
	while let [b'\t' | b'\n' | b'\x0C' | b' ', rest @ ..] = txt { txt = rest; }
	txt.is_empty()
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

	#[test]
	fn t_can_collapse_whitespace() {
		// The collapseable list is really big, but should _not_ include
		// these things.
		for i in [
			local_name!("code"),
			local_name!("plaintext"),
			local_name!("pre"),
			local_name!("script"),
			local_name!("style"),
			local_name!("svg"),
			local_name!("textarea"),
		] {
			let name = QualName::new(None, ns!(html), i);
			assert!(! can_collapse_whitespace(&name));
		}
	}

	#[test]
	fn t_collapse() {
		for (lhs, rhs) in [
			(&b"raw"[..], None),
			(b" ", None),
			(b"  ", Some(vec![b' '])),
			(b"   ", Some(vec![b' '])),
			(b"\n", Some(vec![b' '])),
			(b"hello world", None),
			(b"hello\nworld", Some(b"hello world".to_vec())),
			(b"hello \x0C \t\nworld", Some(b"hello world".to_vec())),
			(b"hello\x0C \t\nworld, hello  moon", Some(b"hello world, hello moon".to_vec())),
		] {
			assert_eq!(collapse(lhs), rhs);
		}
	}

	#[test]
	fn t_is_whitespace() {
		assert!(is_whitespace(b""));
		assert!(is_whitespace(b"  \t\n  \x0C"));
		assert!(! is_whitespace(b"  a "));
	}
}
