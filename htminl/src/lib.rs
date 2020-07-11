/*!
# `HTMinL`

In-place minification of HTML file(s).

This approach based on `html5minify`.
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
	chain_filters,
	Element,
	filter::{
		Action,
		detach_comments,
		detach_pis,
		xmp_to_pre,
	},
	html::{
		parse_utf8,
		t,
	},
	NodeData,
	NodeRef,
};
use regex::{
	bytes::Regex as RegexB,
	Captures,
	Regex,
};
use spec::{
	MinifyAttribute,
	MinifyElement,
	MinifyNode,
};
use std::{
	borrow::Cow,
	io,
};



/// Minify HTML.
pub fn minify_html(mut data: &mut Vec<u8>) -> io::Result<usize> {
	// We need something to encode!
	if data.is_empty() {
		return Err(io::ErrorKind::WriteZero.into());
	}

	// Note our starting length.
	let old_len: usize = data.len();

	// Get a document.
	let mut doc = parse_utf8(data);

	// Filter it a bit.
	doc.filter_breadth(chain_filters!(
		detach_comments, // Strip comments.
		detach_pis,      // Strip XML instructions.
		xmp_to_pre,      // Convert weird shit to <pre>.
	));

	// Apply minifications!
	doc.filter(filter_minify);

	// Serialize it back to our source.
	data.truncate(0);
	doc.serialize(data)?;

	post_minify(&mut data);

	// Return the amount saved.
	let new_len: usize = data.len();
	if new_len >= old_len {
		Err(io::ErrorKind::Other.into())
	}
	else { Ok(old_len - new_len) }
}

#[allow(clippy::suspicious_else_formatting)] // Sorry not sorry.
/// Minify Nodes!
pub fn filter_minify(node: NodeRef<'_>, data: &mut NodeData) -> Action {
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
		NodeData::Text(ref mut txt) => {
			// What we do with the text depends on its parent element.
			if let Some(ref parent) = node.parent() {
				if let Some(el) = (*parent).as_element() {
					// The <head> is no place for text nodes!
					if el.is_elem(t::HEAD) || el.is_elem(t::HTML) {
						return Action::Detach;
					}

					// We can trim JS, but should leave everything else as-is
					// as the state of Rust minification is iffy.
					else if el.is_elem(t::SCRIPT) {
						let alter = txt.trim();
						if alter != txt.to_string() {
							if alter.is_empty() {
								return Action::Detach;
							}
							*txt = alter.into();
						}
					}

					// Styles can be trimmed and collapsed reasonably safely.
					else if el.is_elem(t::STYLE) {
						let alter = naive_collapse_whitespace(txt);
						if alter != txt.to_string() {
							if alter.is_empty() {
								return Action::Detach;
							}
							*txt = alter.into();
						}
					}

					// We can drop whitespace nodes between <pre> and <code>,
					// even though we'd be preserving their whitespace in all
					// other circumstances.
					else if
						el.is_elem(t::PRE) &&
						(
							node.next_sibling_is_elem(t::CODE) ||
							node.prev_sibling_is_elem(t::CODE)
						) &&
						txt.trim().is_empty()
					{
						return Action::Detach;
					}

					// Otherwise most everything else can be collapsed.
					else if el.is_minifiable() {
						let mut alter = collapse_whitespace(&decode_entities(txt));

						// If this is the first or last part of the body, we
						// can trim the sides too.
						if el.is_elem(t::BODY) {
							if node.prev_sibling().is_none() {
								alter = alter.trim_start().into();
							}
							if node.next_sibling().is_none() {
								alter = alter.trim_end().into();
							}
						}

						if alter != txt.to_string() {
							if alter.is_empty() {
								return Action::Detach;
							}
							*txt = alter.into();
						}
					}
				}
			}

			Action::Continue
		},
		// Nothing else matters.
		_ => Action::Continue,
	}
}

/// Post Compile Minify
///
/// The serializer leaves a couple other tasks for us to pick up at the end.
pub fn post_minify(data: &mut Vec<u8>) {
	lazy_static::lazy_static! {
		// Drop ="" on boolean attributes.
		static ref RE_BOOL: RegexB = RegexB::new("(allowfullscreen|async|autofocus|autoplay|checked|compact|controls|declare|default|defaultchecked|defaultmuted|defaultselected|defer|disabled|draggable|enabled|formnovalidate|hidden|indeterminate|inert|ismap|itemscope|loop|multiple|muted|nohref|noresize|noshade|novalidate|nowrap|open|pauseonexit|readonly|required|reversed|scoped|seamless|selected|sortable|truespeed|typemustmatch|visible)=\"\"").unwrap();
	}

	// Find the locations so we can trim manually.
	let found: Vec<usize> = RE_BOOL.find_iter(data).map(|x| x.end()).collect();
	for i in found.into_iter().rev() {
		data.drain(i-3..i);
	}
}

#[must_use]
/// Collapse Whitespace
///
/// HTML doesn't distinguish between whitespace, and only considers the first
/// of a sequence, so we can safely collapse sequences down to a single
/// horizontal space.
///
/// Many minification programs take things further and fully trim whitespace
/// from the ends of text nodes, but we can't be sure such operations won't
/// alter the layout, so we *don't* do that.
pub fn collapse_whitespace(txt: &str) -> String {
	lazy_static::lazy_static! {
		// Whitespace collapsing requires two parts in order to preserve non-
		// breaking spaces.
		static ref RE_OUTER: Regex = Regex::new(r"[\s\n\r\t\f\v]+").unwrap();
		static ref RE_INNER: Regex = Regex::new(r"(^|\xA0+)([^\xA0]+)").unwrap();
	}

	RE_OUTER.replace_all(txt, |caps: &Captures| {
		RE_INNER.replace_all(&caps[0], |caps2: &Captures| {
			if caps2[1].is_empty() { Cow::Borrowed(" ") }
			else if caps2[2].is_empty() {
				Cow::Owned(caps2[1].into())
			}
			else {
				Cow::Owned([&caps2[1], " "].concat())
			}
		}).to_string()
	}).into()
}

#[must_use]
/// Naive Whitespace Minification
///
/// This will trim the edges and convert all remaining contiguous sequences to
/// a single space.
pub fn naive_collapse_whitespace(txt: &str) -> String {
	lazy_static::lazy_static! {
		static ref RE_SPACE: Regex = Regex::new(r"[\s\n\r\t\f\v]+").unwrap();
	}

	RE_SPACE.replace_all(txt.trim(), " ").into()
}

#[must_use]
/// Decode Entities
///
/// Since we're working in UTF-8 anyway, we don't really need encoded entities
/// in Text blocks other than < and >.
///
/// If decoding fails, the original value is returned.
pub fn decode_entities(txt: &str) -> Cow<str> {
	match htmlescape::decode_html(txt) {
		Ok(s) => Cow::Owned(s),
		_ => Cow::Borrowed(txt),
	}
}
