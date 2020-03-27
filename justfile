##
# Development Recipes
#
# This requires Just: https://github.com/casey/just
#
# To see possible tasks, run:
# just --list
##

cargo_dir     := "/tmp/htminl-cargo"
data_dir      := "/tmp/bench-data"
debian_dir    := "/tmp/htminl-release/htminl"
release_dir   := justfile_directory() + "/release"

build_ver     := "1"



# Build Release!
@build:
	# First let's build the Rust bit.
	RUSTFLAGS="-C link-arg=-s" cargo build \
		--release \
		--target-dir "{{ cargo_dir }}"


# Build Debian Package.
@build-debian: build
	[ ! -e "{{ debian_dir }}" ] || rm -rf "{{ debian_dir }}"
	mkdir -p "{{ debian_dir }}/DEBIAN"
	mkdir -p "{{ debian_dir }}/etc/bash_completion.d"
	mkdir -p "{{ debian_dir }}/usr/bin"
	mkdir -p "{{ debian_dir }}/usr/share/man/man1"

	# Steal the version from Cargo.toml really quick.
	cat "{{ justfile_directory() }}/htminl/Cargo.toml" | grep version | head -n 1 | sed 's/[^0-9\.]//g' > "/tmp/VERSION"

	# Copy the application.
	cp -a "{{ cargo_dir }}/release/htminl" "{{ debian_dir }}/usr/bin"
	chmod 755 "{{ debian_dir }}/usr/bin/htminl"
	strip "{{ debian_dir }}/usr/bin/htminl"

	# Generate completions.
	cp -a "{{ cargo_dir }}/htminl.bash" "{{ debian_dir }}/etc/bash_completion.d"
	chmod 644 "{{ debian_dir }}/etc/bash_completion.d/htminl.bash"

	# Set up the control file.
	cp -a "{{ release_dir }}/skel/control" "{{ debian_dir }}/DEBIAN"
	sed -i "s/VERSION/$( cat "/tmp/VERSION" )-{{ build_ver }}/g" "{{ debian_dir }}/DEBIAN/control"
	sed -i "s/SIZE/$( du -scb "{{ debian_dir }}/usr" | tail -n 1 | awk '{print $1}' )/g" "{{ debian_dir }}/DEBIAN/control"

	# Generate the manual.
	just _build-man

	# Build the Debian package.
	chown -R root:root "{{ debian_dir }}"
	cd "$( dirname "{{ debian_dir }}" )" && dpkg-deb --build htminl
	chown --reference="{{ justfile() }}" "$( dirname "{{ debian_dir }}" )/htminl.deb"

	# And a touch of clean-up.
	mv "$( dirname "{{ debian_dir }}" )/htminl.deb" "{{ release_dir }}/htminl_$( cat "/tmp/VERSION" )-{{ build_ver }}.deb"
	rm -rf "/tmp/VERSION" "{{ debian_dir }}"


# Benchmarks.
bench: _bench_init
	#!/usr/bin/env bash

	[ -f "{{ cargo_dir }}/release/htminl" ] || just build
	clear

	fyi print -p Method "(Find + Parallel + html-minifier)"

	[ ! -d "{{ data_dir }}/test" ] || rm -rf "{{ data_dir }}/test"
	cp -aR "{{ data_dir }}/raw" "{{ data_dir }}/test"
	time just _bench-html-minifier >/dev/null 2>&1
	rm -rf "{{ data_dir }}/test"

	echo ""
	fyi print -p Method "HTMinL"

	[ ! -d "{{ data_dir }}/test" ] || rm -rf "{{ data_dir }}/test"
	cp -aR "{{ data_dir }}/raw" "{{ data_dir }}/test"
	time "{{ cargo_dir }}/release/htminl" "{{ data_dir }}/test"
	rm -rf "{{ data_dir }}/test"


@_bench-html-minifier:
	find "{{ data_dir }}/test" \
		-name "*.html" \
		-type f \
		-print0 | \
		parallel -0 html-minifier \
			--case-sensitive \
			--collapse-whitespace \
			--decode-entities \
			--remove-comments \
			-o {} \
			{}


# Benchmark data.
_bench_init:
	#!/usr/bin/env bash

	[ -d "{{ data_dir }}" ] || mkdir "{{ data_dir }}"
	if [ ! -f "{{ data_dir }}/list.csv" ]; then
		wget -O "{{ data_dir }}/list.csv" "https://moz.com/top-500/download/?table=top500Domains"
		sed -i 1d "{{ data_dir }}/list.csv"
	fi

	if [ ! -d "{{ data_dir }}/raw" ]; then
		fyi info "Gathering Top 500 Sites."
		mkdir "{{ data_dir }}/raw"
		echo "" > "{{ data_dir }}/raw.txt"

		while IFS=, read -r field1 field2 field3
		do
			dom="$( echo "$field2" | sd -s '"' '' )"
			[ -z "$dom" ] || echo "https://$dom" >> "{{ data_dir }}/raw.txt"
		done < "{{ data_dir }}/list.csv"

		_user="\"Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/74.0.3729.169 Safari/537.36\""

		cd "{{ data_dir }}/raw"
		parallel --gnu --jobs 50 -a "{{ data_dir }}/raw.txt" wget -q -T5 -t1 -E html -U "$_user"
	fi

	exit 0


# Build MAN page.
@_build-man:
	# Most of it can come straight from the help screen.
	help2man -N \
		"{{ debian_dir }}/usr/bin/htminl" > "{{ debian_dir }}/usr/share/man/man1/htminl.1"

	# Fix a few formatting quirks.
	sed -i -e ':a' -e 'N' -e '$!ba' -Ee \
		"s#HTMinL [0-9\.]+[\n]Blobfolio, LLC. <hello@blobfolio.com>[\n]##g" \
		"{{ debian_dir }}/usr/share/man/man1/htminl.1"

	# Wrap up by gzipping to save some space.
	gzip -9 "{{ debian_dir }}/usr/share/man/man1/htminl.1"


# Get/Set HTMinL version.
version:
	#!/usr/bin/env bash

	# Current version.
	_ver1="$( cat "{{ justfile_directory() }}/htminl/Cargo.toml" | \
		grep version | \
		head -n 1 | \
		sed 's/[^0-9\.]//g' )"

	# Find out if we want to bump it.
	_ver2="$( whiptail --inputbox "Set HTMinL version:" --title "Release Version" 0 0 "$_ver1" 3>&1 1>&2 2>&3 )"

	exitstatus=$?
	if [ $exitstatus != 0 ] || [ "$_ver1" = "$_ver2" ]; then
		exit 0
	fi

	fyi success "Setting plugin version to $_ver2."

	# Set the release version!
	just _version "{{ justfile_directory() }}/htminl/Cargo.toml" "$_ver2" >/dev/null 2>&1


# Truly set version.
_version TOML VER:
	#!/usr/bin/env php
	<?php
	if (! is_file("{{ TOML }}") || ! preg_match('/^\d+.\d+.\d+$/', "{{ VER }}")) {
		exit(1);
	}

	$content = file_get_contents("{{ TOML }}");
	$content = explode("\n", $content);
	$section = null;

	foreach ($content as $k=>$v) {
		if (\preg_match('/^\[[^\]]+\]$/', $v)) {
			$section = $v;
			continue;
		}
		elseif ('[package]' === $section && 0 === \strpos($v, 'version')) {
			$content[$k] = \sprintf(
				'version = "%s"',
				"{{ VER }}"
			);
			break;
		}
	}

	$content = implode("\n", $content);
	file_put_contents("{{ TOML }}", $content);


# Init dependencies.
@_init:
	# A hyperbuild dependency isn't working on 1.41+.
	rustup default 1.40.0

	# And hyperbuild provides no configs, so we need to intervene.
	git clone \
		-b v0.0.44 \
		--single-branch \
		https://github.com/wilsonzlin/hyperbuild.git \
		/tmp/hyperbuild
	cp /share/hyperbuild.patch /tmp/hyperbuild/the.patch
	cd /tmp/hyperbuild && patch -p1 -i the.patch
	rm /tmp/hyperbuild/the.patch
