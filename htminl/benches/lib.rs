/*!
# Benchmark: `htminl`
*/

use criterion::{
	Criterion,
	criterion_group,
	criterion_main,
};
use std::path::PathBuf;



fn collapse_whitespace(c: &mut Criterion) {
	let mut group = c.benchmark_group("htminl::collapse_whitespace");

	for txt in [
		"My name is Jeffrey.\n\nI like flowers!     ",
	].iter() {
		group.bench_function(format!("{:?}", txt), move |b| {
			b.iter(|| htminl::collapse_whitespace(txt))
		});
	}

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
