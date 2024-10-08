/*!
# HTML Library: Minification Serializer

This serializer is almost identical to the one used by `html5ever`, but
includes a few space-saving optimizations.
*/

use crate::HtminlError;
use marked::{
	html5ever::{
		local_name,
		namespace_url,
		ns,
		serialize::{
			AttrRef,
			Serialize,
			Serializer,
			TraversalScope,
		},
	},
	LocalName,
	QualName,
};
use std::{
	default::Default,
	io::{
		self,
		Write,
	},
};



/// Serialize W/ Serializer
///
/// This is a convenience method for serializing a node with our particular
/// serializer implementation.
pub(crate) fn serialize<T>(writer: &mut Vec<u8>, node: &T) -> Result<(), HtminlError>
where
	T: Serialize,
{
	let mut ser = MinifySerializer::new(writer);
	node.serialize(&mut ser, TraversalScope::ChildrenOnly(None))
		.map_err(|_| HtminlError::Parse)
}



#[derive(Default)]
/// Element Info.
///
/// Imported from `html5ever`.
struct ElemInfo {
	/// # Name.
	html_name: Option<LocalName>,

	/// # Ignore Children?
	ignore_children: bool,
}



#[derive(Debug, Copy, Clone, Default, Eq, PartialEq)]
/// Quote Type
enum QuoteKind {
	/// # No quotes.
	None,

	/// # Double (") Quotes.
	Double,

	/// # Single (') Quotes.
	Single,

	#[default]
	/// # Nothing to quote at all!
	Void,
}

impl From<&[u8]> for QuoteKind {
	fn from(txt: &[u8]) -> Self {
		let mut none_ok: bool = true;
		let mut double: u32 = 0;
		let mut single: u32 = 0;

		// Loop through the bytes to count the quotes and see if there are any
		// characters which might require quoting during write.
		//
		// While the HTML spec technically allows most anything without
		// whitespace to be expressed without quotes, we're only going to
		// propose nudity in cases where the whole string is made up of ASCII
		// alphanumeric characters and/or `#,-.:?@_`.
		txt.iter().for_each(|c| match *c {
			b'"' => { double += 1; },
			b'\'' => { single += 1; },
			b'#'
			| b','..=b'/'
			| b':'
			| b'?'
			| b'@'
			| b'_'
			| b'0'..=b'9'
			| b'A'..=b'Z'
			| b'a'..=b'z' => {},
			_ => if none_ok { none_ok = false; },
		});

		// There's nothing requiring quotes!
		if none_ok && double == 0 && single == 0 { Self::None }
		// There are fewer single quotes than double quotes, so wrapping in
		// single will be more efficient.
		else if 0 < single && single < double { Self::Single }
		// Default to double quotes.
		else { Self::Double }
	}
}



/// Minification Serializer
///
/// This is based on `html5ever`'s `HtmlSerializer` and works largely the same
/// way, except some byte-saving routines are employed to reduce the output
/// size.
struct MinifySerializer<Wr: Write> {
	/// # Writer.
	pub(crate) writer: Wr,

	/// # Stack.
	stack: Vec<ElemInfo>,
}

impl<Wr: Write> MinifySerializer<Wr> {
	/// New Instance.
	///
	/// Imported from `html5ever`.
	pub(crate) fn new(writer: Wr) -> Self {
		Self {
			writer,
			stack: vec![ElemInfo {
				html_name: None,
				ignore_children: false,
			}],
		}
	}

	/// Parent.
	///
	/// Imported from `html5ever`.
	fn parent(&mut self) -> &mut ElemInfo {
		assert!(! self.stack.is_empty(), "No parent ElemInfo.");
		self.stack.last_mut().unwrap()
	}

	/// Write Escaped Text Node.
	///
	/// HTML text requires escaping `&`, `<`, and `>`.
	fn write_esc_text(&mut self, txt: &[u8]) -> io::Result<()> {
		let mut idx: usize = 0;
		let len: usize = txt.len();

		while idx < len {
			match txt[idx] {
				194_u8 if idx + 1 < len && txt[idx + 1] == 160_u8 => {
					idx += 1;
					self.writer.write_all(b"&nbsp;")
				},
				b'&' => self.writer.write_all(b"&amp;"),
				b'<' => self.writer.write_all(b"&lt;"),
				b'>' => self.writer.write_all(b"&gt;"),
				c => self.writer.write_all(&[c]),
			}?;

			idx += 1;
		}

		Ok(())
	}

	/// Write Escaped Attr.
	///
	/// HTML attributes require escaping of `&` and the wrapping character, if
	/// any.
	///
	/// This method will pick the most compact quoting style, and escape
	/// accordingly. (Empty values will have been weeded out before reaching
	/// this point.)
	fn write_esc_attr(&mut self, txt: &[u8]) -> io::Result<QuoteKind> {
		match QuoteKind::from(txt) {
			QuoteKind::None => {
				self.writer.write_all(b"=")?;
				self.writer.write_all(txt)?;
				Ok(QuoteKind::None)
			},
			QuoteKind::Single => {
				self.writer.write_all(b"='")?;

				let mut idx: usize = 0;
				let len: usize = txt.len();
				while idx < len {
					match txt[idx] {
						194_u8 if idx + 1 < len && txt[idx + 1] == 160_u8 => {
							idx += 1;
							self.writer.write_all(b"&nbsp;")
						},
						b'&' => self.writer.write_all(b"&amp;"),
						b'\'' => self.writer.write_all(b"&#39;"),
						c => self.writer.write_all(&[c]),
					}?;

					idx += 1;
				}

				self.writer.write_all(b"'")?;
				Ok(QuoteKind::Single)
			},
			_ => {
				self.writer.write_all(b"=\"")?;

				let mut idx: usize = 0;
				let len: usize = txt.len();
				while idx < len {
					match txt[idx] {
						194_u8 if idx + 1 < len && txt[idx + 1] == 160_u8 => {
							idx += 1;
							self.writer.write_all(b"&nbsp;")
						},
						b'&' => self.writer.write_all(b"&amp;"),
						b'"' => self.writer.write_all(b"&#34;"),
						c => self.writer.write_all(&[c]),
					}?;

					idx += 1;
				}

				self.writer.write_all(b"\"")?;
				Ok(QuoteKind::Double)
			}
		}
	}
}

impl<Wr: Write> Serializer for MinifySerializer<Wr> {
	/// Write Opening Tag.
	///
	/// This differs from `html5ever`'s version in that:
	/// * SVG `<path>` elements are self-closed with an XML slash;
	/// * Empty attribute values are omitted;
	/// * Attribute values that can go unquoted go unquoted;
	/// * Attribute values that can be written more compactly with single quotes go single-quoted;
	fn start_elem<'a, AttrIter>(&mut self, name: QualName, attrs: AttrIter) -> io::Result<()>
	where AttrIter: Iterator<Item = AttrRef<'a>> {
		let html_name = match name.ns {
			ns!(html) => Some(name.local.clone()),
			_ => None,
		};

		// Abort: the parent has no children.
		if self.parent().ignore_children {
			self.stack.push(ElemInfo {
				html_name,
				ignore_children: true,
			});
			return Ok(());
		}

		self.writer.write_all(b"<")?;
		self.writer.write_all(name.local.as_bytes())?;

		let mut last_quote = QuoteKind::Void;
		for (name, value) in attrs {
			self.writer.write_all(b" ")?;

			match name.ns {
				ns!() => (),
				ns!(xml) => self.writer.write_all(b"xml:")?,
				ns!(xmlns) => {
					if name.local != local_name!("xmlns") {
						self.writer.write_all(b"xmlns:")?;
					}
				},
				ns!(xlink) => self.writer.write_all(b"xlink:")?,
				_ => {
					self.writer.write_all(b"unknown_namespace:")?;
				},
			}

			self.writer.write_all(name.local.as_bytes())?;

			// Only write values if they exist.
			if ! value.is_empty() {
				last_quote = self.write_esc_attr(value.as_bytes())?;
			}
		}

		// SVG <path> tags should be self-closing in XML-style.
		let is_svg_path: bool = name.local == local_name!("path");
		if is_svg_path {
			// We don't want the slash mistaken for part of the value.
			if last_quote == QuoteKind::None {
				self.writer.write_all(b" />")?;
			}
			else {
				self.writer.write_all(b"/>")?;
			}
		}
		else {
			self.writer.write_all(b">")?;
		}

		// Ignore children?
		let ignore_children =
			is_svg_path ||
			(
				name.ns == ns!(html) &&
				matches!(
					name.local,
					local_name!("area") |
					local_name!("base") |
					local_name!("basefont") |
					local_name!("bgsound") |
					local_name!("br") |
					local_name!("col") |
					local_name!("embed") |
					local_name!("frame") |
					local_name!("hr") |
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
			);

		self.stack.push(ElemInfo {
			html_name,
			ignore_children,
		});

		Ok(())
	}

	/// Write Closing Tag.
	///
	/// Imported from `html5ever`.
	fn end_elem(&mut self, name: QualName) -> io::Result<()> {
		let info = self.stack.pop().ok_or(io::ErrorKind::UnexpectedEof)?;

		// Childless tags don't need closures.
		if info.ignore_children {
			return Ok(());
		}

		self.writer.write_all(b"</")?;
		self.writer.write_all(name.local.as_bytes())?;
		self.writer.write_all(b">")
	}

	/// Write Text.
	///
	/// Imported from `html5ever`.
	fn write_text(&mut self, txt: &str) -> io::Result<()> {
		match self.parent().html_name {
			Some(
				local_name!("style") |
				local_name!("script") |
				local_name!("xmp") |
				local_name!("iframe") |
				local_name!("noembed") |
				local_name!("noframes") |
				local_name!("noscript") |
				local_name!("plaintext")
			) => self.writer.write_all(txt.as_bytes()),
			_ => self.write_esc_text(txt.as_bytes()),
		}
	}

	/// Write Doctype.
	///
	/// Imported from `html5ever`.
	fn write_doctype(&mut self, name: &str) -> io::Result<()> {
		self.writer.write_all(b"<!DOCTYPE ")?;
		self.writer.write_all(name.as_bytes())?;
		self.writer.write_all(b">")
	}

	/// Write Comments.
	///
	/// Comments were stripped earlier, so this does nothing.
	fn write_comment(&mut self, _txt: &str) -> io::Result<()> { Ok(()) }

	/// Write Processing Instructions.
	///
	/// PIs were stripped earlier, so this does nothing.
	fn write_processing_instruction(&mut self, _target: &str, _data: &str) -> io::Result<()> { Ok(()) }
}
