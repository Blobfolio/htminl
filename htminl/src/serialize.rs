/*!
# HTML Library: Minification Serializer

This serializer is almost identical to the one used by `html5ever`, but
includes a few space-saving optimizations.
*/

use html5ever::{
	local_name,
	namespace_url,
	ns,
	serialize::{
		AttrRef,
		Serialize,
		Serializer,
		TraversalScope,
	},
};
use log::warn;
use marked::{
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
pub fn serialize<Wr, T>(writer: Wr, node: &T) -> io::Result<()>
where
	Wr: Write,
	T: Serialize,
{
	let mut ser = MinifySerializer::new(writer);
	node.serialize(&mut ser, TraversalScope::ChildrenOnly(None))
}



#[derive(Default)]
/// Element Info.
///
/// Imported from `html5ever`.
struct ElemInfo {
	html_name: Option<LocalName>,
	ignore_children: bool,
	processed_first_child: bool,
}



/// Minification Serializer
///
/// This is based on `html5ever`'s `HtmlSerializer` and works largely the same
/// way, except some byte-saving routines are employed to reduce the output
/// size.
struct MinifySerializer<Wr: Write> {
	pub writer: Wr,
	stack: Vec<ElemInfo>,
}

/// Retrieve Tag Name.
///
/// Imported from `html5ever`.
fn tagname(name: &QualName) -> LocalName {
	match name.ns {
		ns!(html) | ns!(mathml) | ns!(svg) => (),
		ref ns => {
			// FIXME(#122)
			warn!("node with weird namespace {:?}", ns);
		},
	}

	name.local.clone()
}

#[allow(clippy::default_trait_access)]
impl<Wr: Write> MinifySerializer<Wr> {
	/// New Instance.
	///
	/// Imported from `html5ever`.
	pub fn new(writer: Wr) -> Self {
		Self {
			writer,
			stack: vec![ElemInfo {
				html_name: None,
				ignore_children: false,
				processed_first_child: false,
			}],
		}
	}

	/// Parent.
	///
	/// Imported from `html5ever`.
	fn parent(&mut self) -> &mut ElemInfo {
		if self.stack.is_empty() {
			panic!("no parent ElemInfo")
		}
		self.stack.last_mut().unwrap()
	}

	/// Write Escaped.
	///
	/// Imported from `html5ever`.
	fn write_escaped(&mut self, text: &str, attr_mode: bool) -> io::Result<()> {
		for c in text.chars() {
			match c {
				'&' => self.writer.write_all(b"&amp;"),
				'\u{00A0}' => self.writer.write_all(b"&nbsp;"),
				'"' if attr_mode => self.writer.write_all(b"&quot;"),
				'<' if !attr_mode => self.writer.write_all(b"&lt;"),
				'>' if !attr_mode => self.writer.write_all(b"&gt;"),
				c => self.writer.write_fmt(format_args!("{}", c)),
			}?;
		}
		Ok(())
	}
}

#[allow(clippy::default_trait_access)]
impl<Wr: Write> Serializer for MinifySerializer<Wr> {
	/// Write Opening Tag.
	///
	/// This deviates from `html5ever`:
	/// * SVG `<path>` elements are self-closed;
	/// * Empty attribute values are omitted;
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
				processed_first_child: false,
			});
			return Ok(());
		}

		self.writer.write_all(b"<")?;
		self.writer.write_all(tagname(&name).as_bytes())?;
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
				ref ns => {
					// FIXME(#122)
					warn!("attr with weird namespace {:?}", ns);
					self.writer.write_all(b"unknown_namespace:")?;
				},
			}

			self.writer.write_all(name.local.as_bytes())?;

			// Only write values if they exist.
			if ! value.is_empty() {
				// TODO: quotes may be omitted if the value does not contain:
				// & " ' ` = < > [any kind of whitespace]

				// Will need to track the final attr quote style for path
				// closures to ensure the /> doesn't get treated as part of the
				// value.

				// TODO: attributes without ' but with " could benefit from
				// being enclosed in ='' instead. Will need a different escape
				// function in such cases.

				self.writer.write_all(b"=\"")?;
				self.write_escaped(value, true)?;
				self.writer.write_all(b"\"")?;
			}
		}

		// SVG <path> tags should be self-closing in XML-style.
		let is_svg_path: bool = name.local == local_name!("path");
		if is_svg_path {
			self.writer.write_all(b"/>")?;
		}
		else {
			self.writer.write_all(b">")?;
		}

		// Ignore children?
		let ignore_children =
			is_svg_path ||
			(
				name.ns == ns!(html) &&
				match name.local {
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
					local_name!("wbr") => true,
					_ => false,
				}
			);

		self.parent().processed_first_child = true;

		self.stack.push(ElemInfo {
			html_name,
			ignore_children,
			processed_first_child: false,
		});

		Ok(())
	}

	/// Write Closing Tag.
	///
	/// Imported from `html5ever`.
	fn end_elem(&mut self, name: QualName) -> io::Result<()> {
		let info = match self.stack.pop() {
			Some(info) => info,
			_ => panic!("no ElemInfo"),
		};

		// Childless tags don't need closures.
		if info.ignore_children {
			return Ok(());
		}

		self.writer.write_all(b"</")?;
		self.writer.write_all(tagname(&name).as_bytes())?;
		self.writer.write_all(b">")
	}

	/// Write Text.
	///
	/// Imported from `html5ever`.
	fn write_text(&mut self, text: &str) -> io::Result<()> {
		let escape = match self.parent().html_name {
			Some(local_name!("style")) |
			Some(local_name!("script")) |
			Some(local_name!("xmp")) |
			Some(local_name!("iframe")) |
			Some(local_name!("noembed")) |
			Some(local_name!("noframes")) |
			Some(local_name!("noscript")) |
			Some(local_name!("plaintext")) => false,
			_ => true,
		};

		if escape {
			self.write_escaped(text, false)
		} else {
			self.writer.write_all(text.as_bytes())
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
	fn write_comment(&mut self, _text: &str) -> io::Result<()> {
		Ok(())
	}

	/// Write Processing Instructions.
	///
	/// PIs were stripped earlier, so this does nothing.
	fn write_processing_instruction(&mut self, _target: &str, _data: &str) -> io::Result<()> {
		Ok(())
	}
}
