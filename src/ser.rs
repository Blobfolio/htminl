/*!
# HTMinL: Serialization
*/

use crate::{
	Handle,
	NodeInner,
	spec,
	Tree,
};
use html5ever::{
	interface::TreeSink,
	local_name,
	ns,
	QualName,
	serialize::AttrRef,
};
use std::{
	borrow::Cow,
	collections::VecDeque,
	rc::Rc,
};



/// # HTML5 Doctype.
const DOCTYPE: &[u8] = b"<!DOCTYPE html>\n";



/// Back to HTML!
///
/// Serialize the tree back into a (hopefully!) valid HTML text document,
/// returning the result if successful.
///
/// This borrows heavily from `markup5ever_rcdom::SerializableHandle`, but
/// doesn't actually leverage the `Serialize`/`r` traits.
pub(crate) fn serialize(dom: &Tree, size_hint: usize) -> Option<Vec<u8>> {
	#[derive(Debug)]
	enum Stage {
		/// # Open Tag.
		Open(Handle),

		/// # Close Tag.
		Close(QualName),
	}

	// What we're writing to.
	let mut serializer = Serializer::new(size_hint);

	// A stack to work with.
	let mut stack = VecDeque::new();
	stack.extend(
		dom.get_document().children.borrow()
			.iter()
			.map(|h| Stage::Open(Rc::clone(h)))
	);

	while let Some(op) = stack.pop_front() {
		match op {
			// New tag!
			Stage::Open(handle) => match handle.inner {
				NodeInner::Element {
					ref name,
					ref attrs,
					..
				} => {
					serializer.start_elem(
						name,
						attrs.borrow().iter().map(|(k, v)| (k, v.as_ref())),
						! handle.children.borrow().is_empty(),
					)?;

					stack.reserve(1 + handle.children.borrow().len());
					stack.push_front(Stage::Close(name.clone()));

					for child in handle.children.borrow().iter().rev() {
						stack.push_front(Stage::Open(Rc::clone(child)));
					}
				},
				NodeInner::Text { ref contents } => serializer.write_text(&contents.borrow())?,

				// Unused.
				NodeInner::Document |
				NodeInner::Ignored => {},
			},

			// Close it.
			Stage::Close(name) => { serializer.end_elem(&name)?; },
		}
	}

	Some(serializer.writer)
}



#[derive(Debug, Clone, Copy)]
/// # Element Details.
///
/// This struct holds top-level tag details for `Serializer`. It's only
/// really used to be able to track a given object's parent.
struct ParentTag {
	/// # Line Break for Children.
	///
	/// To make the document slightly more readable, direct descendents of
	/// `<html>` and `<body>` tags are given a fresh line.
	child_lines: bool,

	/// # Unescaped Text?
	///
	/// Text nodes are passed through _without_ the usual escaping for
	/// `<plaintext>`, `<script>`, `<style>`, and `<xmp>` tags.
	plain_text: bool,

	/// # Self-Closing?
	///
	/// This is `true` for "void" HTML tags, but also for any SVG child tags
	/// that have no children.
	void: bool,
}

impl ParentTag {
	#[must_use]
	/// # New.
	const fn new(tag: &QualName, void: bool) -> Self {
		let mut child_lines = false;
		let mut plain_text = false;

		if matches!(tag.ns, ns!(html)) {
			match tag.local {
				local_name!("body") |
				local_name!("html") => { child_lines = true; },

				local_name!("plaintext") |
				local_name!("script") |
				local_name!("style") |
				local_name!("xmp") => { plain_text = true; },

				_ => {},
			}
		}

		Self { child_lines, plain_text, void }
	}
}



#[derive(Debug, Copy, Clone, Default, Eq, PartialEq)]
/// # Quote Type
///
/// If an attribute value contains the same character used for quoting, it has
/// to be encoded, jumping from one to five bytes per occurrence.
///
/// Space can often be saved in such cases by wrapping the value with single
/// quotes instead of the usual double.
///
/// (Serialization will pick whichever works out to be smallest for each
/// value.)
enum QuoteKind {
	#[default]
	/// # Double (") Quotes.
	Double,

	/// # Single (') Quotes.
	Single,
}

impl QuoteKind {
	#[must_use]
	/// # For Attribute Value.
	const fn for_value(mut src: &[u8]) -> Self {
		let mut double = 0;
		let mut single = 0;

		while let [n, rest @ ..] = src {
			match *n {
				b'"' => { double += 1; },
				b'\'' => { single += 1; },
				_ => {},
			}
			src = rest;
		}

		// Prefer single if there are fewer of them.
		if single < double { Self::Single }
		// Otherwise stick with the default.
		else { Self::Double }
	}
}



/// Minification Serializer
///
/// This is roughly based on `html5ever::serialize::Serializer`, but doesn't
/// actually implement the trait.
struct Serializer {
	/// # Writer.
	writer: Vec<u8>,

	/// # Stack.
	stack: Vec<ParentTag>,
}

impl Serializer {
	#[must_use]
	/// # New Instance.
	pub(crate) fn new(size_hint: usize) -> Self {
		let mut writer = Vec::with_capacity(size_hint);
		writer.extend_from_slice(DOCTYPE);
		Self {
			writer,
			stack: vec![ParentTag {
				child_lines: false,
				plain_text: false,
				void: false,
			}],
		}
	}

	#[must_use]
	/// # Parent Element.
	const fn parent(&self) -> Option<ParentTag> {
		debug_assert!(! self.stack.is_empty(), "BUG: No parent element?!");
		self.stack.as_slice().last().copied()
	}

	/// # Write Escaped Text Node.
	///
	/// XML/HTML text requires escaping `&`, `<`, and `>`.
	///
	/// This method also converts literal non-breaking space characters to
	/// `&nbsp;` for clarity, since most readers won't make it clear that
	/// it's irregular.
	fn write_esc_text(&mut self, txt: &[u8]) {
		let mut idx: usize = 0;
		let len: usize = txt.len();

		while idx < len {
			match txt[idx] {
				// Non-breaking space.
				194_u8 if idx + 1 < len && txt[idx + 1] == 160_u8 => {
					idx += 1;
					self.writer.extend_from_slice(b"&nbsp;");
				},
				b'&' => { self.writer.extend_from_slice(b"&amp;"); },
				b'<' => { self.writer.extend_from_slice(b"&lt;"); },
				b'>' => { self.writer.extend_from_slice(b"&gt;"); },
				c =>    { self.writer.push(c); },
			}

			idx += 1;
		}
	}

	/// # Write Escaped Attr.
	///
	/// HTML attributes require escaping of `&` and the wrapping character, if
	/// any.
	///
	/// This method will pick the most compact quoting style, and escape
	/// accordingly.
	fn write_esc_attr(&mut self, txt: &[u8]) {
		// Easy abort: empty tags.
		if txt.is_empty() {
			self.writer.extend_from_slice(b"=\"\"");
			return;
		}

		// Single or double quotes?
		let single = matches!(QuoteKind::for_value(txt), QuoteKind::Single);

		if single { self.writer.extend_from_slice(b"='"); }
		else { self.writer.extend_from_slice(b"=\""); }

		let mut idx: usize = 0;
		let len: usize = txt.len();
		while idx < len {
			match txt[idx] {
				194_u8 if idx + 1 < len && txt[idx + 1] == 160_u8 => {
					idx += 1;
					self.writer.extend_from_slice(b"&nbsp;");
				},
				b'&' => { self.writer.extend_from_slice(b"&amp;"); },
				b'\'' if single => { self.writer.extend_from_slice(b"&#39;"); },
				b'"' if ! single => { self.writer.extend_from_slice(b"&#34;"); },
				c => { self.writer.push(c); },
			}

			idx += 1;
		}

		if single { self.writer.push(b'\''); }
		else { self.writer.push(b'\"'); }
	}
}

impl Serializer {
	#[must_use]
	/// # Write Opening Tag.
	///
	/// Most minification will have already occurred, but serialization
	/// performs a few more optimizations on-the-fly:
	///
	/// * Childless SVG elements are self-closed;
	/// * Empty attribute values are omitted;
	/// * Default `style` and `script` types are omitted;
	/// * Quote attribute values with single quotes if that save space over double quotes.
	fn start_elem<'a, AttrIter>(
		&mut self,
		tag: &QualName,
		attrs: AttrIter,
		has_children: bool,
	) -> Option<()>
	where AttrIter: Iterator<Item = AttrRef<'a>> {
		// There should be a non-void parent element or what are we even doing
		// here?!
		let parent = self.parent()?;
		if parent.void {
			self.stack.push(ParentTag::new(tag, true));
			return Some(());
		}

		// Move direct children of <html>/<body> to a fresh line.
		if parent.child_lines { self.writer.push(b'\n'); }

		// Opening tag.
		self.writer.push(b'<');
		self.writer.extend_from_slice(tag.local.as_bytes());

		// Attribute(s).
		for (key, value) in attrs {
			self.write_attr(tag, key, value.as_bytes())?;
		}

		// Finish the tag, and figure out if it's self-closing.
		let void =
			if
				! has_children &&
				matches!(tag.ns, ns!(svg)) &&
				! matches!(tag.local, local_name!("svg"))
			{
				// XML requires />
				self.writer.extend_from_slice(b"/>");
				true
			}
			else {
				self.writer.push(b'>');
				spec::is_void_html_tag(tag)
			};

		// Stack it.
		self.stack.push(ParentTag::new(tag, void));

		Some(())
	}

	#[must_use]
	/// # Write Closing Tag.
	///
	/// Note that for self-closing tags, the work will have already been done.
	fn end_elem(&mut self, name: &QualName) -> Option<()> {
		// We only need to close for the children.
		if ! self.stack.pop()?.void {
			// Move <body>/<html> closures to their own line.
			if
				matches!(name.ns, ns!(html)) &&
				matches!(name.local, local_name!("body") | local_name!("html"))
			{
				self.writer.push(b'\n');
			}

			self.writer.extend_from_slice(b"</");
			self.writer.extend_from_slice(name.local.as_bytes());
			self.writer.push(b'>');
		}

		Some(())
	}

	#[must_use]
	/// # Write Tag Attribute.
	fn write_attr(&mut self, tag: &QualName, key: &QualName, value: &[u8]) -> Option<()> {
		// We can skip type="text/css" and type="text/javascript" on
		// style and script blocks, respectively.
		if
			matches!(key.local, local_name!("type")) &&
			(
				(matches!(tag.local, local_name!("script")) && value.eq_ignore_ascii_case(b"text/javascript")) ||
				(matches!(tag.local, local_name!("style")) && value.eq_ignore_ascii_case(b"text/css"))
			)
		{ return Some(()); }

		// Handle (some) namespaces, and/or just add a leading space.
		match key.ns {
			ns!() => self.writer.push(b' '),
			ns!(xml) => self.writer.extend_from_slice(b" xml:"),
			ns!(xmlns) =>
				if matches!(key.local, local_name!("xmlns")) { self.writer.push(b' '); }
				else { self.writer.extend_from_slice(b" xmlns:"); },
			ns!(xlink) => self.writer.extend_from_slice(b" xlink:"),
			// Unsupported?
			_ => return None,
		}

		// Push the key name.
		self.writer.extend_from_slice(key.local.as_bytes());

		// We might be able to minify the attribute value.
		let value: Cow<[u8]> = match key.local {
			// Boolean attributes don't need values in HTML contexts.
			local_name!("allowfullscreen") |
			local_name!("async") |
			local_name!("autofocus") |
			local_name!("autoplay") |
			local_name!("checked") |
			local_name!("controls") |
			local_name!("default") |
			local_name!("defer") |
			local_name!("disabled") |
			local_name!("formnovalidate") |
			// local_name!("inert") | // Not yet supported.
			local_name!("ismap") |
			local_name!("itemscope") |
			local_name!("loop") |
			local_name!("multiple") |
			local_name!("muted") |
			local_name!("nomodule") |
			local_name!("novalidate") |
			local_name!("open") |
			// local_name!("playsinline") | // Not yet supported.
			local_name!("readonly") |
			local_name!("required") |
			local_name!("reversed") |
			local_name!("selected") |
			local_name!("shadowrootclonable") |
			local_name!("shadowrootdelegatesfocus") |
			local_name!("shadowrootserializable") =>
				// For HTML, we can skip the value if it's empty, "true",
				// or matches the key name.
				if
					matches!(tag.ns, ns!(html)) &&
					(
						value.trim_ascii().is_empty() ||
						value.eq_ignore_ascii_case(key.local.as_bytes()) ||
						value.eq_ignore_ascii_case(b"true")
					)
				{ Cow::Borrowed(&[]) }
				// For other contexts, leave it alone.
				else { Cow::Borrowed(value) },

			// Classes can have whitespace trimmed/collapsed.
			local_name!("class") => {
				let value = value.trim_ascii();
				spec::collapse(value).map_or(
					Cow::Borrowed(value),
					Cow::Owned,
				)
			},
			_ => Cow::Borrowed(value),
		};

		// If we have a value, write it!
		if ! value.is_empty() || ! matches!(tag.ns, ns!(html)) {
			self.write_esc_attr(&value);
		}

		Some(())
	}

	#[must_use]
	/// # Write Text.
	fn write_text(&mut self, txt: &str) -> Option<()> {
		// Pass it through.
		if self.parent()?.plain_text {
			self.writer.extend_from_slice(txt.as_bytes());
		}
		// Escape `&`, `<`, and `>`.
		else { self.write_esc_text(txt.as_bytes()); }

		Some(())
	}
}
