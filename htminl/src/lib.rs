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


pub mod spec;

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
use regex::{
	bytes::Regex as RegexB,
	Regex,
};
use spec::{
	MinifyAttribute,
	MinifyElement,
	MinifyNode,
};
use std::{
	cell::RefCell,
	io,
	ops::Range,
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
	doc.filter_breadth(filter_minify_one);

	// Second Pass: merge adjacent text nodes.
	doc.filter(filter_minify_two);

	// Third Pass: clean up and minify text nodes.
	doc.filter_breadth(filter_minify_three);

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
pub fn filter_minify_one(_: NodeRef<'_>, data: &mut NodeData) -> Action {
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
					// Truthy values can be empty.
					else {
						attrs[idx].value = "".into();
						idx += 1;
					}
				}
				else { idx += 1 }
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
	match data {
		NodeData::Text(ref mut txt) => {
			// What we do with the text depends on its parent element.
			if let Some(ref parent) = node.parent() {
				if let Some(el) = (*parent).as_element() {
					// The outer <html> and <head> elements are no place for
					// text nodes!
					if el.is_elem(t::HEAD) || el.is_elem(t::HTML) {
						return Action::Detach;
					}

					// We can trim JS, but should leave everything else as-is
					// as the state of Rust minification is iffy.
					else if el.is_elem(t::SCRIPT) {
						txt.trim();
						if txt.is_empty() {
							return Action::Detach;
						}
					}

					// Styles can be trimmed and collapsed reasonably safely.
					else if el.is_elem(t::STYLE) {
						txt.trim();
						txt.collapse_whitespace();
						if txt.is_empty() {
							return Action::Detach;
						}
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
						if txt.is_empty() {
							return Action::Detach;
						}
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

						if txt.is_empty() {
							return Action::Detach;
						}
					}
				}
			}

			Action::Continue
		},
		// Whatever else there is can pass through unchanged.
		_ => Action::Continue,
	}
}

/// Post Compile Minify
///
/// The final minification pass runs in-place after serialization to clean up a
/// few odds and ends.
///
/// It would be more efficient to handle this within the serializer, but that
/// is outside the immediate scope of work. Maybe later…
pub fn post_minify(data: &mut Vec<u8>) {
	lazy_static::lazy_static! {
		// We want to drop empty boolean attribute values, and turn SVG <path>
		// back into a self-closing tag.
		static ref RE: RegexB = RegexB::new("(></path>|(allowfullscreen|async|autofocus|autoplay|checked|compact|controls|declare|default|defaultchecked|defaultmuted|defaultselected|defer|disabled|draggable|enabled|formnovalidate|hidden|indeterminate|inert|ismap|itemscope|loop|multiple|muted|nohref|noresize|noshade|novalidate|nowrap|open|pauseonexit|readonly|required|reversed|scoped|seamless|selected|sortable|truespeed|typemustmatch|visible)=\"\")").unwrap();
	}

	// This glorious chain calculates matching ranges, then drains them from
	// the source in reverse order.
	RE.find_iter(data)
		.flat_map(|x| {
			// By trimming "><" and "path", we're left with "/>", i.e. the
			// self-closing tag we want.
			if x.as_bytes() == b"></path>" {
				vec![x.start()..x.start()+2, x.start()+3..x.end()-1]
			}
			// Here we just need to get rid of the trailing ="".
			else {
				vec![x.end()-3..x.end()]
			}
		})
		.collect::<Vec<Range<usize>>>()
		.into_iter()
		.rev()
		.for_each(|x| { data.drain(x); });
}

#[must_use]
/// Collapse Whitespace
///
/// HTML rendering largely ignores whitespace, and at any rate treats all types
/// (other than the no-break space `\xA0`) the same.
///
/// There is some nuance, but for most elements, we can safely convert all
/// contiguous sequences of whitespace to a single horizontal space character.
///
/// Complete trimming gets dangerous, particularly given that CSS can override
/// the display state of any element arbitrarily, so we are *not* doing that
/// here.
pub fn collapse_whitespace(txt: &str) -> StrTendril {
	lazy_static::lazy_static! {
		static ref RE_SPACE: Regex = Regex::new(r"[\s\n\r\t\f\v]+").unwrap();
	}

	// We don't want to strip no-break spaces, so we temporarily convert them
	// into entities before running Regex, then change them back after (as the
	// serializer expects unencoded UTF-8).
	RE_SPACE.replace_all(&txt.replace("\u{a0}", "&nbsp;"), " ")
		.replace("&nbsp;", "\u{a0}").into()
}

/// Extra String Methods for Tendril.
trait StrTendrilExt {
	/// Collapse Whitespace
	fn collapse_whitespace(&mut self);

	/// Trim.
	fn trim(&mut self);

	/// Trim Start.
	fn trim_start(&mut self);

	/// Trim End.
	fn trim_end(&mut self);
}

impl StrTendrilExt for StrTendril {
	/// Collapse Whitespace
	fn collapse_whitespace(&mut self) {
		let alter = collapse_whitespace(self);
		if (*self).ne(&alter) {
			*self = alter;
		}
	}

	/// Trim.
	fn trim(&mut self) {
		let mut alter = Self::with_capacity(self.len() as u32);
		alter.push_slice((**self).trim());
		if (*self).ne(&alter) {
			*self = alter;
		}
	}

	/// Trim Start.
	fn trim_start(&mut self) {
		let mut alter = Self::with_capacity(self.len() as u32);
		alter.push_slice((**self).trim_start());
		if (*self).ne(&alter) {
			*self = alter;
		}
	}

	/// Trim End.
	fn trim_end(&mut self) {
		let mut alter = Self::with_capacity(self.len() as u32);
		alter.push_slice((**self).trim_end());
		if (*self).ne(&alter) {
			*self = alter;
		}
	}
}
