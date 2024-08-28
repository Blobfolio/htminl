/*!
# HTML Traits: `Meta`

This module re-exports TAG and ATTRIBUTE constants, adding a few of its own.
*/

#[expect(clippy::missing_docs_in_private_items, reason = "Self-explanatory.")]
/// Attribute Constants.
pub(super) mod a {
	use marked::{LocalName, html5ever::local_name};
	pub(crate) use marked::html::a::*;

	pub(crate) const ALLOWFULLSCREEN: LocalName = local_name!("allowfullscreen");
	pub(crate) const ASYNC: LocalName = local_name!("async");
	pub(crate) const AUTOFOCUS: LocalName = local_name!("autofocus");
	pub(crate) const AUTOPLAY: LocalName = local_name!("autoplay");
	pub(crate) const CHECKED: LocalName = local_name!("checked");
	pub(crate) const COMPACT: LocalName = local_name!("compact");
	pub(crate) const DECLARE: LocalName = local_name!("declare");
	pub(crate) const DEFAULT: LocalName = local_name!("default");
	pub(crate) const DEFER: LocalName = local_name!("defer");
	pub(crate) const DISABLED: LocalName = local_name!("disabled");
	pub(crate) const FOR: LocalName = local_name!("for");
	pub(crate) const FORMNOVALIDATE: LocalName = local_name!("formnovalidate");
	pub(crate) const ISMAP: LocalName = local_name!("ismap");
	pub(crate) const ITEMSCOPE: LocalName = local_name!("itemscope");
	pub(crate) const LOOP: LocalName = local_name!("loop");
	pub(crate) const MULTIPLE: LocalName = local_name!("multiple");
	pub(crate) const MUTED: LocalName = local_name!("muted");
	pub(crate) const NOHREF: LocalName = local_name!("nohref");
	pub(crate) const NOMODULE: LocalName = local_name!("nomodule");
	pub(crate) const NORESIZE: LocalName = local_name!("noresize");
	pub(crate) const NOSHADE: LocalName = local_name!("noshade");
	pub(crate) const NOVALIDATE: LocalName = local_name!("novalidate");
	pub(crate) const OPEN: LocalName = local_name!("open");
	pub(crate) const PLACEHOLDER: LocalName = local_name!("placeholder");
	pub(crate) const READONLY: LocalName = local_name!("readonly");
	pub(crate) const REQUIRED: LocalName = local_name!("required");
	pub(crate) const SCOPED: LocalName = local_name!("scoped");
	pub(crate) const SEAMLESS: LocalName = local_name!("seamless");
	pub(crate) const SELECTED: LocalName = local_name!("selected");
	pub(crate) const SRCSET: LocalName = local_name!("srcset");
}

/// Tag Constants.
pub(super) use marked::html::t;
