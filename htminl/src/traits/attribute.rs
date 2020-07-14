/*!
# HTML Traits: `Attribute`

This trait exposes a few methods to the `Attribute` struct.
*/

use marked::{
	Attribute,
	html::t,
	LocalName,
};



/// Minification-related Attribute Methods.
pub trait MinifyAttribute {
	/// Can Compact?
	///
	/// Without going overboard, there is some opportunity to safely save
	/// a few bytes by trimming and compacting the whitespace in certain
	/// types of attributes like classes and styles (which can get long
	/// enough while writing that devs might choose to split them into
	/// separate lines, etc.).
	fn can_compact_value(&self) -> bool;

	/// Can Drop Attribute?
	///
	/// Certain attributes, such as `type="text/javascript"` on a `<script>`
	/// element are unnecessary (and actively discouraged), so can be safely
	/// removed from the document.
	fn can_drop(&self, tag: &LocalName) -> bool;

	/// Can Drop Value?
	///
	/// HTML doesn't require explicit empty values, or values on various
	/// "boolean"-like properties such as `autoplay`, `defer`, etc. In such
	/// cases the attribute name is all that matters; the value can be safely
	/// dropped.
	fn can_drop_value(&self) -> bool;

	/// Is Boolean Attribute?
	///
	/// These attributes either are or aren't. Their existence implies "true",
	/// so if they're true they don't need values, and if they're false they
	/// don't need to be at all.
	fn is_boolean(&self) -> bool;
}

impl MinifyAttribute for Attribute {
	/// Can Compact?
	///
	/// Without going overboard, there is some opportunity to safely save
	/// a few bytes by trimming and compacting the whitespace in certain
	/// types of attributes like classes and styles (which can get long
	/// enough while writing that devs might choose to split them into
	/// separate lines, etc.).
	fn can_compact_value(&self) -> bool {
		match &*self.name.local {
			"class" | "style" => true,
			_ => false,
		}
	}

	/// Can Drop Attribute?
	///
	/// Certain attributes, such as `type="text/javascript"` on a `<script>`
	/// element are unnecessary (and actively discouraged), so can be safely
	/// removed from the document.
	fn can_drop(&self, tag: &LocalName) -> bool {
		match &*self.name.local {
			// Default "type" tags. Technically `<input type="text"/>` is
			// also a default, but because it is frequently matched in CSS
			// rules, we'll leave those be.
			"type" => match *tag {
				t::SCRIPT => match self.value.to_ascii_lowercase().as_str() {
					"text/javascript"
					| "application/javascript"
					| "application/x-javascript"
					| "text/ecmascript"
					| "application/ecmascript"
					| "text/jscript" => true,
					_ => false,
				},
				t::STYLE => self.value.eq_ignore_ascii_case("text/css"),
				_ => self.value.is_empty(),
			},
			// These tags serve no purpose if they have no values! There are
			// lots of others, but these are the most common, and also the most
			// asinine to leave blank.
			"abbr"
			| "alt"
			| "class"
			| "for"
			| "href"
			| "id"
			| "name"
			| "placeholder"
			| "rel"
			| "src"
			| "srcset"
			| "style"
			| "target"
			| "title" => self.value.is_empty(),
			// If this is a falsey boolean attribute, we can get rid of it.
			_ => self.is_boolean() && self.value.eq_ignore_ascii_case("false"),
		}
	}

	/// Can Drop Value?
	///
	/// HTML doesn't require explicit empty values, or values on various
	/// "boolean"-like properties such as `autoplay`, `defer`, etc. In such
	/// cases the attribute name is all that matters; the value can be safely
	/// dropped.
	fn can_drop_value(&self) -> bool {
		self.value.is_empty() ||
		(self.is_boolean() && ! self.value.eq_ignore_ascii_case("false"))
	}

	/// Is Boolean Attribute?
	///
	/// These attributes either are or aren't. Their existence implies "true",
	/// so if they're true they don't need values, and if they're false they
	/// don't need to be at all.
	fn is_boolean(&self) -> bool {
		match &*self.name.local {
			"allowfullscreen"
			| "allowpaymentrequest"
			| "async"
			| "autofocus"
			| "autoplay"
			| "checked"
			| "compact"
			| "controls"
			| "declare"
			| "default"
			| "defaultchecked"
			| "defaultmuted"
			| "defaultselected"
			| "defer"
			| "disabled"
			| "enabled"
			| "formnovalidate"
			| "hidden"
			| "indeterminate"
			| "inert"
			| "ismap"
			| "itemscope"
			| "loop"
			| "multiple"
			| "muted"
			| "nohref"
			| "nomodule"
			| "noresize"
			| "noshade"
			| "novalidate"
			| "nowrap"
			| "open"
			| "pauseonexit"
			| "playsinline"
			| "readonly"
			| "required"
			| "reversed"
			| "scoped"
			| "seamless"
			| "selected"
			| "sortable"
			| "truespeed"
			| "typemustmatch"
			| "visible" => true,
			_ => false,
		}
	}
}
