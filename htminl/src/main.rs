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
extern crate fyi_core;
extern crate hyperbuild;
extern crate num_format;
extern crate rayon;
extern crate walkdir;

mod menu;

use clap::ArgMatches;
use fyi_core::{Msg, Prefix, Progress, misc, progress_arc, PROGRESS_NO_ELAPSED};
use hyperbuild::hyperbuild;
use num_format::{Locale, ToFormattedString};
use rayon::prelude::*;
use std::fs::File;
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
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
			let progress: bool = opts.is_present("progress");
			let summary: bool = opts.is_present("summary");
			let bar = Progress::new("", paths.len() as u64, PROGRESS_NO_ELAPSED);

			let old_len = AtomicU64::new(0);
			let new_len = AtomicU64::new(0);

			paths.into_par_iter().for_each(|ref x| {
				if let Ok((ol, nl)) = x.htminl_encode() {
					old_len.fetch_add(ol, Ordering::SeqCst);
					new_len.fetch_add(nl, Ordering::SeqCst);

					// Update progress?
					if true == progress {
						progress_arc::set_path(bar.clone(), &x);
						progress_arc::increment(bar.clone(), 1);
						progress_arc::tick(bar.clone());
					}
				}
			});

			// Finish progress bar if applicable.
			if true == progress {
				progress_arc::finish(bar.clone());
			}

			if true == summary {
				summarize(
					old_len.load(Ordering::SeqCst),
					new_len.load(Ordering::SeqCst)
				);
			}
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

/// Print summary.
fn summarize(old_len: u64, new_len: u64) {
	if old_len > new_len {
		let olb: String = format!(
			"{} bytes",
			old_len.to_formatted_string(&Locale::en)
		);
		let nlb: String = format!(
			"{} bytes",
			new_len.to_formatted_string(&Locale::en)
		);
		let slb: String = format!(
			"{} bytes",
			(old_len - new_len).to_formatted_string(&Locale::en)
		);

		// Print original.
		Msg::new(olb.as_str())
			.with_prefix(Prefix::Custom("Original", 4))
			.print();

		// Print minified.
		let msg: String = format!(
			"{}{}",
			misc::strings::whitespace(olb.len() - nlb.len()),
			nlb
		);
		Msg::new(msg.as_str())
			.with_prefix(Prefix::Custom("Minified", 6))
			.print();

		// Print savings.
		let msg: String = format!(
			"{}{} ({:3.*}%)",
			misc::strings::whitespace(olb.len() - slb.len()),
			slb,
			2,
			(1.0 - (new_len as f64 / old_len as f64)) * 100.0
		);
		Msg::new(msg.as_str())
			.with_prefix(Prefix::Custom(" Savings", 2))
			.print();
	}
	else {
		Msg::new("Everything was already minified.")
			.with_prefix(Prefix::Warning)
			.print();
	}
}

/// Path Helpers
pub trait PathFuckery {
	/// Encode file!
	fn htminl_encode(&self) -> Result<(u64, u64), String>;

	/// Find files.
	fn htminl_find(&self) -> Result<Vec<PathBuf>, String>;

	/// Is HTML?
	fn is_html(&self) -> bool;
}

impl PathFuckery for Path {
	/// Encode file!
	fn htminl_encode(&self) -> Result<(u64, u64), String> {
		// Load the full file contents as we'll need to reference it twice.
		let src = std::fs::read(&self).expect("Unable to read file.");
		let mut data = src.to_vec();
		let old_len: u64 = data.len() as u64;
		let mut new_len: u64 = old_len;

		if let Ok(len) = hyperbuild(&mut data) {
			// Save it?
			if 0 < len {
				let mut out = File::create(&self).expect("That didn't work!");
				out.write_all(&data[..len]).unwrap();
				out.flush().unwrap();
				new_len = len as u64;
			}

			return Ok((old_len, new_len));
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
