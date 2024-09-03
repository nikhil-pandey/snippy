# 📎 Snippy

## 🌟 Overview

Snippy is a command-line tool for copying code snippets to the clipboard with optional formatting features. It's designed to make sharing code easier and more efficient, with special features for working with Large Language Models. ✨Here's a quick demo of Snippy in action:

![Snippy Demo](assets/demo.mp4)

## 🚀 Features

-   📋 Copy multiple files to clipboard
-   🖌️ Optional markdown code block formatting
-   🔢 Customizable line numbering
-   👀 Clipboard watching mode for real-time processing
-   🤖 Seamless integration with LLM outputs
-   🖥️ Cross-platform support (Windows, macOS, Linux)

## 💻 Installation

To install `snippy`, you need to have [Rust](https://www.rust-lang.org/tools/install) installed. Then you can install it using `cargo`:

```sh
cargo install snippy
```

## 🛠️ Usage

```sh
snippy [OPTIONS] [FILES]...
```

### 📚 Examples

1. **Copy files to clipboard with default settings:**

    ```sh
    snippy copy file1.rs file2.py
    ```

2. **Copy files without markdown ticks:**

    ```sh
    snippy copy --no-markdown file1.rs file2.py
    ```

3. **Add line numbers with a custom prefix:**

    ```sh
    snippy copy --line-number 3 --prefix ">> " file1.rs file2.py
    ```

4. **Watch clipboard for changes and process new content:**

    ```sh
    snippy watch
    ```

### 🔍 Clipboard Watching for LLM Integration

The watch command (`watch`) is particularly powerful when working with LLMs:

```sh
snippy watch
```

This mode continuously monitors your clipboard. When you copy output from an LLM, snippy automatically processes and applies the code to your workspace. 🪄

To use this feature effectively:

1. Use prompts that generate code in a format Snippy can recognize.
2. Check the `prompts/` directory for example prompts that work well with Snippy.
3. Ensure your LLM output includes clear file paths and properly formatted code blocks.

Example of a Snippy-friendly code format:

````
### src/main.rs
```rust
fn main() {
    println!("Hello, Snippy!");
}
```
````

## 📜 License

This project is licensed under the GNU General Public License v3.0 - see the [LICENSE](LICENSE) file for details.

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request. 👍

## 💬 Support

If you encounter any problems or have any questions, please open an issue on the GitHub repository. We're here to help! 🆘

---

Happy coding with Snippy! 🎉
