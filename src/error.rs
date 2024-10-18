/*!
# HTML Library: Errors
*/

use fyi_msg::ProglessError;
use std::{
	error::Error,
	fmt,
};



/// # Help Text.
const HELP: &str = concat!(r"
     __,---.__
  ,-'         `-.__
&/           `._\ _\
/               ''._    ", "\x1b[38;5;199mHTMinL\x1b[0;38;5;69m v", env!("CARGO_PKG_VERSION"), "\x1b[0m", r#"
|   ,             (∞)   Fast, safe, in-place
|__,'`-..--|__|--''     HTML minification.

USAGE:
    htminl [FLAGS] [OPTIONS] <PATH(S)>...

FLAGS:
    -h, --help        Print help information and exit.
    -p, --progress    Show progress bar while minifying.
    -V, --version     Print program version and exit.

OPTIONS:
    -l, --list <FILE> Read (absolute) file and/or directory paths from this
                      text file — or STDIN if "-" — one entry per line, instead
                      of or in addition to the trailing <PATH(S)>.

ARGS:
    <PATH(S)>...      One or more files or directories to compress.
"#);



#[expect(clippy::missing_docs_in_private_items, reason = "Self-explanatory.")]
#[derive(Debug, Copy, Clone)]
/// # Generic Error.
pub(super) enum HtminlError {
	EmptyFile,
	NoDocuments,
	Parse,
	Progress(ProglessError),
	Read,
	PrintHelp,    // Not an error.
	PrintVersion, // Not an error.
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

impl From<ProglessError> for HtminlError {
	#[inline]
	fn from(src: ProglessError) -> Self { Self::Progress(src) }
}

impl HtminlError {
	/// # As Str.
	pub(super) const fn as_str(self) -> &'static str {
		match self {
			Self::EmptyFile => "The file is empty.",
			Self::NoDocuments => "No documents were found.",
			Self::Parse => "Unable to parse the document.",
			Self::Progress(e) => e.as_str(),
			Self::Read => "Unable to read the file.",
			Self::PrintHelp => HELP,
			Self::PrintVersion => concat!("HTMinL v", env!("CARGO_PKG_VERSION")),
		}
	}
}
