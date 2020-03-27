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
	Progress,
	progress_arc,
	witcher,
	PROGRESS_NO_ELAPSED
};
use fyi_core::witcher::mass::FYIMassOps;
use fyi_core::witcher::ops::FYIOps;
use hyperbuild::hyperbuild;
use rayon::prelude::*;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;



fn main() -> Result<(), String> {
	// Command line arguments.
	let opts: ArgMatches = menu::menu()
		.get_matches();

	// What path are we dealing with?
	let mut paths: Vec<PathBuf> = opts.values_of("path").unwrap()
		.into_iter()
		.filter_map(|x| Some(PathBuf::from(x)))
		.collect();

	let pattern = witcher::pattern_to_regex(r"(?i).+\.html?$");
	paths.fyi_walk_filtered_mut(&pattern);

	if paths.is_empty() {
		return Err("No HTML files were found.".to_string());
	}

	let found: u64 = paths.len() as u64;
	let time = Instant::now();
	let progress: bool = opts.is_present("progress");
	let summary: bool = opts.is_present("summary");
	let bar = Progress::new("", paths.len() as u64, PROGRESS_NO_ELAPSED);

	let old_len = AtomicU64::new(0);
	let new_len = AtomicU64::new(0);

	paths.into_par_iter().for_each(|ref x| {
		if let Ok((ol, nl)) = x.encode() {
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
		witcher::walk_summary(
			found,
			time,
			old_len.load(Ordering::SeqCst),
			new_len.load(Ordering::SeqCst)
		);
	}

	Ok(())
}

/// Encoding!
pub trait HTMinLEncode {
	/// Encode.
	fn encode(&self) -> Result<(u64, u64), String>;
}

impl HTMinLEncode for PathBuf {
	/// Encode.
	fn encode(&self) -> Result<(u64, u64), String> {
		// Load it.
		let mut data = self.fyi_read()?;
		let old_len: u64 = data.len() as u64;
		let mut new_len: u64 = old_len;

		if let Ok(len) = hyperbuild(&mut data) {
			// Save it?
			if 0 < len {
				self.fyi_write(&data[..len])?;
				new_len = len as u64;
			}

			return Ok((old_len, new_len));
		}

		Err("Unable to minify file.".into())
	}
}
