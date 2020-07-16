/*!
# HTML Library
*/

#![warn(missing_docs)]
#![warn(trivial_casts)]
#![warn(trivial_numeric_casts)]
#![warn(unused_import_braces)]

#![deny(missing_copy_implementations)]
#![deny(missing_debug_implementations)]

#![allow(clippy::unknown_clippy_lints)]

#![warn(clippy::filetype_is_file)]
#![warn(clippy::integer_division)]
#![warn(clippy::needless_borrow)]
#![warn(clippy::nursery)]
#![warn(clippy::pedantic)]
#![warn(clippy::perf)]
#![warn(clippy::suboptimal_flops)]
#![warn(clippy::unneeded_field_pattern)]

#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)]



pub mod meta;
pub mod traits;
mod serialize;



use crate::{
	meta::{a, t},
	serialize::serialize,
	traits::{
		MinifyAttribute,
		MinifyElement,
		MinifyNodeRef,
		MinifyStrTendril,
	},
};
use marked::{
	Element,
	filter::Action,
	html::parse_utf8,
	NodeData,
	NodeRef,
};
use std::{
	cell::RefCell,
	io,
};
use tendril::StrTendril;



/// Minify HTML.
pub fn minify_html(mut data: &mut Vec<u8>) -> io::Result<usize> {
	// We need something to encode!
	if data.is_empty() {
		return Err(io::ErrorKind::WriteZero.into());
	}

	// Note our starting length.
	let old_len: usize = data.len();

	// Parse the document.
	let mut doc = parse_utf8(data);

	// First Pass: clean up elements, strip pointless nodes.
	// Second Pass: merge adjacent text nodes.
	// Third Pass: clean up and minify text nodes.
	doc.filter(filter_minify_one);
	doc.filter(filter_minify_two);
	doc.filter(filter_minify_three);

	// Save it!
	data.truncate(0);
	serialize(&mut data, &doc.document_node_ref())?;

	// Return the amount saved.
	let new_len: usize = data.len();
	if new_len >= old_len {
		Err(io::ErrorKind::Other.into())
	}
	else { Ok(old_len - new_len) }
}

/// Minify #1
///
/// This strips comments and XML processing instructions, removes default type
/// attributes for scripts and styles, and removes truthy attribute values for
/// boolean properties like "defer" and "disabled".
pub fn filter_minify_one(node: NodeRef<'_>, data: &mut NodeData) -> Action {
	match data {
		NodeData::Elem(Element { name, attrs, .. }) => {
			let mut len: usize = attrs.len();
			let mut idx: usize = 0;

			while idx < len {
				// Almost always trim...
				if attrs[idx].name.local != a::VALUE {
					attrs[idx].value.trim();
				}

				// Drop the whole thing.
				if attrs[idx].can_drop(&name.local) {
					attrs.remove(idx);
					len -= 1;
					continue;
				}

				// Drop the value.
				if attrs[idx].can_drop_value() {
					attrs[idx].value = StrTendril::new();
				}
				// Compact the value.
				else if attrs[idx].can_compact_value() {
					attrs[idx].value.collapse_whitespace();
				}

				idx += 1;
			}

			Action::Continue
		},
		NodeData::Text(_) => {
			// We never need text nodes in the `<head>` or `<html>`.
			if let Some(el) = node.parent().as_deref().and_then(|p| p.as_element()) {
				if el.can_drop_text_nodes() {
					return Action::Detach;
				}
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
pub fn filter_minify_two(pos: NodeRef<'_>, data: &mut NodeData) -> Action {
    thread_local! {
        static MERGE_Q: RefCell<StrTendril> = RefCell::new(StrTendril::new())
    };

    if let Some(t) = data.as_text_mut() {
        // If the immediately following sibling is also text, then push this
        // tendril to the merge queue and detach.
        let node_r = pos.next_sibling();
        if node_r.map_or(false, |n| n.as_text().is_some()) {
            MERGE_Q.with(|q| {
                q.borrow_mut().push_tendril(t)
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
pub fn filter_minify_three(node: NodeRef<'_>, data: &mut NodeData) -> Action {
	if let Some(txt) = data.as_text_mut() {
		if let Some(el) = node.parent().as_deref().and_then(|p| p.as_element()) {
			// Special cases.
			if txt.is_whitespace() && node.can_drop_if_whitespace() {
				return Action::Detach;
			}

			// Can we trim the text?
			if el.can_trim_whitespace() {
				txt.trim();
			}

			// How about collapse it?
			if el.can_collapse_whitespace() {
				txt.collapse_whitespace();

				// If the body starts or ends with a text node, we can trim it
				// from the left or the right respectively.
				if el.is_elem(t::BODY) {
					// Drop the start.
					if txt.starts_with(' ') && node.is_first_child() {
						txt.pop_front(1);
					}
					// Drop the end.
					if txt.ends_with(' ') && node.is_last_child() {
						txt.pop_back(1);
					}
				}
			}

			// Drop empty nodes entirely.
			if txt.is_empty() {
				return Action::Detach;
			}
		}
	}

	Action::Continue
}
