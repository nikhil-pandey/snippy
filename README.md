# snippy

## Overview

## Features

## Installation

To install `snippy`, you need to have [Rust](https://www.rust-lang.org/tools/install) installed. Then you can install it using `cargo`:

```sh
cargo install .....
```

## Usage

```sh
snippy [OPTIONS] [FILES]...
```

### Examples

1. **Copy files to clipboard with default settings:**

    ```sh
    snippy file1.rs file2.py
    ```

2. **Copy files without markdown ticks:**

    ```sh
    snippy --no-markdown file1.rs file2.py
    ```

3. **Add line numbers with a custom prefix:**

    ```sh
    snippy --line-number 3 --prefix ">> " file1.rs file2.py
    ```

4. **Watch clipboard for changes and process new content:**

    ```sh
    snippy -w
    ```