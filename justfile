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
pkg_dir1    := justfile_directory()

cargo_dir   := "/tmp/" + pkg_id + "-cargo"
cargo_bin   := cargo_dir + "/release/" + pkg_id
doc_dir     := justfile_directory() + "/doc"
release_dir := justfile_directory() + "/release"

export RUSTFLAGS := "-Ctarget-cpu=x86-64-v3 -Cllvm-args=--cost-kind=throughput -Clinker-plugin-lto -Clink-arg=-fuse-ld=lld"
export CC        := "clang"
export CXX       := "clang++"
export CFLAGS    := `llvm-config --cflags` + " -march=x86-64-v3 -Wall -Wextra -flto"
export CXXFLAGS  := `llvm-config --cxxflags` + " -march=x86-64-v3 -Wall -Wextra -flto"
export LDFLAGS   := `llvm-config --ldflags` + " -fuse-ld=lld -flto"



# Bench it!
bench BENCH="":
	#!/usr/bin/env bash

	clear
	if [ -z "{{ BENCH }}" ]; then
		cargo bench \
			--benches \
			--target-dir "{{ cargo_dir }}"
	else
		cargo bench \
			--bench "{{ BENCH }}" \
			--target-dir "{{ cargo_dir }}"
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
@build:
	# First let's build the Rust bit.
	cargo build \
		--bin "{{ pkg_id }}" \
		--release \
		--target-dir "{{ cargo_dir }}"


# Build Debian package!
@build-deb: clean credits build
	# cargo-deb doesn't support target_dir flags yet.
	[ ! -d "{{ justfile_directory() }}/target" ] || rm -rf "{{ justfile_directory() }}/target"
	mv "{{ cargo_dir }}" "{{ justfile_directory() }}/target"

	# Build the deb.
	cargo-deb \
		--no-build \
		--quiet \
		-p {{ pkg_id }} \
		-o "{{ release_dir }}"

	just _fix-chown "{{ release_dir }}"
	mv "{{ justfile_directory() }}/target" "{{ cargo_dir }}"


@clean:
	# Most things go here.
	[ ! -d "{{ cargo_dir }}" ] || rm -rf "{{ cargo_dir }}"

	# But some Cargo apps place shit in subdirectories even if
	# they place *other* shit in the designated target dir. Haha.
	[ ! -d "{{ justfile_directory() }}/target" ] || rm -rf "{{ justfile_directory() }}/target"
	[ ! -d "{{ pkg_dir1 }}/target" ] || rm -rf "{{ pkg_dir1 }}/target"

	cargo update


# Clippy.
@clippy:
	clear
	cargo clippy \
		--release \
		--target-dir "{{ cargo_dir }}"


# Generate CREDITS.
@credits:
	cargo bashman -m "{{ pkg_dir1 }}/Cargo.toml" -t x86_64-unknown-linux-gnu
	just _fix-chown "{{ justfile_directory() }}/CREDITS.md"


# Build Docs.
@doc:
	# Make the docs.
	cargo doc \
		--release \
		--no-deps \
		--target-dir "{{ cargo_dir }}"

	# Move the docs and clean up ownership.
	[ ! -d "{{ doc_dir }}" ] || rm -rf "{{ doc_dir }}"
	mv "{{ cargo_dir }}/doc" "{{ justfile_directory() }}"
	just _fix-chown "{{ doc_dir }}"

	exit 0


# Test Run.
@run +ARGS:
	cargo run \
		--bin "{{ pkg_id }}" \
		--release \
		--target-dir "{{ cargo_dir }}" \
		-- {{ ARGS }}


# Unit tests!
@test:
	clear
	cargo test \
		--target-dir "{{ cargo_dir }}"
	cargo test \
		--release \
		--target-dir "{{ cargo_dir }}"


# Get/Set version.
version:
	#!/usr/bin/env bash
	set -e

	# Current version.
	_ver1="$( tomli query -f "{{ pkg_dir1 }}/Cargo.toml" package.version | \
		sed 's/[" ]//g' )"

	# Find out if we want to bump it.
	set +e
	_ver2="$( whiptail --inputbox "Set {{ pkg_name }} version:" --title "Release Version" 0 0 "$_ver1" 3>&1 1>&2 2>&3 )"

	exitstatus=$?
	if [ $exitstatus != 0 ] || [ "$_ver1" = "$_ver2" ]; then
		exit 0
	fi
	set -e

	# Set the release version!
	tomli set -f "{{ pkg_dir1 }}/Cargo.toml" -i package.version "$_ver2"

	fyi success "Set version to $_ver2."


# Reset benchmarks.
@_bench-reset:
	[ ! -d "/tmp/bench-data" ] || rm -rf "/tmp/bench-data"
	cp -aR "{{ justfile_directory() }}/skel/test-assets" "/tmp/bench-data"


# Init dependencies.
@_init:
	# We need beta until 1.51 is stable.
	# env RUSTUP_PERMIT_COPY_RENAME=true rustup default beta
	# env RUSTUP_PERMIT_COPY_RENAME=true rustup component add clippy


# Fix file/directory permissions.
@_fix-chmod PATH:
	[ ! -e "{{ PATH }}" ] || find "{{ PATH }}" -type f -exec chmod 0644 {} +
	[ ! -e "{{ PATH }}" ] || find "{{ PATH }}" -type d -exec chmod 0755 {} +


# Fix file/directory ownership.
@_fix-chown PATH:
	[ ! -e "{{ PATH }}" ] || chown -R --reference="{{ justfile() }}" "{{ PATH }}"
