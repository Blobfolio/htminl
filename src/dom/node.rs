/*!
# HTMinL: DOM Node.

This module includes the node-related business, but most of it is dedicated
to serialization/formatting.
*/

use crate::spec;
use html5ever::{
	local_name,
	ns,
	QualName,
	tendril::StrTendril,
};
use indexmap::IndexMap;
use std::{
	cell::RefCell,
	fmt,
	rc::Rc,
};



/// # Reference-Counted Node.
///
/// Nodes are self-referential, so generally need to be wrapped in `Rc`.
pub(crate) type Handle = Rc<Node>;



#[derive(Debug)]
/// # DOM Node.
///
/// This struct holds tag/attribute/content details for a node and its
/// children. At the root level, it's the whole damn tree.
///
/// In practice, most references hold `Handle` instead, which is an `Rc`-
/// wrapped version of `Node`.
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
///
/// This enum holds the details for a given node, differentiated by kind.
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



/// # Node Display.
///
/// This wrapper is used for serialization/display of a `Node` and its
/// children.
pub(super) struct NodeDisplay {
	/// # Parent Element (if any).
	parent: Option<QualName>,

	/// # The Current Object.
	node: Handle,
}

impl fmt::Display for NodeDisplay {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		use std::fmt::Write;

		match self.node.inner {
			// New document!
			NodeInner::Document => {
				// Write the DOCTYPE.
				f.write_str("<!DOCTYPE html>\n")?;

				// Recurse children.
				for child in self.node.children.borrow().iter() {
					<Self as fmt::Display>::fmt(&Self::new(child, None), f)?;
				}

				Ok(())
			},

			// An element.
			NodeInner::Element { ref name, ref attrs } => {
				if self.line_before_open() { f.write_char('\n')? }

				// Opening tag.
				write!(f, "<{}", name.local.as_ref())?;

				// Attribute(s).
				for (key, value) in attrs.borrow().iter() {
					<AttrDisplay as fmt::Display>::fmt(
						&AttrDisplay {
							tag: name,
							key,
							value: value.as_ref(),
						},
						f,
					)?;
				}

				// Self-closing SVG requires an extra `/`.
				if
					matches!(name.ns, ns!(svg)) &&
					! matches!(name.local, local_name!("svg")) &&
					self.node.children.borrow().is_empty()
				{
					// XML requires />
					return f.write_str("/>");
				}

				// Otherwise a `>` will do.
				f.write_char('>')?;

				// If this is a void HTML tag, we're done.
				if spec::is_void_html_tag(name) { return Ok(()); }

				// Recurse children.
				for child in self.node.children.borrow().iter() {
					<Self as fmt::Display>::fmt(&Self::new(child, Some(name.clone())), f)?;
				}

				// Move <body>/<html> closures to their own line, again for
				// readability.
				if
					matches!(name.ns, ns!(html)) &&
					matches!(name.local, local_name!("body") | local_name!("html"))
				{
					f.write_char('\n')?;
				}

				// Write the closing tag.
				write!(f, "</{}>", name.local.as_ref())
			},

			// Text node.
			NodeInner::Text { ref contents } => {
				let contents = contents.borrow();
				let v: &str = contents.as_ref();

				// Pass text through unchanged?
				if self.passthrough_text() { f.write_str(v) }
				// Escape it the usual way.
				else {
					<TextDisplay as fmt::Display>::fmt(&TextDisplay(v), f)
				}
			},

			// Don't care.
			NodeInner::Ignored => Ok(()),
		}
	}
}

impl NodeDisplay {
	#[must_use]
	/// # New.
	///
	/// Create and return a new display wrapper given the `node` and `children`.
	pub(super) fn new(node: &Handle, parent: Option<QualName>) -> Self {
		Self {
			parent,
			node: Rc::clone(node),
		}
	}

	#[must_use]
	/// # New Line Before Opening?
	///
	/// Direct children of `<html`> and `<body>` are given a new line to
	/// improve readability, at the cost of a couple extra bytes.
	const fn line_before_open(&self) -> bool {
		if let Some(parent) = self.parent.as_ref() {
			matches!(parent.ns, ns!(html)) &&
			matches!(parent.local, local_name!("body") | local_name!("html"))
		}
		else { false }
	}

	#[must_use]
	/// # Pass-Through Text?
	///
	/// Returns `true` if the parent element is one of the few requiring text
	/// be _unescaped_.
	const fn passthrough_text(&self) -> bool {
		if let Some(parent) = self.parent.as_ref() {
			matches!(parent.ns, ns!(html)) &&
			matches!(
				parent.local,
				local_name!("plaintext") |
				local_name!("script") |
				local_name!("style") |
				local_name!("xmp"),
			)
		}
		else { false }
	}
}



/// # Attribute Display.
///
/// This wrapper is used to write an opening element tag attribute.
struct AttrDisplay<'a> {
	/// # Element Tag.
	tag: &'a QualName,

	/// # Attribute Key.
	key: &'a QualName,

	/// # Attribute Value.
	value: &'a str,
}

impl fmt::Display for AttrDisplay<'_> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		use std::fmt::Write;

		// We can skip type="text/css" and type="text/javascript" on
		// style and script blocks, respectively.
		if
			matches!(self.key.ns, ns!()) &&
			matches!(self.key.local, local_name!("type")) &&
			match self.tag.local {
				local_name!("script") => self.value.eq_ignore_ascii_case("text/javascript"),
				local_name!("style") => self.value.eq_ignore_ascii_case("text/css"),
				_ => false,
			}
		{
			return Ok(());
		}

		// Handle (some) namespaces, and/or just add a leading space.
		match self.key.ns {
			ns!() => { f.write_char(' ')?; },
			ns!(xml) => { f.write_str(" xml:")?; },
			ns!(xmlns) =>
				if matches!(self.key.local, local_name!("xmlns")) { f.write_char(' ')?; }
				else { f.write_str(" xmlns:")?; },
			ns!(xlink) => { f.write_str(" xlink:")?; },
			// Unsupported?
			_ => return Err(fmt::Error),
		}

		// Push the key name.
		f.write_str(self.key.local.as_ref())?;

		// If this is a boolean HTML key, we're done.
		if self.is_boolean() { return Ok(()); }

		// Figure out the best quoting style for the value.
		let v = AttrValueDisplay::new(
			self.value,
			WhitespaceNormalization::new(self.tag, self.key),
		);
		if
			matches!(self.tag.ns, ns!(html)) &&
			matches!(self.key.ns, ns!()) &&
			v.is_empty() {
			return Ok(());
		}

		// Write it if we got it!
		<AttrValueDisplay as fmt::Display>::fmt(&v, f)
	}
}

impl AttrDisplay<'_> {
	#[expect(clippy::cognitive_complexity, reason = "That's what the macro's for. Haha.")]
	#[must_use]
	/// # Is HTML Boolean Attribute?
	///
	/// Returns `true` if this is a boolean HTML attribute, allowing the value
	/// to be elided as implied.
	const fn is_boolean(&self) -> bool {
		macro_rules! list {
			( $( $k:tt ( $( $el:tt )+ ), )+ ) => (
				match self.key.local {
					$(
						local_name!($k) => matches!(self.tag.local, $( local_name!($el) )|+),
					)+
					local_name!("hidden") => ! self.value.as_bytes().eq_ignore_ascii_case(b"until-found"),
					// "inert" is global too, but not in html5ever yet.
					local_name!("autofocus") | local_name!("itemscope") => true,
					_ => false,
				}
			);
		}

		matches!(self.tag.ns, ns!(html)) &&
		matches!(self.key.ns, ns!()) &&
		list! {
			"allowfullscreen"          ("iframe"),
			"async"                    ("script"),
			"autoplay"                 ("audio" "video"),
			"checked"                  ("input"),
			"controls"                 ("audio" "video"),
			"default"                  ("track"),
			"defer"                    ("script"),
			"disabled"                 ("button" "fieldset" "optgroup" "option" "select" "textarea" "input"),
			"formnovalidate"           ("button" "input"),
			"ismap"                    ("img"),
			"loop"                     ("audio" "video"),
			"multiple"                 ("input" "select"),
			"muted"                    ("audio" "video"),
			"nomodule"                 ("script"),
			"novalidate"               ("form"),
			"open"                     ("details" "dialog"),
			//"playsinline"              ("audio" "video"),
			"readonly"                 ("input" "textarea"),
			"required"                 ("input" "select" "textarea"),
			"reversed"                 ("ol"),
			"selected"                 ("option"),
			"shadowrootclonable"       ("template"),
			"shadowrootdelegatesfocus" ("template"),
			"shadowrootserializable"   ("template"),
		}
	}
}



#[derive(Clone, Copy, Eq, PartialEq)]
/// # Attribute Value Display Wrapper.
///
/// This wrapper is used to write an attribute value (for an opening tag),
/// including the leading `=`.
///
/// Quotes and whitespace touchups vary by context.
enum AttrValueDisplay<'a> {
	/// # Double Quoted.
	Double(&'a str, WhitespaceNormalization),

	/// # Single Quoted.
	Single(&'a str, WhitespaceNormalization),

	/// # Empty.
	Empty,
}

impl fmt::Display for AttrValueDisplay<'_> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		use std::fmt::Write;

		// Write the opening = bit.
		let (v, collapse, single) = match *self {
			Self::Double(v, ws) => {
				f.write_str("=\"")?;
				(v, ws.collapse(), false)
			},
			Self::Single(v, ws) => {
				f.write_str("='")?;
				(v, ws.collapse(), true)
			},
			Self::Empty => return f.write_str("=\"\""),
		};

		// Write the value, escaping problematic characters and/or normalizing
		// whitespace as needed.
		let mut in_ws = false;
		for c in v.chars() {
			match c {
				'\u{a0}' => {
					in_ws = false;
					f.write_str("&nbsp;")?;
				},
				'&' => {
					in_ws = false;
					f.write_str("&amp;")?;
				},
				'\'' if single => {
					in_ws = false;
					f.write_str("&#39;")?;
				},
				'"' if ! single => {
					in_ws = false;
					f.write_str("&#34;")?;
				},
				'\t' | '\n' | '\x0C' | ' ' if collapse => if ! in_ws {
					in_ws = true;
					f.write_char(' ')?;
				},
				c => {
					in_ws = false;
					f.write_char(c)?;
				},
			}
		}

		// Write the final quote to close it off.
		f.write_char(if single { '\'' } else { '"' })
	}
}

impl<'a> AttrValueDisplay<'a> {
	#[must_use]
	/// # New.
	///
	/// Determine the best quoting style for the attribute value and
	/// return a display wrapper that can be used to print it accordingly.
	const fn new(mut src: &'a str, ws: WhitespaceNormalization) -> Self {
		if ws.trim() { src = src.trim_ascii(); }
		if src.is_empty() { return Self::Empty; }

		// Figure out which style yields the shortest result.
		let mut double = 0;
		let mut single = 0;
		let mut bytes = src.as_bytes();
		while let [n, rest @ ..] = bytes {
			match *n {
				b'"' =>  { double += 1; },
				b'\'' => { single += 1; },
				_ => {},
			}
			bytes = rest;
		}

		// Prefer single if there are fewer of them.
		if single < double { Self::Single(src, ws) }
		// Otherwise stick with the default.
		else { Self::Double(src, ws) }
	}

	#[must_use]
	/// # Is Empty?
	const fn is_empty(&self) -> bool { matches!(self, Self::Empty) }
}



#[derive(Clone, Copy)]
/// # Escaped HTML Display Wrapper.
///
/// This wrapper is used to escape text for HTML contexts. Specifically, it
/// escapes non-breaking spaces, `&`, `<`, and `>`.
struct TextDisplay<'a>(&'a str);

impl fmt::Display for TextDisplay<'_> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		use std::fmt::Write;

		for c in self.0.chars() {
			match c {
				'\u{a0}' => { f.write_str("&nbsp;")?; },
				'&' =>      { f.write_str("&amp;")?; },
				'<' =>      { f.write_str("&lt;")?; },
				'>' =>      { f.write_str("&gt;")?; },
				_ =>        { f.write_char(c)?; },
			}
		}

		Ok(())
	}
}



#[derive(Clone, Copy, Eq, PartialEq)]
/// # Whitespace Normalization Strategy.
///
/// This is used by `AttrValueDisplay` to potentially minify/normalize the
/// whitespace in attribute value representations.
enum WhitespaceNormalization {
	/// # Leave As-Is.
	None,

	/// # Collapse/Trim.
	Collapse,

	/// # Trim.
	Trim,
}

impl WhitespaceNormalization {
	#[must_use]
	/// # From Tag/Key.
	const fn new(tag: &QualName, key: &QualName) -> Self {
		if matches!(key.ns, ns!()) {
			match tag.ns {
				ns!(html) => match key.local {
					local_name!("alt") |
					local_name!("class") |
					local_name!("height") |
					local_name!("href") |
					local_name!("id") |
					local_name!("sizes") |
					local_name!("src") |
					local_name!("srcset") |
					local_name!("title") |
					local_name!("width") => Self::Collapse,
					local_name!("style") => Self::Trim,
					_ =>                    Self::None,
				},
				ns!(svg) => match key.local {
					local_name!("class") |
					local_name!("fill") |
					local_name!("height") |
					local_name!("href") |
					local_name!("id") |
					local_name!("title") |
					local_name!("viewBox") |
					local_name!("width") |
					local_name!("xlink") => Self::Collapse,
					local_name!("style") => Self::Trim,
					_ =>                    Self::None,
				},
				_ => Self::None,
			}
		}
		else { Self::None }
	}

	#[must_use]
	/// # Trim?
	const fn trim(self) -> bool { matches!(self, Self::Collapse | Self::Trim) }

	#[must_use]
	/// # Collapse?
	const fn collapse(self) -> bool { matches!(self, Self::Collapse) }
}
