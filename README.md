# HTMinL

[![ci](https://img.shields.io/github/actions/workflow/status/Blobfolio/htminl/ci.yaml?style=flat-square&label=ci)](https://github.com/Blobfolio/htminl/actions)
[![deps.rs](https://deps.rs/repo/github/blobfolio/htminl/status.svg?style=flat-square&label=deps.rs)](https://deps.rs/repo/github/blobfolio/htminl)<br>
[![license](https://img.shields.io/badge/license-wtfpl-ff1493?style=flat-square)](https://en.wikipedia.org/wiki/WTFPL)
[![contributions welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg?style=flat-square&label=contributions)](https://github.com/Blobfolio/htminl/issues)

HTMinL is a CLI tool for x86-64 Linux machines that simplifies the task of minifying HTML in-place for production environments.



## Features

HTMinL is a _fast_, in-place HTML minifier. It prioritizes safety and code sanity over _ULTIMATE COMPRESSION_, so may not save quite as many bytes as other tools, but it's also less likely to break shit. Haha.

Critically, HTMinL is _not_ a stream processor; it constructs a complete DOM tree from the full source _before_ getting down to business. This allows for much more accurate processing and robust error recovery.

See the [minification](#minification) section for more details about the process, as well as the [cautions](#cautions) section for important assumptions, requirements, gotchas, etc.



## Installation

Debian and Ubuntu users can just grab the pre-built `.deb` package from the [latest release](https://github.com/Blobfolio/htminl/releases/latest).

This application is written in [Rust](https://www.rust-lang.org/) and can alternatively be built/installed from source using [Cargo](https://github.com/rust-lang/cargo):

```bash
# See "cargo install --help" for more options.
cargo install \
    --git https://github.com/Blobfolio/htminl.git \
    --bin htminl
```



## Usage

It's easy. Just run `htminl [FLAGS] [OPTIONS] <PATH(S)>…`.

The following flags and options are available:

| Short | Long | Value | Description |
| ----- | ---- | ----- | ----------- |
| `-h` | `--help` | | Print help information and exit. |
| `-l` | `--list` | `<FILE>` | Read (absolute) file and/or directory paths from this text file — or STDIN if "-" — one entry per line, instead of or in addition to the trailing `<PATH(S)>`. |
| `-p` | `--progress` | | Show progress bar while minifying. |
| `-V` | `--version` | | Print program version and exit. |

Paths can be specified as trailing command arguments, and/or loaded via text file (with one path per line) with the `-l` option. Directories are scanned recursively for `.htm`/`.html`.

Some quick examples:

```bash
# Minify one file.
htminl /path/to/index.html

# Tackle a whole folder at once with a nice progress bar:
htminl -p /path/to/html

# Or load it up with a lot of places separately:
htminl /path/to/html /path/to/index.html …
```



## Minification

Browsers aren't very picky about whitespace, so unsurprisingly, most savings are achieved by simply "collapsing" contiguous regions of mixed whitespace into single horizontal spaces.

HTMinL does this automatically for most text nodes — except those inside `<code>`, `<plaintext>`, `<pre>`, `<script>`, `<style>`, `<svg>`, `<textarea>` — and conservatively trims/drops text nodes from a few non-renderable regions like `<head>`, but doesn't push its luck.

_Lots_ of people use layout elements for content, or vice versa; a few leftover bytes aren't worth quibbling over.

Besides, there are all sorts of _other_ things that can be stripped, like:

* Default `type` attributes on `<script>` and `<style>`;
* HTML comments;
* Implied values on boolean attributes like `disabled`, `readonly`, etc.;
* Trailing slashes on void HTML elements;
* Whitespace between element attributes;
* XML processing instructions;
 
But wait, there's more!

HTMinL also:

* Converts CRLF/CR (literals) globally to `\n`;
* Normalizes element tag casing;
* Quotes attribute values with `'` when shorter than `"`;
* Rewrites the doctype as `<!DOCTYPE html>`;
* Self-closes childless SVG tags;



## Cautions

While care has been taken to balance savings and safety, there are some (intentional) limitations to be aware of:

* Documents are expected to be encoded in UTF-8;
* Documents are processed as **HTML**, _not_ XML or XHTML; inline SVG elements should be okay, but other XMLish data may be corrupted;
* Whitespace collapsing _can_ change how content is rendered in cases where `whitespace: pre` is applied willynilly to elements that wouldn't normally have it;
