/*!
# HTML Traits: `Element`

This trait exposes a few methods to the `Element` struct.
*/

use marked::{
	Element,
	html::{
		t,
		TAG_META,
	},
};



/// Minification-related Element Methods.
pub trait MinifyElement {
	/// Can Collapse Whitespace?
	///
	/// Text nodes in these elements can safely have their whitespace
	/// collapsed.
	fn can_collapse_whitespace(&self) -> bool;

	/// Can Drop Text Nodes?
	///
	/// Text nodes in these elements are never needed.
	fn can_drop_text_nodes(&self) -> bool;

	/// Can Drop Whitespace Between?
	///
	/// Whitespace-only text nodes sitting between two elements of this kind
	/// (or at the beginning and end of the parent) can be safely dropped.
	fn can_drop_whitespace_sandwhich(&self) -> bool;

	/// Can Trim Whitespace?
	///
	/// Text nodes in these elements can safely have their whitespace
	/// trimmed.
	fn can_trim_whitespace(&self) -> bool;
}

impl MinifyElement for Element {
	#[must_use]
	/// Can Collapse Whitespace?
	///
	/// Text nodes in these elements can safely have their whitespace
	/// collapsed.
	///
	/// At the moment, this applies to all "known" tags other than `<code>`,
	/// `<pre>`, `<script>`, `<svg>`, and `<textarea>`.
	fn can_collapse_whitespace(&self) -> bool {
		match self.name.local {
			t::CODE
			| t::PRE
			| t::SCRIPT
			| t::SVG
			| t::TEXTAREA => false,
			ref x => TAG_META.contains_key(x),
		}
	}

	/// Can Drop Text Nodes?
	///
	/// Text nodes in these elements are never needed.
	fn can_drop_text_nodes(&self) -> bool {
		match self.name.local {
			t::AUDIO
			| t::HEAD
			| t::HTML
			| t::OPTION
			| t::PICTURE
			| t::VIDEO => true,
			_ => false,
		}
	}

	/// Can Drop Whitespace Between?
	///
	/// Whitespace-only text nodes sitting between two elements of this kind
	/// (or at the beginning and end of the parent) can be safely dropped.
	fn can_drop_whitespace_sandwhich(&self) -> bool {
		match self.name.local {
			t::NOSCRIPT
			| t::SCRIPT
			| t::STYLE => true,
			_ => false,
		}
	}

	/// Can Trim Whitespace?
	///
	/// Text nodes in these elements can safely have their whitespace
	/// trimmed.
	///
	/// At the moment, this only applies to `<script>`, `<noscript>`,
	/// `<style>`, and `<transition>` tags.
	fn can_trim_whitespace(&self) -> bool {
		match self.name.local {
			t::NOSCRIPT | t::SCRIPT | t::STYLE => true,
			_ => &*self.name.local == "transition",
		}
	}
}
