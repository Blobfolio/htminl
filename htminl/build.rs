/*!
# Build

Generate BASH completions when building.
*/

extern crate clap;

use clap::Shell;
use std::path::PathBuf;

include!("src/menu.rs");



fn main() {
	// Store the completions here.
	let outdir: PathBuf = PathBuf::from("/tmp/htminl-cargo");
	if false == outdir.is_dir() {
		std::fs::create_dir(&outdir).expect("Unable to create temporary completion directory.");
	}

	// Complete it!
	menu().gen_completions(
		"htminl",
		Shell::Bash,
		outdir
	);
}
