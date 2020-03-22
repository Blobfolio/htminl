/*!
# HTMinL

In-place minification of HTML file(s).
*/

#![warn(missing_docs)]
#![warn(trivial_casts)]
#![warn(trivial_numeric_casts)]
#![warn(unused_import_braces)]

#![deny(missing_copy_implementations)]
#![deny(missing_debug_implementations)]

extern crate clap;
extern crate hyperbuild;
extern crate rayon;
extern crate walkdir;

mod menu;

use clap::ArgMatches;
use hyperbuild::hyperbuild;
use rayon::prelude::*;
use std::fs::File;
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;



fn main() -> Result<(), String> {
	// Command line arguments.
	let opts: ArgMatches = menu::menu()
		.get_matches();

	// What path are we dealing with?
	let path: PathBuf = PathBuf::from(opts.value_of("path").expect("A path is required."));

	// Recurse a directory.
	if path.is_dir() {
		// Loop and compress!
		if let Ok(paths) = path.htminl_find() {
			paths.into_par_iter().for_each(|ref x| {
				let _noop = x.htminl_encode().is_ok();
			});
		}
	}
	// Just hit one file.
	else if path.is_file() {
		if false == path.htminl_encode().is_ok() {
			return Err("No files were compressed.".to_string());
		}
	}
	// There's nothing to do!
	else {
		return Err("No files were compressed.".to_string());
	}

	Ok(())
}

/// Path Helpers
pub trait PathFuckery {
	/// Encode file!
	fn htminl_encode(&self) -> Result<(), String>;

	/// Find files.
	fn htminl_find(&self) -> Result<Vec<PathBuf>, String>;

	/// Is HTML?
	fn is_html(&self) -> bool;
}

impl PathFuckery for Path {
	/// Encode file!
	fn htminl_encode(&self) -> Result<(), String> {
		// Load the full file contents as we'll need to reference it twice.
		let src = std::fs::read(&self).expect("Unable to read file.");
		let mut data = src.to_vec();

		if let Ok(len) = hyperbuild(&mut data) {
			// Save it?
			if len < data.len() {
				let mut out = File::create(&self).expect("That didn't work!");
				out.write_all(&data[..len]).unwrap();
				out.flush().unwrap();
			}

			return Ok(());
		}

		Err("Unable to minify file.".into())
	}

	/// Find files.
	fn htminl_find(&self) -> Result<Vec<PathBuf>, String> {
		let paths: Vec<PathBuf> = WalkDir::new(&self)
			.follow_links(true)
			.into_iter()
			.filter_map(|x| match x {
				Ok(path) => {
					let path = path.path();
					if path.is_html() {
						Some(path.to_path_buf())
					}
					else {
						None
					}
				},
				_ => None,
			})
			.collect();

		match paths.is_empty() {
			false => Ok(paths),
			true => Err("Invalid path.".into())
		}
	}

	/// Is HTML?
	fn is_html(&self) -> bool {
		if self.is_file() {
			let name = self.to_str()
				.unwrap_or("")
				.to_string()
				.to_lowercase();

			return name.ends_with(".html") || name.ends_with(".htm");
		}

		false
	}
}
