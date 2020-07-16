/*!
# HTML Traits
*/

mod strtendril;
mod attribute;
mod element;
mod noderef;

pub use strtendril::MinifyStrTendril;
pub use attribute::MinifyAttribute;
pub use element::MinifyElement;
pub use noderef::MinifyNodeRef;
