##
# Development Recipes
#
# This justfile is intended to be run from inside a Docker sandbox:
# https://github.com/Blobfolio/righteous-sandbox
#
# docker run \
#	--rm \
#	-v "{{ invocation_directory() }}":/share \
#	-it \
#	--name "righteous_sandbox" \
#	"righteous/sandbox:debian"
#
# Alternatively, you can just run cargo commands the usual way and ignore these
# recipes.
##

pkg_id      := "htminl"
pkg_name    := "HTMinL"
pkg_dir1    := justfile_directory() + "/htminl"
pkg_dir2    := justfile_directory() + "/htminl_core"

cargo_dir   := "/tmp/" + pkg_id + "-cargo"
cargo_bin   := cargo_dir + "/x86_64-unknown-linux-gnu/release/" + pkg_id
data_dir    := "/tmp/bench-data"
doc_dir     := justfile_directory() + "/doc"
release_dir := justfile_directory() + "/release"

rustflags   := "-C link-arg=-s"



# Benchmark Rust functions.
bench BENCH="" FILTER="":
	#!/usr/bin/env bash

	clear

	if [ -z "{{ BENCH }}" ]; then
		cargo bench \
			-q \
			--workspace \
			--all-features \
			--target x86_64-unknown-linux-gnu \
			--target-dir "{{ cargo_dir }}" -- "{{ FILTER }}"
	else
		cargo bench \
			-q \
			--bench "{{ BENCH }}" \
			--workspace \
			--all-features \
			--target x86_64-unknown-linux-gnu \
			--target-dir "{{ cargo_dir }}" -- "{{ FILTER }}"
	fi

	exit 0


# Bench Bin.
bench-bin DIR NATIVE="":
	#!/usr/bin/env bash

	# Validate directory.
	if [ ! -d "{{ DIR }}" ]; then
		fyi error "Invalid directory."
		exit 1
	fi

	clear

	# Before Stats.
	before=$( find "{{ DIR }}" \
		\( -iname "*.htm" -o -iname "*.html" \) \
		-type f \
		-print0 | \
			xargs -r0 du -scb | \
				tail -n 1 | \
					cut -f 1 )

	if [ -z "{{ NATIVE }}" ]; then
		# Make sure we have a bin built.
		[ -f "{{ cargo_bin }}" ] || just build

		fyi print -p "{{ cargo_bin }}" -c 199 "$( "{{ cargo_bin }}" -V )"

		start_time="$(date -u +%s.%N)"
		"{{ cargo_bin }}" "{{ DIR }}"
		end_time="$(date -u +%s.%N)"
		elapsed="$(bc <<<"$end_time-$start_time")"
	elif [ -f "{{ NATIVE }}" ]; then
		echo Native
	else
		fyi print -p "$( command -v html-minifier )" -c 199 "$( html-minifier -V )"

		start_time="$(date -u +%s.%N)"

		for i in $( find "{{ DIR }}" \( -iname "*.htm" -o -iname "*.html" \) -type f ! -size 0 | sort ); do
			html-minifier \
				--collapse-boolean-attributes \
				--collapse-whitespace \
				--decode-entities \
				--remove-attribute-quotes \
				--remove-comments \
				--remove-empty-attributes \
				--remove-optional-tags \
				--remove-optional-tags \
				--remove-redundant-attributes \
				--remove-redundant-attributes \
				--remove-script-type-attributes \
				--remove-style-link-type-attributes \
				-o "$i" \
				"$i" >/dev/null 2>&1 || true
		done

		end_time="$(date -u +%s.%N)"
		elapsed="$(bc <<<"$end_time-$start_time")"
	fi

	# After Stats.
	after=$( find "{{ DIR }}" \
		\( -iname "*.htm" -o -iname "*.html" \) \
		-type f \
		-print0 | \
			xargs -r0 du -scb | \
				tail -n 1 | \
					cut -f 1 )

	# Print the info!
	fyi blank
	fyi print -p "Elapsed" -c 15 "${elapsed} seconds"
	fyi print -p " Before" -c 53 "${before} bytes"
	fyi print -p "  After" -c 53 "${after} bytes"


# Build Release!
@build: clean
	# First let's build the Rust bit.
	RUSTFLAGS="--emit asm {{ rustflags }}" cargo build \
		--bin "{{ pkg_id }}" \
		--release \
		--target x86_64-unknown-linux-gnu \
		--target-dir "{{ cargo_dir }}"


# Build Debian package!
@build-deb: build-man build
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
@build-man:
	# Pre-clean.
	find "{{ pkg_dir1 }}/misc" -name "{{ pkg_id }}.1*" -type f -delete

	# Build a quickie version with the unsexy help so help2man can parse it.
	RUSTFLAGS="{{ rustflags }}" cargo build \
		--bin "{{ pkg_id }}" \
		--release \
		--all-features \
		--target x86_64-unknown-linux-gnu \
		--target-dir "{{ cargo_dir }}"

	# Clean up the BASH completion script.
	just _fix-chown "{{ pkg_dir1 }}/misc/{{ pkg_id }}.bash"
	chmod 644 "{{ pkg_dir1 }}/misc/{{ pkg_id }}.bash"

	# Use help2man to make a crappy MAN page.
	help2man -o "{{ pkg_dir1 }}/misc/{{ pkg_id }}.1" \
		-N "{{ cargo_bin }}"

	# Gzip it and reset ownership.
	gzip -k -f -9 "{{ pkg_dir1 }}/misc/{{ pkg_id }}.1"
	just _fix-chown "{{ pkg_dir1 }}"


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

	# But some Cargo apps place shit in subdirectories even if
	# they place *other* shit in the designated target dir. Haha.
	[ ! -d "{{ justfile_directory() }}/target" ] || rm -rf "{{ justfile_directory() }}/target"
	[ ! -d "{{ pkg_dir1 }}/target" ] || rm -rf "{{ pkg_dir1 }}/target"
	[ ! -d "{{ pkg_dir2 }}/target" ] || rm -rf "{{ pkg_dir2 }}/target"


# Clippy.
@clippy:
	clear
	RUSTFLAGS="{{ rustflags }}" cargo clippy \
		--workspace \
		--release \
		--all-features \
		--target x86_64-unknown-linux-gnu \
		--target-dir "{{ cargo_dir }}"


# Build Docs.
doc:
	#!/usr/bin/env bash

	# Make sure nightly is installed; this version generates better docs.
	rustup install nightly

	# Make the docs.
	cargo +nightly doc \
		--workspace \
		--release \
		--no-deps \
		--target x86_64-unknown-linux-gnu \
		--target-dir "{{ cargo_dir }}"

	# Move the docs and clean up ownership.
	[ ! -d "{{ doc_dir }}" ] || rm -rf "{{ doc_dir }}"
	mv "{{ cargo_dir }}/x86_64-unknown-linux-gnu/doc" "{{ justfile_directory() }}"
	just _fix-chown "{{ doc_dir }}"

	exit 0


# Test Run.
@run +ARGS:
	RUSTFLAGS="{{ rustflags }}" cargo run \
		--bin "{{ pkg_id }}" \
		--release \
		--target x86_64-unknown-linux-gnu \
		--target-dir "{{ cargo_dir }}" \
		-- {{ ARGS }}


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
	just _version "{{ pkg_dir2 }}" "$_ver2"


# Set version for real.
@_version DIR VER:
	[ -f "{{ DIR }}/Cargo.toml" ] || exit 1

	# Set the release version!
	toml set "{{ DIR }}/Cargo.toml" package.version "{{ VER }}" > /tmp/Cargo.toml
	just _fix-chown "/tmp/Cargo.toml"
	mv "/tmp/Cargo.toml" "{{ DIR }}/Cargo.toml"


# Benchmark data.
_bench-init:
	#!/usr/bin/env bash

	# Make sure the data dir is set up.
	[ -d "{{ data_dir }}" ] || mkdir "{{ data_dir }}"

	# Pull some test assets.
	if [ ! -d "{{ data_dir }}/raw" ]; then
		mkdir "{{ data_dir }}/raw"

		# Vue JS.
		fyi blank
		fyi task "VueJS.org"
		git clone --single-branch \
			-b master \
			https://github.com/vuejs/vuejs.org \
			"{{ data_dir }}/raw/tmp"
		cd "{{ data_dir }}/raw/tmp" && npm i && npm run build
		mv "{{ data_dir }}/raw/tmp/public" "{{ data_dir }}/raw/vue"
		cd "{{ justfile_directory() }}"
		rm -rf "{{ data_dir }}/raw/tmp"

		# Build site docs.
		just doc
		cp -aR "{{ doc_dir }}" "{{ data_dir }}/raw/"
	fi

	# Fix permissions.
	just _fix-chown "{{ data_dir }}"


# Reset benchmarks.
@_bench-reset: _bench-init
	[ ! -d "{{ data_dir }}/test" ] || rm -rf "{{ data_dir }}/test"
	cp -aR "{{ data_dir }}/raw" "{{ data_dir }}/test"


# Init dependencies.
@_init:
	[ ! -f "{{ justfile_directory() }}/Cargo.lock" ] || rm "{{ justfile_directory() }}/Cargo.lock"
	cargo update


# Fix file/directory permissions.
@_fix-chmod PATH:
	[ ! -e "{{ PATH }}" ] || find "{{ PATH }}" -type f -exec chmod 0644 {} +
	[ ! -e "{{ PATH }}" ] || find "{{ PATH }}" -type d -exec chmod 0755 {} +


# Fix file/directory ownership.
@_fix-chown PATH:
	[ ! -e "{{ PATH }}" ] || chown -R --reference="{{ justfile() }}" "{{ PATH }}"
