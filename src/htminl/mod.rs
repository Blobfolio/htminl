/*!
# `HTMinL` - Helpers
*/

mod attribute;
mod element;
mod meta;
mod noderef;
mod serialize;
mod strtendril;



use crate::HtminlError;
use marked::{
	Element,
	filter::Action,
	html::parse_utf8,
	NodeData,
	NodeRef,
};
use meta::{a, t};
use serialize::serialize;
use std::{
	borrow::BorrowMut,
	cell::RefCell,
	path::PathBuf,
};
use tendril::StrTendril;



#[derive(Debug)]
/// # `HTMinL`
pub(super) struct Htminl<'a> {
	/// # Source Path.
	src: &'a PathBuf,

	/// # Raw Data.
	buf: Vec<u8>,

	/// # File Size.
	pub(super) size: u64,
}

impl<'a> TryFrom<&'a PathBuf> for Htminl<'a> {
	type Error = HtminlError;

	fn try_from(src: &'a PathBuf) -> Result<Self, Self::Error> {
		let buf: Vec<u8> = std::fs::read(src).map_err(|_| HtminlError::Read)?;
		let size = u64::try_from(buf.len()).map_err(|_| HtminlError::Read)?;
		if size == 0 {
			return Err(HtminlError::EmptyFile);
		}

		Ok(Self { src, buf, size })
	}
}

impl Htminl<'_> {
	/// # Minify!
	pub(super) fn minify(&mut self) -> Result<(u64, u64), HtminlError> {
		// Determine whether we're working on a whole document or fragment.
		let fragment: bool = self.is_fragment();

		// Add padding for fragments.
		if fragment { self.make_whole(); }

		// Parse the document.
		let mut doc = parse_utf8(&self.buf);

		// First Pass: clean up elements, strip pointless nodes.
		// Second Pass: merge adjacent text nodes.
		// Third Pass: clean up and minify text nodes.
		doc.filter(filter_minify_one);
		doc.filter(filter_minify_two);
		doc.filter(filter_minify_three);

		// Convert it back to an HTML string (er, byte slice)!
		self.buf.truncate(0);
		serialize(&mut self.buf, &doc.document_node_ref())?;

		// Remove fragment padding.
		if fragment { self.make_fragment()?; }

		// Save it!
		let new_len: u64 = self.buf.len() as u64;
		if
			0 < new_len &&
			new_len < self.size &&
			write_atomic::write_file(self.src, &self.buf).is_ok()
		{
			Ok((self.size, new_len))
		}
		// No change.
		else {
			Ok((self.size, self.size))
		}
	}

	/// # Is Fragment.
	///
	/// This returns `false` if the document contains (case-insensitively)
	/// `<html`, `<body`, `</body>`, or `</html>`.
	fn is_fragment(&self) -> bool {
		for w in self.buf.windows(7) {
			if w[0] == b'<' {
				match w[1] {
					b'/' => if w[6] == b'>' {
						let mid = &w[2..6];
						if mid.eq_ignore_ascii_case(b"body") || mid.eq_ignore_ascii_case(b"html") {
							return false;
						}
					},
					b'b' | b'B' => if w[2..5].eq_ignore_ascii_case(b"ody") { return false; },
					b'h' | b'H' => if w[2..5].eq_ignore_ascii_case(b"tml") { return false; },
					_ => {},
				}
			}
		}

		true
	}

	/// # Make (Fragment) Whole.
	fn make_whole(&mut self) {
		let mut new: Vec<u8> = Vec::with_capacity(25 + 14 + self.buf.len());
		new.extend_from_slice(b"<html><head></head><body>");
		new.extend_from_slice(&self.buf);
		new.extend_from_slice(b"</body></html>");

		std::mem::swap(&mut self.buf, &mut new);
	}

	/// # Back to Fragment.
	fn make_fragment(&mut self) -> Result<(), HtminlError> {
		// Chop off the fragment scaffold, if present.
		if
			self.buf.starts_with(b"<html><head></head><body>") &&
			self.buf.ends_with(b"</body></html>")
		{
			self.buf.drain(0..25);
			self.buf.truncate(self.buf.len() - 14);
			Ok(())
		}
		else {
			Err(HtminlError::Parse)
		}
	}
}

/// Minify #1
///
/// This strips comments and XML processing instructions, removes default type
/// attributes for scripts and styles, and removes truthy attribute values for
/// boolean properties like "defer" and "disabled".
fn filter_minify_one(node: NodeRef<'_>, data: &mut NodeData) -> Action {
	match data {
		NodeData::Elem(Element { name, attrs, .. }) => {
			let mut len: usize = attrs.len();
			let mut idx: usize = 0;

			while idx < len {
				// Almost always trim...
				if attrs[idx].name.local != a::VALUE {
					strtendril::trim(&mut attrs[idx].value);
				}

				// Drop the whole thing.
				if attribute::can_drop(&attrs[idx], &name.local) {
					attrs.remove(idx);
					len -= 1;
					continue;
				}

				// Drop the value.
				if attribute::can_drop_value(&attrs[idx]) {
					attrs[idx].value = StrTendril::new();
				}
				// Compact the value.
				else if attribute::can_compact_value(&attrs[idx]) {
					strtendril::collapse_whitespace(&mut attrs[idx].value);
				}

				idx += 1;
			}

			Action::Continue
		},
		NodeData::Text(_) => {
			// We never need text nodes in the `<head>` or `<html>`.
			if node.parent()
				.as_deref()
				.and_then(|p| p.as_element())
				.filter(|el| element::can_drop_text_nodes(el))
				.is_some()
			{
				return Action::Detach;
			}

			Action::Continue
		},
		// Remove comments and XML processing instructions.
		NodeData::Comment(_) | NodeData::Pi(_) => Action::Detach,
		// Whatever else there is can pass through unchanged.
		_ => Action::Continue,
	}
}

/// Minify #2
///
/// This pass merges adjacent text nodes. The code is identical to what
/// `marked` exports under `text_normalize`, except it does not mess about with
/// whitespace. (We do that later, with greater intention.)
fn filter_minify_two(pos: NodeRef<'_>, data: &mut NodeData) -> Action {
    thread_local! {
        static MERGE_Q: RefCell<StrTendril> = RefCell::new(StrTendril::new())
    };

    if let Some(t) = data.as_text_mut() {
        // If the immediately following sibling is also text, then push this
        // tendril to the merge queue and detach.
        let node_r = pos.next_sibling();
        if node_r.map_or(false, |n| n.as_text().is_some()) {
            MERGE_Q.with_borrow_mut(|q| { q.push_tendril(t); });
            return Action::Detach;
        }

        // Otherwise add this tendril to anything in the queue, consuming it.
        MERGE_Q.with(|q| {
            let mut qt = q.borrow_mut();
            if qt.len() > 0 {
                qt.push_tendril(t);
                drop(qt);
                *t = q.replace(StrTendril::new());
            }
        });

        if t.is_empty() {
            return Action::Detach;
        }
    }

    Action::Continue
}

/// Minify #3
///
/// This pass cleans up text nodes, collapsing whitespace (when it is safe to
/// do so), and trimming a bit (also only when safe).
///
/// See `collapse_whitespace` for more details.
fn filter_minify_three(node: NodeRef<'_>, data: &mut NodeData) -> Action {
	if let Some((txt, el)) = data.as_text_mut()
		.zip(node.parent().as_deref().and_then(|p| p.as_element()))
	{
		// Special cases.
		if strtendril::is_whitespace(txt) && noderef::can_drop_if_whitespace(&node) {
			return Action::Detach;
		}

		// Can we trim the text?
		if element::can_trim_whitespace(el) {
			strtendril::trim(txt.borrow_mut());
		}

		// How about collapse it?
		if element::can_collapse_whitespace(el) {
			strtendril::collapse_whitespace(txt.borrow_mut());

			// If the body starts or ends with a text node, we can trim it
			// from the left or the right respectively.
			if el.is_elem(t::BODY) {
				// Drop the start.
				if txt.starts_with(' ') && noderef::is_first_child(&node) {
					txt.pop_front(1);
				}
				// Drop the end.
				if txt.ends_with(' ') && noderef::is_last_child(&node) {
					txt.pop_back(1);
				}
			}
		}

		// Drop empty nodes entirely.
		if txt.is_empty() {
			return Action::Detach;
		}
	}

	Action::Continue
}



#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn t_fragments() {
		let path = PathBuf::from("foo.html");
		let mut h = Htminl {
			src: &path,
			buf: b"<html><body>Hi!</body></html>".to_vec(),
			size: 0,
		};
		assert!(!h.is_fragment());

		h.buf.truncate(0);
		h.buf.extend_from_slice(b"<div>Hello world!</div></body>");
		assert!(!h.is_fragment());

		h.buf.truncate(0);
		h.buf.extend_from_slice(b"<div>Hello world!</div></HTML>");
		assert!(!h.is_fragment());

		h.buf.truncate(0);
		h.buf.extend_from_slice(b"<body class='foo'>Hello world!</div>");
		assert!(!h.is_fragment());

		h.buf.truncate(0);
		h.buf.extend_from_slice(b"<div>Hello world!</div>");
		assert!(h.is_fragment());
		h.make_whole();
		assert_eq!(h.buf, b"<html><head></head><body><div>Hello world!</div></body></html>");
		assert!(!h.is_fragment());

		assert!(h.make_fragment().is_ok());
		assert_eq!(h.buf, b"<div>Hello world!</div>");
		assert!(h.is_fragment());
	}
}
