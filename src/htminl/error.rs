/*!
# HTML Library: Errors
*/

use std::fmt;



#[allow(clippy::redundant_pub_crate)] // Doesn't work without it.
#[derive(Debug, Copy, Clone)]
/// # Generic Error.
pub(crate) enum HtminlError {
	EmptyFile,
	Parse,
	Read,
	Write,
}

impl fmt::Display for HtminlError {
	#[inline]
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(self.as_str())
	}
}

impl std::error::Error for HtminlError {}

impl HtminlError {
	/// # As Str.
	pub(crate) const fn as_str(self) -> &'static str {
		match self {
			Self::EmptyFile => "The file is empty.",
			Self::Parse => "Unable to parse the document.",
			Self::Read => "Unable to read the file.",
			Self::Write => "Unable to save the changes.",
		}
	}
}
