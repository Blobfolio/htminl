/*!
# `HTMinL` - Helpers
*/

pub(self) mod attribute;
pub(self) mod element;
pub(super) mod error;
mod meta;
pub(self) mod noderef;
pub(self) mod serialize;
pub(self) mod strtendril;



use error::HtminlError;
use marked::{
	Element,
	filter::Action,
	html::parse_utf8,
	NodeData,
	NodeRef,
};
use meta::{a, t};
use once_cell::sync::Lazy;
use serialize::serialize;
use std::{
	borrow::BorrowMut,
	cell::RefCell,
	convert::TryFrom,
	num::NonZeroUsize,
	path::PathBuf,
};
use tendril::StrTendril;



#[derive(Debug)]
/// # `HTMinL`
pub(super) struct Htminl<'a> {
	src: &'a PathBuf,
	buf: Vec<u8>,
	size: NonZeroUsize,
}

impl<'a> TryFrom<&'a PathBuf> for Htminl<'a> {
	type Error = HtminlError;

	fn try_from(src: &'a PathBuf) -> Result<Self, Self::Error> {
		let buf: Vec<u8> = std::fs::read(src).map_err(|_| HtminlError::Read)?;
		let size = NonZeroUsize::new(buf.len()).ok_or(HtminlError::EmptyFile)?;

		Ok(Self { src, buf, size })
	}
}

impl Htminl<'_> {
	/// # Minify!
	pub(super) fn minify(&mut self) -> Result<(), HtminlError> {
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

		let new_len: usize = self.buf.len();
		if 0 < new_len && new_len < self.size.get() {
			// Save it!
			write_atomic::write_file(self.src, &self.buf)
				.map_err(|_| HtminlError::Write)?;
		}

		Ok(())
	}

	/// # Is Fragment.
	fn is_fragment(&self) -> bool {
		use regex::bytes::Regex;

		static RE_HAS_HTML: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?i)(<html|<body|</body>|</html>)").unwrap());

		! RE_HAS_HTML.is_match(&self.buf)
	}

	/// # Make (Fragment) Whole.
	fn make_whole(&mut self) {
		// The combined length of the opener and closer.
		const ADJ: usize = 25 + 14;

		// Reserve space for the opener and closer.
		self.buf.reserve(ADJ);

		unsafe {
			// Copy all content 25 (opener) to the right.
			std::ptr::copy(
				self.buf.as_ptr(),
				self.buf.as_mut_ptr().add(25),
				self.size.get()
			);

			// Copy opener.
			std::ptr::copy_nonoverlapping(
				b"<html><head></head><body>".as_ptr(),
				self.buf.as_mut_ptr(),
				25
			);

			// Copy closer.
			std::ptr::copy_nonoverlapping(
				b"</body></html>".as_ptr(),
				self.buf.as_mut_ptr().add(self.size.get() + 25),
				14
			);

			// Adjust the buffer length.
			self.buf.set_len(self.size.get() + ADJ);
		}
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
            MERGE_Q.with(|q| {
                q.borrow_mut().push_tendril(t);
            });
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

#[allow(clippy::suspicious_else_formatting)] // Sorry not sorry.
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
