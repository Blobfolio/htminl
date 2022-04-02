/*!
# HTML Traits: `Element`

This trait exposes a few methods to the `Element` struct.
*/

use marked::{
	Element,
	html::TAG_META,
};
use super::meta::t;



#[must_use]
/// Can Collapse Whitespace?
///
/// Text nodes in these elements can safely have their whitespace
/// collapsed.
///
/// At the moment, this applies to all "known" tags other than `<code>`,
/// `<pre>`, `<script>`, `<svg>`, and `<textarea>`.
pub(super) fn can_collapse_whitespace(el: &Element) -> bool {
	match el.name.local {
		t::CODE
		| t::PLAINTEXT
		| t::PRE
		| t::SCRIPT
		| t::SVG
		| t::TEXTAREA => false,
		ref x => TAG_META.contains_key(x),
	}
}

#[must_use]
/// Can Drop Text Nodes?
///
/// Text nodes in these elements are never needed.
pub(super) const fn can_drop_text_nodes(el: &Element) -> bool {
	matches!(
		el.name.local,
		t::AUDIO
		| t::HEAD
		| t::HTML
		| t::OPTION
		| t::PICTURE
		| t::VIDEO
	)
}

#[must_use]
/// Can Drop Whitespace Between?
///
/// Whitespace-only text nodes sitting between two elements of this kind
/// (or at the beginning and end of the parent) can be safely dropped.
pub(super) const fn can_drop_whitespace_sandwhich(el: &Element) -> bool {
	matches!(
		el.name.local,
		t::NOSCRIPT | t::SCRIPT | t::STYLE
	)
}

#[must_use]
/// Can Trim Whitespace?
///
/// Text nodes in these elements can safely have their whitespace
/// trimmed.
///
/// At the moment, this only applies to `<script>`, `<noscript>`,
/// `<style>`, `<title>`, and `<transition>` tags.
pub(super) fn can_trim_whitespace(el: &Element) -> bool {
	match el.name.local {
		t::NOSCRIPT | t::SCRIPT | t::STYLE | t::TITLE => true,
		_ => (&*el.name.local).eq_ignore_ascii_case("transition"),
	}
}
