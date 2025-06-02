/*!
# HTML Library: Errors
*/

use fyi_msg::{
	fyi_ansi::{
		ansi,
		csi,
		dim,
	},
	ProglessError,
};
use std::{
	error::Error,
	fmt,
};



/// # Help Text.
const HELP: &str = concat!(r"
     __,---.__
  ,-'         `-.__
&/           `._\ _\
/               ''._    ", csi!(199), "HTMinL", ansi!((cornflower_blue) " v", env!("CARGO_PKG_VERSION")), r#"
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
#[derive(Debug, Clone)]
/// # Generic Error.
pub(super) enum HtminlError {
	EmptyFile,
	InvalidCli(String),
	ListFile,
	NoDocuments,
	Parse,
	Progress(ProglessError),
	Read,
	PrintHelp,    // Not an error.
	PrintVersion, // Not an error.
}

impl fmt::Display for HtminlError {
	#[inline]
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let prefix = self.as_str();
		match self {
			Self::InvalidCli(s) => write!(
				f,
				concat!("{} ", dim!("{}")),
				prefix,
				s,
			),
			_ => f.write_str(prefix),
		}
	}
}

impl Error for HtminlError {}

impl From<ProglessError> for HtminlError {
	#[inline]
	fn from(src: ProglessError) -> Self { Self::Progress(src) }
}

impl HtminlError {
	/// # As Str.
	pub(super) const fn as_str(&self) -> &'static str {
		match self {
			Self::EmptyFile => "The file is empty.",
			Self::InvalidCli(_) => "Invalid/unknown argument:",
			Self::ListFile => "Invalid -l/--list text file.",
			Self::NoDocuments => "No documents were found.",
			Self::Parse => "Unable to parse the document.",
			Self::Progress(e) => e.as_str(),
			Self::Read => "Unable to read the file.",
			Self::PrintHelp => HELP,
			Self::PrintVersion => concat!("HTMinL v", env!("CARGO_PKG_VERSION")),
		}
	}
}
