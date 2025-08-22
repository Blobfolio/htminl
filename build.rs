/*!
# HTMinL: Build
*/

use argyle::KeyWordsBuilder;
use std::path::PathBuf;



/// # Build.
///
/// We might as well pre-compile the arguments and extensions we're looking for.
fn main() {
	println!("cargo:rerun-if-env-changed=CARGO_PKG_VERSION");

	build_cli();
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

/// # Output Path.
///
/// Append the sub-path to OUT_DIR and return it.
fn out_path(stub: &str) -> PathBuf {
	std::fs::canonicalize(std::env::var("OUT_DIR").expect("Missing OUT_DIR."))
		.expect("Missing OUT_DIR.")
		.join(stub)
}
