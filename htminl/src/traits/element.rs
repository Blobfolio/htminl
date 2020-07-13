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
	/// Is Minifiable
	///
	/// Can inner whitespace be collapsed? Most of the time the answer is yes,
	/// but there are a few cases where it is safer to leave things be.
	fn is_minifiable(&self) -> bool;
}

impl MinifyElement for Element {
	#[must_use]
	/// Is Minifiable
	///
	/// Can inner whitespace be collapsed? Most of the time the answer is yes,
	/// but there are a few cases where it is safer to leave things be.
	fn is_minifiable(&self) -> bool {
		match self.name.local {
			t::CODE
			| t::PRE
			| t::SCRIPT
			| t::STYLE
			| t::SVG
			| t::TEXTAREA => false,
			ref x => TAG_META.contains_key(x),
		}
	}
}
