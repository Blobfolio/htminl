/*!
# HTML Traits: `Attribute`

This trait exposes a few methods to the `Attribute` struct.
*/

use marked::{
	Attribute,
	LocalName,
};
use super::meta::{a, t};



/// Can Compact?
///
/// Without going overboard, there is some opportunity to safely save
/// a few bytes by trimming and compacting the whitespace in certain
/// types of attributes like classes and styles (which can get long
/// enough while writing that devs might choose to split them into
/// separate lines, etc.).
pub(super) const fn can_compact_value(attr: &Attribute) -> bool {
	matches!(attr.name.local, a::CLASS | a::STYLE)
}

/// Can Drop Attribute?
///
/// Certain attributes, such as `type="text/javascript"` on a `<script>`
/// element are unnecessary (and actively discouraged), so can be safely
/// removed from the document.
pub(super) fn can_drop(attr: &Attribute, tag: &LocalName) -> bool {
	match attr.name.local {
		// Default "type" tags. Technically `<input type="text"/>` is
		// also a default, but because it is frequently matched in CSS
		// rules, we'll leave those be.
		a::TYPE => match *tag {
			t::SCRIPT => matches!(
				attr.value.to_ascii_lowercase().as_str(),
				"text/javascript"
				| "application/javascript"
				| "application/x-javascript"
				| "text/ecmascript"
				| "application/ecmascript"
				| "text/jscript"
			),
			t::STYLE => attr.value.eq_ignore_ascii_case("text/css"),
			_ => attr.value.is_empty(),
		},
		// These tags serve no purpose if they have no values! There are
		// lots of others, but these are the most common, and also the most
		// asinine to leave blank.
		a::ABBR
		| a::ALT
		| a::CLASS
		| a::FOR
		| a::HREF
		| a::ID
		| a::NAME
		| a::PLACEHOLDER
		| a::REL
		| a::SRC
		| a::SRCSET
		| a::STYLE
		| a::TARGET
		| a::TITLE => attr.value.is_empty(),
		// If this is a falsey boolean attribute, we can get rid of it.
		_ => is_boolean(attr) && attr.value.eq_ignore_ascii_case("false"),
	}
}

/// Can Drop Value?
///
/// HTML doesn't require explicit empty values, or values on various
/// "boolean"-like properties such as `autoplay`, `defer`, etc. In such
/// cases the attribute name is all that matters; the value can be safely
/// dropped.
pub(super) fn can_drop_value(attr: &Attribute) -> bool {
	attr.value.is_empty() ||
	(is_boolean(attr) && ! attr.value.eq_ignore_ascii_case("false"))
}

/// Is Boolean Attribute?
///
/// These attributes either are or aren't. Their existence implies "true",
/// so if they're true they don't need values, and if they're false they
/// don't need to be at all.
pub(super) const fn is_boolean(attr: &Attribute) -> bool {
	matches!(
		attr.name.local,
		a::ALLOWFULLSCREEN
		| a::ASYNC
		| a::AUTOFOCUS
		| a::AUTOPLAY
		| a::CHECKED
		| a::COMPACT
		| a::CONTROLS
		| a::DECLARE
		| a::DEFAULT
		| a::DEFER
		| a::DISABLED
		| a::FORMNOVALIDATE
		| a::HIDDEN
		| a::ISMAP
		| a::ITEMSCOPE
		| a::LOOP
		| a::MULTIPLE
		| a::MUTED
		| a::NOHREF
		| a::NOMODULE
		| a::NORESIZE
		| a::NOSHADE
		| a::NOVALIDATE
		| a::NOWRAP
		| a::OPEN
		| a::READONLY
		| a::REQUIRED
		| a::SCOPED
		| a::SEAMLESS
		| a::SELECTED
	)
}
