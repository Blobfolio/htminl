/*!
# Benchmark: `htminl`
*/

use criterion::{
	Criterion,
	criterion_group,
	criterion_main,
};
use htminl::traits::MinifyStrTendril;
use std::path::PathBuf;
use tendril::StrTendril;



fn collapse_whitespace(c: &mut Criterion) {
	let mut group = c.benchmark_group("htminl::collapse_whitespace");

	group.bench_function(format!("{:?}", "My name is 	Jeffrey.\n\nI like flowers!     "), move |b| {
		b.iter_with_setup(||
			StrTendril::from("My name is 	Jeffrey.\n\nI like flowers!     "),
			|mut st| st.collapse_whitespace()
		)
	});

	group.finish();
}

fn minify_html(c: &mut Criterion) {
	let mut group = c.benchmark_group("htminl::minify_html");

	let path = PathBuf::from("../test-assets/blobfolio.com.html");
	assert!(path.is_file());

	group.bench_function("blobfolio.com.html", move |b| {
		b.iter_with_setup(||
			std::fs::read(&path).unwrap(),
			|mut t| htminl::minify_html(&mut t)
		)
	});

	group.finish();
}

fn post_minify(c: &mut Criterion) {
	let mut group = c.benchmark_group("htminl::post_minify");

	let path = PathBuf::from("../test-assets/peanut.min.html");
	assert!(path.is_file());

	group.bench_function("peanut.min.html", move |b| {
		b.iter_with_setup(||
			std::fs::read(&path).unwrap(),
			|mut t| htminl::post_minify(&mut t)
		)
	});

	group.finish();
}



criterion_group!(
	benches,
	minify_html,
	collapse_whitespace,
	post_minify,
);
criterion_main!(benches);
