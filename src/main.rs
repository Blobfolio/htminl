/*!
# HTMinL
*/

#![forbid(unsafe_code)]

#![deny(
	clippy::allow_attributes_without_reason,
	clippy::correctness,
	unreachable_pub,
)]

#![warn(
	clippy::complexity,
	clippy::nursery,
	clippy::pedantic,
	clippy::perf,
	clippy::style,

	clippy::allow_attributes,
	clippy::clone_on_ref_ptr,
	clippy::create_dir,
	clippy::filetype_is_file,
	clippy::format_push_string,
	clippy::get_unwrap,
	clippy::impl_trait_in_params,
	clippy::lossy_float_literal,
	clippy::missing_assert_message,
	clippy::missing_docs_in_private_items,
	clippy::needless_raw_strings,
	clippy::panic_in_result_fn,
	clippy::pub_without_shorthand,
	clippy::rest_pat_in_fully_bound_structs,
	clippy::semicolon_inside_block,
	clippy::str_to_string,
	clippy::string_to_string,
	clippy::todo,
	clippy::undocumented_unsafe_blocks,
	clippy::unneeded_field_pattern,
	clippy::unseparated_literal_suffix,
	clippy::unwrap_in_result,

	macro_use_extern_crate,
	missing_copy_implementations,
	missing_docs,
	non_ascii_idents,
	trivial_casts,
	trivial_numeric_casts,
	unused_crate_dependencies,
	unused_extern_crates,
	unused_import_braces,
)]

#![expect(clippy::redundant_pub_crate, reason = "Unresolvable.")]
#![expect(clippy::doc_markdown, reason = "HTMinL makes this annoying.")]



mod error;
mod htminl;

use dowser::{
	Dowser,
	Extension,
};
pub(crate) use error::HtminlError;
use fyi_msg::{
	AnsiColor,
	BeforeAfter,
	Msg,
	MsgKind,
	Progless,
};
use rayon::iter::{
	IntoParallelRefIterator,
	ParallelIterator,
};
use std::{
	path::PathBuf,
	process::ExitCode,
	sync::atomic::{
		AtomicU64,
		Ordering::Relaxed,
	},
};



/// # Extension: HTM.
const E_HTM: Extension = Extension::new("htm").unwrap();

/// # Extension: HTML.
const E_HTML: Extension = Extension::new("html").unwrap();



/// # Main.
fn main() -> ExitCode {
	match main__() {
		Ok(()) => ExitCode::SUCCESS,
		Err(e @ (HtminlError::PrintHelp | HtminlError::PrintVersion)) => {
			println!("{e}");
			ExitCode::SUCCESS
		},
		Err(e) => {
			Msg::error(e.to_string()).eprint();
			ExitCode::FAILURE
		},
	}
}

#[inline]
/// # Actual Main.
fn main__() -> Result<(), HtminlError> {
	argyle::argue! {
		Help     "-h" "--help",
		Progress "-p" "--progress",
		Version  "-V" "--version",

		@options
		List     "-l" "--list",

		@catchall-paths Path,
	}

	// Parse CLI arguments.
	let mut progress = false;
	let mut paths = Dowser::default();
	for arg in Argument::args_os() {
		match arg {
			Argument::Help =>     return Err(HtminlError::PrintHelp),
			Argument::Progress => { progress = true; },
			Argument::Version =>  return Err(HtminlError::PrintVersion),

			Argument::List(s) => {
				paths.push_paths_from_file(&s).map_err(|_| HtminlError::ListFile)?;
			},

			Argument::Path(s) => { paths = paths.with_path(s); },

			// Mistake?
			Argument::Other(s) =>   return Err(HtminlError::InvalidCli(s)),
			Argument::OtherOs(s) => return Err(HtminlError::InvalidCli(s.to_string_lossy().into_owned())),
		}
	}

	// Put it all together!
	let paths: Vec<PathBuf> = paths.filter(|p|
		matches!(Extension::from_path(p), Some(E_HTM | E_HTML))
	)
		.collect();

	if paths.is_empty() { return Err(HtminlError::NoDocuments); }

	// Sexy run-through.
	if progress {
		// Boot up a progress bar.
		let progress = Progless::try_from(paths.len())?
			.with_title(Some(Msg::new(("HTMinL", AnsiColor::Misc199), "Reticulating &splines;")));

		// Check file sizes before we start.
		let before = AtomicU64::new(0);
		let after = AtomicU64::new(0);

		// Process!
		paths.par_iter().for_each(|x|
			if let Ok(mut enc) = htminl::Htminl::try_from(x) {
				let task = progress.task(x.to_string_lossy());

				if let Ok((b, a)) = enc.minify() {
					before.fetch_add(b, Relaxed);
					after.fetch_add(a, Relaxed);
				}
				else {
					before.fetch_add(enc.size, Relaxed);
					after.fetch_add(enc.size, Relaxed);
				}

				drop(task);
			}
		);

		// Finish up.
		progress.finish();
		progress.summary(MsgKind::Crunched, "document", "documents")
			.with_bytes_saved(BeforeAfter::from((
				before.into_inner(),
				after.into_inner(),
			)))
			.print();
	}
	else {
		paths.par_iter().for_each(|x|
			if let Ok(mut enc) = htminl::Htminl::try_from(x) {
				let _res = enc.minify();
			}
		);
	}

	Ok(())
}
