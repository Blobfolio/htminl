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

HTMinL performs a lot of little optimizations to shrink the size of documents without affecting how they're rendered by web browsers, like:

* Normalizing tag/attribute casing;
* Removing (default) `type` attributes on `<script>` and `<style>` tags;
* Removing HTML comments;
* Removing implied values on boolean HTML attributes;
* Removing trailing slashes from void HTML element tags;
* Removing XML processing instructions;
* Replacing CRLF/CR literals with LF;
* Rewriting the doctype as `<!DOCTYPE html>`;
* Using self-closing sytnax on childless SVG elements;
* Using the shorter of `'` and `"` to quote value attributes;

But at the end of the day, most savings come down to basic whitespace manipulation.

HTMinL parses HTML documents the same way web browsers do, and employs a ~~naive~~ conservative version of the same inline whitespace-collapsing strategies [they themselves use](https://developer.mozilla.org/en-US/docs/Web/CSS/CSS_text/Whitespace).

Unlike some of the more aggressive minifiers, HTMinL does not assume strict adherence to layout/content and inline/block distinctions. This may leave a few extra bytes on the table, but it greatly decreases the risk of accidental render fuckery.

And besides, any difference will be _negligible_ after proper [content encoding](https://github.com/Blobfolio/channelz/) anyway!

No sense going overboard. ;)



## Cautions

While care has been taken to balance savings and safety, there are some (intentional) limitations to be aware of:

* Documents are expected to be encoded in UTF-8;
* Documents are processed as **HTML**, _not_ XML, XHTML, liquid, markdown, PHP, etc.;
* HTMinL's parsing is pretty forgiving, but doesn't officially recognize "quirks mode";
* Whitespace collapsing _can_ adversely affect layouts when CSS properties like `white-space: pre` are applied to elements that don't normally have them;
