/*!
# HTML Library: Errors
*/

use argyle::ArgyleError;
use fyi_msg::ProglessError;
use std::{
	error::Error,
	fmt,
};



#[derive(Debug, Copy, Clone)]
/// # Generic Error.
pub(super) enum HtminlError {
	Argue(ArgyleError),
	EmptyFile,
	NoDocuments,
	Parse,
	Progress(ProglessError),
	Read,
}

impl AsRef<str> for HtminlError {
	#[inline]
	fn as_ref(&self) -> &str { self.as_str() }
}

impl fmt::Display for HtminlError {
	#[inline]
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(self.as_str())
	}
}

impl Error for HtminlError {}

impl From<ArgyleError> for HtminlError {
	#[inline]
	fn from(src: ArgyleError) -> Self { Self::Argue(src) }
}

impl From<ProglessError> for HtminlError {
	#[inline]
	fn from(src: ProglessError) -> Self { Self::Progress(src) }
}

impl HtminlError {
	/// # As Str.
	pub(super) const fn as_str(self) -> &'static str {
		match self {
			Self::Argue(e) => e.as_str(),
			Self::EmptyFile => "The file is empty.",
			Self::NoDocuments => "No documents were found.",
			Self::Parse => "Unable to parse the document.",
			Self::Progress(e) => e.as_str(),
			Self::Read => "Unable to read the file.",
		}
	}
}
