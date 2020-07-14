/*!
# HTML Traits: `Meta`

This module re-exports TAG and ATTRIBUTE constants, adding a few of its own.
*/



#[allow(missing_docs)]
/// Attribute Constants.
pub mod a {
	use marked::LocalName;
	use html5ever::local_name;
	pub use marked::html::a::*;

	pub const ALLOWFULLSCREEN: LocalName = local_name!("allowfullscreen");
	pub const ASYNC: LocalName = local_name!("async");
	pub const AUTOFOCUS: LocalName = local_name!("autofocus");
	pub const AUTOPLAY: LocalName = local_name!("autoplay");
	pub const CHECKED: LocalName = local_name!("checked");
	pub const COMPACT: LocalName = local_name!("compact");
	pub const DECLARE: LocalName = local_name!("declare");
	pub const DEFAULT: LocalName = local_name!("default");
	pub const DEFER: LocalName = local_name!("defer");
	pub const DISABLED: LocalName = local_name!("disabled");
	pub const FOR: LocalName = local_name!("for");
	pub const FORMNOVALIDATE: LocalName = local_name!("formnovalidate");
	pub const ISMAP: LocalName = local_name!("ismap");
	pub const ITEMSCOPE: LocalName = local_name!("itemscope");
	pub const LOOP: LocalName = local_name!("loop");
	pub const MULTIPLE: LocalName = local_name!("multiple");
	pub const MUTED: LocalName = local_name!("muted");
	pub const NOHREF: LocalName = local_name!("nohref");
	pub const NOMODULE: LocalName = local_name!("nomodule");
	pub const NORESIZE: LocalName = local_name!("noresize");
	pub const NOSHADE: LocalName = local_name!("noshade");
	pub const NOVALIDATE: LocalName = local_name!("novalidate");
	pub const OPEN: LocalName = local_name!("open");
	pub const PLACEHOLDER: LocalName = local_name!("placeholder");
	pub const READONLY: LocalName = local_name!("readonly");
	pub const REQUIRED: LocalName = local_name!("required");
	pub const SCOPED: LocalName = local_name!("scoped");
	pub const SEAMLESS: LocalName = local_name!("seamless");
	pub const SELECTED: LocalName = local_name!("selected");
	pub const SRCSET: LocalName = local_name!("srcset");
}

/// Tag Constants.
pub mod t {
	use marked::LocalName;
	pub use marked::html::t::*;

	lazy_static::lazy_static! {
		/// Vue Transition.
		pub static ref TRANSITION: LocalName = "transition".into();
	}
}

