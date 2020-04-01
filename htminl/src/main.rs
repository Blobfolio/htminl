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
extern crate rayon;

mod menu;

use clap::ArgMatches;
use fyi_core::{
	Msg,
	Progress,
	progress_arc,
	PROGRESS_NO_ELAPSED
};
use fyi_core::witcher::{
	self,
	mass::FYIMassOps,
	ops::FYIOps,
};
use hyperbuild::hyperbuild;
use rayon::prelude::*;
use std::path::PathBuf;
use std::time::Instant;



fn main() -> Result<(), String> {
	// Command line arguments.
	let opts: ArgMatches = menu::menu()
		.get_matches();

	let pattern = witcher::pattern_to_regex(r"(?i).+\.html?$");

	// What path are we dealing with?
	let paths: Vec<PathBuf> = match opts.is_present("list") {
		false => opts.values_of("path").unwrap()
			.into_iter()
			.filter_map(|x| Some(PathBuf::from(x)))
			.collect::<Vec<PathBuf>>()
			.fyi_walk_filtered(&pattern),
		true => PathBuf::from(opts.value_of("list").unwrap_or(""))
			.fyi_walk_file_lines(Some(pattern)),
	};

	if paths.is_empty() {
		return Err("No HTML files were found.".to_string());
	}

	// With progress.
	if opts.is_present("progress") {
		let time = Instant::now();
		let before: u64 = paths.fyi_file_sizes();
		let found: u64 = paths.len() as u64;

		{
			let bar = Progress::new("", found, PROGRESS_NO_ELAPSED);
			paths.par_iter().for_each(|ref x| {
				let _ = x.encode().is_ok();

				progress_arc::set_path(bar.clone(), &x);
				progress_arc::increment(bar.clone(), 1);
				progress_arc::tick(bar.clone());
			});
			progress_arc::finish(bar.clone());
		}

		let after: u64 = paths.fyi_file_sizes();
		Msg::msg_crunched_in(found, time, Some((before, after)))
			.print();
	}
	// Without progress.
	else {
		paths.par_iter().for_each(|ref x| {
			let _ = x.encode().is_ok();
		});
	}

	Ok(())
}

/// Encoding!
pub trait HTMinLEncode {
	/// Encode.
	fn encode(&self) -> Result<(), String>;
}

impl HTMinLEncode for PathBuf {
	/// Encode.
	fn encode(&self) -> Result<(), String> {
		// Load it.
		let mut data = self.fyi_read()?;

		if let Ok(len) = hyperbuild(&mut data) {
			// Save it?
			if 0 < len {
				self.fyi_write(&data[..len])?;
			}

			return Ok(());
		}

		Err("Unable to minify file.".into())
	}
}
