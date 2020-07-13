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



pub mod traits;



use marked::{
	Element,
	filter::Action,
	html::{
		parse_utf8,
		t,
	},
	NodeData,
	NodeRef,
};
use std::{
	cell::RefCell,
	io,
};
use tendril::StrTendril;
use traits::{
	MinifyAttribute,
	MinifyElement,
	MinifyNodeRef,
	MinifyStrTendril,
};



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
	doc.serialize(data)?;

	// Final Pass: tidy up after the serializer.
	post_minify(&mut data);

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
				// We can drop default type attributes altogether.
				if attrs[idx].is_default_type(&name.local) {
					attrs.remove(idx);
					len -= 1;
				}
				// Boolean values can be simplified or removed.
				else if attrs[idx].is_boolean_value() {
					// False is implied; we can just remove it.
					if attrs[idx].value.to_ascii_lowercase() == "false" {
						attrs.remove(idx);
						len -= 1;
					}
					// Truthy values can be empty. The placeholder "*hNUL" is
					// something we'll clear out in post.
					else {
						attrs[idx].value = "*hNUL".into();
						idx += 1;
					}
				}
				// We'll strip these in post so they don't print as ="".
				else if attrs[idx].value.is_empty() {
					attrs[idx].value = "*hNUL".into();
					idx += 1;
				}
				else { idx += 1 }
			}

			Action::Continue
		},
		NodeData::Text(_) => {
			// We never need text nodes in the `<head>` or `<html>`.
			if let Some(el) = node.parent().as_deref().and_then(|p| p.as_element()) {
				if el.is_elem(t::HEAD) || el.is_elem(t::HTML) {
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
			// We can trim JS, but should leave everything else as-is
			// as the state of Rust minification is iffy.
			if el.is_elem(t::SCRIPT) {
				txt.trim();
			}

			// Styles can be trimmed and collapsed reasonably safely.
			else if el.is_elem(t::STYLE) {
				txt.trim();
				txt.collapse_whitespace();
			}

			// We don't need empty/whitespace text nodes between <pre>
			// and <code> tags.
			else if el.is_elem(t::PRE) {
				if
					(**txt).trim().is_empty() &&
					(
						node.next_sibling_is_elem(t::CODE) ||
						node.prev_sibling_is_elem(t::CODE)
					)
				{
					return Action::Detach;
				}
			}

			// Vue `transition` tags can be full-on trimmed.
			else if &*el.name.local == "transition" {
				txt.trim();
			}

			// Otherwise most everything else can be collapsed.
			else if el.is_minifiable() {
				txt.collapse_whitespace();

				// First and last body text can be trimmed.
				if el.is_elem(t::BODY) {
					// Drop the start.
					if node.prev_sibling().is_none() {
						txt.trim_start();
					}
					// Drop the end.
					if node.next_sibling().is_none() {
						txt.trim_end();
					}
				}
			}

			if txt.is_empty() {
				return Action::Detach;
			}
		}
	}

	Action::Continue
}

/// Post Compile Minify
///
/// The final minification pass runs in-place after serialization to clean up a
/// few odds and ends.
///
/// It would be more efficient to handle this within the serializer, but that
/// is outside the immediate scope of work. Maybe later…
pub fn post_minify(data: &mut Vec<u8>) {
	let len: usize = data.len();
	let ptr = data.as_mut_ptr();
	let mut del: usize = 0;

	unsafe {
		let mut idx: usize = 0;
		while idx < len {
			// We've made life easy on ourselves by using needles of the same
			// length (8).
			if idx + 8 < len {
				// It's a path closure!
				if &data[idx..idx+8] == b"></path>" {
					// A quick shuffle places "/>" at the beginning of the
					// slice (so we can just drop the rest).
					ptr.add(idx).swap(ptr.add(idx + 2));
					ptr.add(idx + 1).swap(ptr.add(idx + 2));

					// If we've deleted stuff, we need to shift them both into
					// place.
					if del > 0 {
						ptr.add(idx).swap(ptr.add(idx - del));
						ptr.add(idx + 1).swap(ptr.add(idx + 1 - del));
					}

					// Increase the del and idx markers accordingly.
					del += 6;
					idx += 8;
					continue;
				}

				// It's a null attribute value!
				else if &data[idx..idx+8] == b"=\"*hNUL\"" {
					del += 8;
					idx += 8;
					continue;
				}
			}

			if del > 0 {
				ptr.add(idx).swap(ptr.add(idx - del));
			}

			idx += 1;
		}
	}

	// If we "removed" anything, truncate to drop the extra bits from memory.
	if del > 0 {
		data.truncate(len - del);
	}
}
