# HTMinL

[![ci](https://img.shields.io/github/actions/workflow/status/Blobfolio/htminl/ci.yaml?style=flat-square&label=ci)](https://github.com/Blobfolio/htminl/actions)
[![deps.rs](https://deps.rs/repo/github/blobfolio/htminl/status.svg?style=flat-square&label=deps.rs)](https://deps.rs/repo/github/blobfolio/htminl)<br>
[![license](https://img.shields.io/badge/license-wtfpl-ff1493?style=flat-square)](https://en.wikipedia.org/wiki/WTFPL)
[![contributions welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg?style=flat-square&label=contributions)](https://github.com/Blobfolio/htminl/issues)

HTMinL is a CLI tool for x86-64 Linux machines that simplifies the task of minifying HTML in-place for production environments.



## Features

HTMinL is a fast, in-place HTML minifier. It prioritizes safety and code sanity over _ULTIMATE COMPRESSION_, so may not save quite as many bytes as Node's venerable [html-minifier](https://github.com/kangax/html-minifier), but it is also much less likely to break shit.

And it runs _magnitudes_ faster…

Unlike virtually every other minifier in the wild, HTMinL is _not_ a stream processor; it constructs a complete DOM tree from the full source _before_ getting down to business. This allows for much more accurate processing and robust error recovery.

See the [minification](#minification) section for more details about the process, as well as the [cautions](#cautions) section for important assumptions, requirements, gotchas, etc.



## Installation

Debian and Ubuntu users can just grab the pre-built `.deb` package from the [latest release](https://github.com/Blobfolio/htminl/releases/latest).

This application is written in [Rust](https://www.rust-lang.org/) and can alternatively be built from source using [Cargo](https://github.com/rust-lang/cargo):

```bash
# Clone the source.
git clone https://github.com/Blobfolio/htminl.git

# Go to it.
cd htminl

# Build as usual. Specify additional flags as desired.
cargo build \
    --bin htminl \
    --release
```

(This should work under other 64-bit Unix environments too, like MacOS.)



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

Minification is primarily achieved through (conservative) whitespace manipulation — trimming, collapsing, or both — in text nodes, tags, and attribute values, but only when it is judged completely safe to do so.

For example, whitespace is _not_ altered in "value" attributes or inside elements like `<pre>` or `<textarea>`, where it generally matters.

Speaking of "generally matters", `HTMinL` does _not_ make any assumptions about the display type of elements, as *CSS is a Thing*. Just because a `<div>` is normally block doesn't mean someone hasn't styled one to render inline. This often leaves some whitespace around tags, but helps ensure styled layouts display correctly.

Additional savings are achieved by stripping:

 * HTML Comments;
 * XML processing instructions;
 * Child text nodes of `<html>` and `<head>` elements (they don't belong there!);
 * Leading and trailing whitespace directly in the `<body>`;
 * Whitespace in inline CSS is collapsed and trimmed (but otherwise unaltered);
 * Whitespace sandwhiched between non-renderable elements like `<script>` or `<style>` tags;
 * Default `type` attributes on `<script>` and `<style>` elements;
 * Pointless attributes (like an empty "id" or "alt" or a falsey boolean like `hidden="false"`);
 * Empty or implied attribute values;
 * Leading and trailing whitespace in non-value attributes;

The above list is non-exhaustive, but hopefully you get the idea!

With the exception of CSS — which has its whitespace fully minified — inline foreign content like Javascript and JSON are passed through unchanged. This is one of the biggest "missed opportunities" for byte savings, but also where minifiers tend to accidentally break things. Better a few extra bytes than a broken page!



## Cautions

While care has been taken to balance savings and safety, there are a few design choices that could potentially break documents, worth noting before you use it on your project:

 * Documents are expected to be encoded in UTF-8. Other encodings might be OK, but some text could get garbled.
 * Documents are processed as *HTML*, not XML or XHTML. Inline SVG elements should be fine, but it may well corrupt other XML-ish data.
 * Child text nodes of `<html>` and `<head>` elements are removed. Text doesn't belong there anyway, but HTML is awfully forgiving; who knows what kinds of markup will be found in the wild!
 * CSS whitespace is trimmed and collapsed, which could break (very unlikely!) selectors like `input[value="Spa  ced"]`.
 * Element tags are normalized, which can break fussy `camelCaseCustomElements`. (Best to write tags like `my-custom-tag` anyway...)



## Benchmarks

These benchmarks were performed on a Intel® Core™ i7-10610U with four discrete cores, averaging 100 runs. To best approximate feature parity, [html-minifier](https://github.com/kangax/html-minifier) was run with the following flags:

    --collapse-boolean-attributes
    --collapse-whitespace
    --decode-entities
    --remove-attribute-quotes
    --remove-comments
    --remove-empty-attributes
    --remove-optional-tags
    --remove-optional-tags
    --remove-redundant-attributes
    --remove-redundant-attributes
    --remove-script-type-attributes
    --remove-style-link-type-attributes

#### Bench: HTMinL Documentation

    Files: 270/322
    Size:  1,253,110 bytes (HTML)

| Program | Time (s) | Minified (b) |
| ---- | ---- | ---- |
| HTMinL | **0.0262** | 1,141,615 |
| html-minifier | 30.7296 | **1,138,712** |

#### Bench: VueJS.org

    Files: 146/321
    Size:  3,999,552 bytes (HTML)

| Program | Time (s) | Minified (b) |
| ---- | ---- | ---- |
| HTMinL | **0.0494** | 3,461,501 |
| html-minifier | 43.9917 | **3,331,880** |

**TL/DR;** With these sources, anyway, `html-minifier` eeks out 1–4% extra savings, but HTMinL is _hundreds of times faster_. 

It is important to note that `html-minifier` is _not_ designed for this particular use case — recursive in-place HTML minification with random non-HTML assets sprinkled about — which goes a long way toward explaining the gross difference in runtime cost.

#### Bench: Single

But averaging the runtimes of processing each HTML file individually, HTMinL still runs forty times faster:

| Program | Time (ms) |
| ---- | ---- |
| HTMinL | **3** |
| html-minifier | 122 |

Still, not too shabby!
