/*!
# HTMinL: Build
*/

use argyle::KeyWordsBuilder;
use dowser::Extension;
use std::{
	fs::File,
	io::Write,
	path::{
		Path,
		PathBuf,
	},
};



/// # Build.
///
/// We might as well pre-compile the arguments and extensions we're looking for.
fn main() {
	println!("cargo:rerun-if-env-changed=CARGO_PKG_VERSION");

	build_cli();
	build_ext();
}

/// # Build CLI Keys.
fn build_cli() {
	let mut builder = KeyWordsBuilder::default();
	builder.push_keys([
		"-h", "--help",
		"-p", "--progress",
		"-V", "--version",
	]);
	builder.push_keys_with_values(["-l", "--list"]);
	builder.save(out_path("argyle.rs"));
}

/// # Build Extensions.
fn build_ext() {
	let out = format!(
		r"
/// # Extension: HTM.
const E_HTM: Extension = {};

/// # Extension: HTML.
const E_HTML: Extension = {};
",
		Extension::codegen(b"htm"),
		Extension::codegen(b"html"),
	);

	write(&out_path("htminl-extensions.rs"), out.as_bytes());
}

/// # Write File.
fn write(path: &Path, data: &[u8]) {
	File::create(path).and_then(|mut f| f.write_all(data).and_then(|_| f.flush()))
		.expect("Unable to write file.");
}

/// # Output Path.
///
/// Append the sub-path to OUT_DIR and return it.
fn out_path(stub: &str) -> PathBuf {
	std::fs::canonicalize(std::env::var("OUT_DIR").expect("Missing OUT_DIR."))
		.expect("Missing OUT_DIR.")
		.join(stub)
}
