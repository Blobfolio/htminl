##
# Development Recipes
#
# This requires Just: https://github.com/casey/just
#
# To see possible tasks, run:
# just --list
##

pkg_id      := "htminl"
pkg_name    := "HTMinL"
pkg_dir1    := justfile_directory() + "/htminl"

cargo_dir   := "/tmp/" + pkg_id + "-cargo"
cargo_bin   := cargo_dir + "/x86_64-unknown-linux-gnu/release/" + pkg_id
data_dir    := "/tmp/bench-data"
pgo_dir     := "/tmp/pgo-data"
release_dir := justfile_directory() + "/release"

rustflags   := "-Clinker-plugin-lto -Clinker=clang-9 -Clink-args=-fuse-ld=lld-9 -C link-arg=-s"



# Benchmarks.
bench CLEAN="":
	#!/usr/bin/env bash

	# Force a rebuild.
	if [ ! -z "{{ CLEAN }}" ]; then
		just build-pgo
	elif [ ! -f "{{ cargo_bin }}" ]; then
		just build-pgo
	fi

	clear
	hyperfine --warmup 3 \
		--prepare 'just _bench-reset; sleep 3' \
		'just _bench-html-minifier' \
		'{{ cargo_bin }} {{ data_dir }}'

	# Let's check compression too.
	START_SIZE=$( du -scb "{{ justfile_directory() }}/test-assets" | head -n 1 | cut -f1 )

	just _bench-reset
	just _bench-html-minifier
	END_SIZE=$( du -scb "{{ data_dir }}" | head -n 1 | cut -f1 )
	echo "$(($START_SIZE-$END_SIZE)) <-- saved by html-minifier"

	just _bench-reset
	{{ cargo_bin }} {{ data_dir }}
	END_SIZE=$( du -scb "{{ data_dir }}" | head -n 1 | cut -f1 )
	echo "$(($START_SIZE-$END_SIZE)) <-- saved by htminl"


# Build Release!
@build:
	# First let's build the Rust bit.
	RUSTFLAGS="{{ rustflags }}" cargo build \
		--bin "{{ pkg_id }}" \
		--release \
		--target x86_64-unknown-linux-gnu \
		--target-dir "{{ cargo_dir }}"


# Build Debian package!
@build-deb: build-man
	# cargo-deb doesn't support target_dir flags yet.
	[ ! -d "{{ justfile_directory() }}/target" ] || rm -rf "{{ justfile_directory() }}/target"
	mv "{{ cargo_dir }}" "{{ justfile_directory() }}/target"

	# First let's build the Rust bit.
	cargo-deb \
		--no-build \
		-p {{ pkg_id }} \
		-o "{{ justfile_directory() }}/release"

	just _fix-chown "{{ release_dir }}"
	mv "{{ justfile_directory() }}/target" "{{ cargo_dir }}"


# Build Man.
@build-man: build-pgo
	# Pre-clean.
	find "{{ release_dir }}/man" -type f -delete

	# Use help2man to make a crappy MAN page.
	help2man -o "{{ release_dir }}/man/{{ pkg_id }}.1" \
		-N "{{ cargo_bin }}"

	# Strip some ugly out.
	sd '{{ pkg_name }} [0-9.]+\nBlobfolio, LLC. <hello@blobfolio.com>\n' \
		'' \
		"{{ release_dir }}/man/{{ pkg_id }}.1"

	# Gzip it and reset ownership.
	gzip -k -f -9 "{{ release_dir }}/man/{{ pkg_id }}.1"
	just _fix-chown "{{ release_dir }}/man"


# Build PGO.
@build-pgo: clean
	# First let's build the Rust bit.
	RUSTFLAGS="{{ rustflags }} -Cprofile-generate={{ pgo_dir }}" \
		cargo build \
			--bin "{{ pkg_id }}" \
			--release \
			--target x86_64-unknown-linux-gnu \
			--target-dir "{{ cargo_dir }}"

	clear

	# Instrument a few tests.
	just _bench-reset
	"{{ cargo_bin }}" "{{ data_dir }}"

	# Do them again with the UI.
	just _bench-reset
	"{{ cargo_bin }}" -p "{{ data_dir }}"

	# Do a file.
	just _bench-reset
	echo "{{ data_dir }}/blobfolio.com.html" > "/tmp/pgo-list.txt"
	"{{ cargo_bin }}" -p -l "/tmp/pgo-list.txt"
	rm "/tmp/pgo-list.txt"

	# A bunk path.
	"{{ cargo_bin }}" "/nowhere/blankety" || true

	# And some CLI screens.
	"{{ cargo_bin }}" -V
	"{{ cargo_bin }}" -h

	clear

	# Merge the data back in.
	llvm-profdata-9 \
		merge -o "{{ pgo_dir }}/merged.profdata" "{{ pgo_dir }}"

	RUSTFLAGS="{{ rustflags }} -Cprofile-use={{ pgo_dir }}/merged.profdata" \
		cargo build \
			--bin "{{ pkg_id }}" \
			--release \
			--target x86_64-unknown-linux-gnu \
			--target-dir "{{ cargo_dir }}"


# Check Release!
@check:
	# First let's build the Rust bit.
	cargo check \
		--bin "{{ pkg_id }}" \
		--release \
		--target x86_64-unknown-linux-gnu \
		--target-dir "{{ cargo_dir }}"


@clean:
	# Most things go here.
	[ ! -d "{{ cargo_dir }}" ] || rm -rf "{{ cargo_dir }}"
	[ ! -d "{{ pgo_dir }}" ] || rm -rf "{{ pgo_dir }}"

	# But some Cargo apps place shit in subdirectories even if
	# they place *other* shit in the designated target dir. Haha.
	[ ! -d "{{ justfile_directory() }}/target" ] || rm -rf "{{ justfile_directory() }}/target"
	[ ! -d "{{ pkg_dir1 }}/target" ] || rm -rf "{{ pkg_dir1 }}/target"


# Clippy.
@clippy:
	clear
	cargo clippy \
		--bin "{{ pkg_id }}" \
		--release \
		--target x86_64-unknown-linux-gnu \
		--target-dir "{{ cargo_dir }}"


# Get/Set version.
version:
	#!/usr/bin/env bash

	# Current version.
	_ver1="$( toml get "{{ pkg_dir1 }}/Cargo.toml" package.version | \
		sed 's/"//g' )"

	# Find out if we want to bump it.
	_ver2="$( whiptail --inputbox "Set {{ pkg_name }} version:" --title "Release Version" 0 0 "$_ver1" 3>&1 1>&2 2>&3 )"

	exitstatus=$?
	if [ $exitstatus != 0 ] || [ "$_ver1" = "$_ver2" ]; then
		exit 0
	fi

	fyi success "Setting version to $_ver2."

	# Set the release version!
	just _version "{{ pkg_dir1 }}" "$_ver2"


# Set version for real.
@_version DIR VER:
	[ -f "{{ DIR }}/Cargo.toml" ] || exit 1

	# Set the release version!
	toml set "{{ DIR }}/Cargo.toml" package.version "{{ VER }}" > /tmp/Cargo.toml
	just _fix-chown "/tmp/Cargo.toml"
	mv "/tmp/Cargo.toml" "{{ DIR }}/Cargo.toml"


# Wrapper for testing HTML-Minifier performance.
_bench-html-minifier:
	#!/usr/bin/env bash

	# Such a piece of shit. Haha. While there are input/output dir
	# options, they're all-or-nothing shots (and move shit around); to
	# mimic in-place, arbitrary minification, we need to pipe from
	# `find`, and we need to filter out any empty entries as that causes
	# Node to run forever and ever without making any progress.
	for i in $( find "{{ data_dir }}" -name "*.html" -type f ! -size 0 | sort ); do
		html-minifier \
			--case-sensitive \
			--collapse-whitespace \
			--decode-entities \
			--remove-comments \
			-o "$i" \
			"$i" >/dev/null 2>&1 || true
	done


# Reset benchmarks.
@_bench-reset:
	[ ! -d "{{ data_dir }}" ] || rm -rf "{{ data_dir }}"
	cp -aR "{{ justfile_directory() }}/test-assets" "{{ data_dir }}"


# Init dependencies.
@_init:
	# A hyperbuild dependency isn't working on 1.41+. Until there's a better
	# solution, we need to downgrade.
	rustup default 1.40.0
	rustup component add clippy llvm-tools-preview

	# And hyperbuild provides no configs, so we need to intervene.
	git clone \
		-b v0.0.45 \
		--single-branch \
		https://github.com/wilsonzlin/hyperbuild.git \
		/tmp/hyperbuild
	cp /share/hyperbuild.patch /tmp/hyperbuild/the.patch
	cd /tmp/hyperbuild && patch -p1 -i the.patch
	rm /tmp/hyperbuild/the.patch

	[ ! -f "{{ justfile_directory() }}/Cargo.lock" ] || rm "{{ justfile_directory() }}/Cargo.lock"
	cargo update


# Fix file/directory permissions.
@_fix-chmod PATH:
	[ ! -e "{{ PATH }}" ] || find "{{ PATH }}" -type f -exec chmod 0644 {} +
	[ ! -e "{{ PATH }}" ] || find "{{ PATH }}" -type d -exec chmod 0755 {} +


# Fix file/directory ownership.
@_fix-chown PATH:
	[ ! -e "{{ PATH }}" ] || chown -R --reference="{{ justfile() }}" "{{ PATH }}"
