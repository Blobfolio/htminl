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

mod dom;
mod err;
mod minify;

use dactyl::{
	NiceElapsed,
	NiceU64,
	traits::NiceInflection,
};
use dom::{
	node::{
		Handle,
		Node,
		NodeInner,
	},
	Tree,
};
use dowser::{
	Dowser,
	Extension,
};
use err::HtminlError;
use flume::Receiver;
use fyi_msg::{
	fyi_ansi::dim,
	BeforeAfter,
	Msg,
	MsgKind,
	Progless,
};
use std::{
	num::NonZeroUsize,
	path::{
		Path,
		PathBuf,
	},
	process::ExitCode,
	sync::atomic::{
		AtomicU64,
		Ordering::SeqCst,
	},
	thread,
};

/// # Extension: HTM.
const E_HTM: Extension = Extension::new("htm").unwrap();

/// # Extension: HTML.
const E_HTML: Extension = Extension::new("html").unwrap();

/// # Skip Count.
static SKIPPED: AtomicU64 = AtomicU64::new(0);

/// # Total Size Before.
static BEFORE: AtomicU64 = AtomicU64::new(0);

/// # Total Size After.
static AFTER: AtomicU64 = AtomicU64::new(0);



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

			Argument::List(s) =>
				if s == "-" { paths.push_paths_from_stdin(); }
				else {
					paths.push_paths_from_file(&s).map_err(|_| HtminlError::ListFile)?;
				},

			Argument::Path(s) => { paths = paths.with_path(s); },

			// Mistake?
			Argument::Other(s) =>   return Err(HtminlError::InvalidCli(s)),
			Argument::OtherOs(s) => return Err(HtminlError::InvalidCli(s.to_string_lossy().into_owned())),
		}
	}

	// Put it all together!
	let mut paths: Vec<PathBuf> = paths.filter(|p|
		matches!(Extension::from_path(p), Some(E_HTM | E_HTML))
	)
		.collect();
	let total = NonZeroUsize::new(paths.len()).ok_or(HtminlError::NoDocuments)?;
	paths.sort();

	// How many threads?
	let threads = thread::available_parallelism().map_or(
		NonZeroUsize::MIN,
		|t| NonZeroUsize::min(t, total),
	);

	// Set up the killswitch.
	let killed = Progless::sigint_two_strike();

	// Boot up a progress bar, if desired.
	let progress =
		if progress {
			Progless::try_from(total)
				.ok()
				.map(|p| p.with_reticulating_splines("HTMinL"))
		}
		else { None };

	// Thread business!
	let (tx, rx) = flume::bounded::<&Path>(threads.get());
	thread::scope(#[inline(always)] |s| {
		// Set up the worker threads.
		let mut workers = Vec::with_capacity(threads.get());
		for _ in 0..threads.get() {
			workers.push(s.spawn(#[inline(always)] || crunch(&rx, progress.as_ref())));
		}

		// Push all the files to it, then drop the sender to disconnect.
		for path in &paths {
			if killed.load(SeqCst) || tx.send(path).is_err() { break; }
		}
		drop(tx);

		// Sum the totals as each thread finishes.
		for worker in workers {
			worker.join().map_err(|_| HtminlError::JobServer)?;
		}

		Ok::<(), HtminlError>(())
	})?;
	drop(rx);

	// Summarize?
	if let Some(progress) = progress { summarize(&progress, total.get() as u64); }

	// Early abort?
	if killed.load(SeqCst) { Err(HtminlError::Killed) }
	else { Ok(()) }
}

#[inline(never)]
/// # Worker Callback.
///
/// This is the worker callback for HTML crunching. It listens for "new" HTML
/// paths and crunches them — and maybe updates the progress bar, etc. — then
/// quits as soon as the work has dried up.
fn crunch(rx: &Receiver::<&Path>, progress: Option<&Progless>) {
	let Some(progress) = progress else {
		// If we aren't tracking progress, the code is a lot simpler. Haha.
		while let Ok(p) = rx.recv() { let _res = minify::minify(p); }
		return;
	};

	// The pretty version.
	while let Ok(p) = rx.recv() {
		match minify::minify(p) {
			Ok((b, a)) => {
				BEFORE.fetch_add(b.get(), SeqCst);
				AFTER.fetch_add(a.get(), SeqCst);
			},
			Err(e) => {
				SKIPPED.fetch_add(1, SeqCst);
				let _res = progress.push_msg(Msg::skipped(format!(
					concat!("{} ", dim!("({})")),
					p.display(),
					e.as_str(),
				)));
			}
		}
	}
}

/// # Summarize Results.
fn summarize(progress: &Progless, total: u64) {
	let elapsed = progress.finish();
	let skipped = SKIPPED.load(SeqCst);
	if skipped == 0 {
		progress.summary(MsgKind::Crunched, "document", "documents")
	}
	else {
		// And summarize what we did do.
		Msg::crunched(format!(
			concat!(
				"{}",
				dim!("/"),
				"{} in {}.",
			),
			NiceU64::from(total - skipped),
			total.nice_inflect("document", "documents"),
			NiceElapsed::from(elapsed),
		))
	}
		.with_bytes_saved(BeforeAfter::from((
			BEFORE.load(SeqCst),
			AFTER.load(SeqCst),
		)))
		.eprint();
}
