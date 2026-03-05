<h1 align="center">
  <br>
  <!-- <img src="assets/logo.png" alt="Klein IDE" width="200"> -->
  Klein IDE
  <br>
</h1>

<h4 align="center">A sleek, terminal-based Integrated Development Environment written in Rust.</h4>

<p align="center">
  <a href="https://rustup.rs/">
    <img src="https://img.shields.io/badge/Rust-1.75+-orange.svg" alt="Rust Version">
  </a>
  <a href="https://github.com/your-username/klein/blob/main/LICENSE">
    <img src="https://img.shields.io/badge/License-MIT-blue.svg" alt="License">
  </a>
  <a href="#features">
    <img src="https://img.shields.io/badge/Features-Rich-success.svg" alt="Features">
  </a>
</p>

<p align="center">
  <a href="#-overview">Overview</a> •
  <a href="#-key-features">Features</a> •
  <a href="#-installation">Installation</a> •
  <a href="#-keybindings--shortcuts">Shortcuts</a> •
  <a href="#%EF%B8%8F-configuration">Configuration</a> •
  <a href="#-contributing">Contributing</a>
</p>

---

## 🌟 Overview

**Klein** is a blazingly fast, terminal-based IDE built strictly with Rust. Leveraging the power of `ratatui` for its aesthetic user interface and `portable-pty` for robust terminal integration, Klein aims to provide developers with a seamless, keyboard-centric coding experience directly in their command line. 

Whether you're editing code across multiple tabs, exploring your project directory, or compiling your app via the integrated bash terminal, Klein houses all your workflow needs into a single, beautifully rendered console window.

![Klein IDE Screenshot](https://via.placeholder.com/1000x500.png?text=Klein+IDE+-+Your+Terminal+Workflow+Reimagined)
*(Note: Replace the placeholder image above with an actual screenshot or GIF of the IDE in action)*

---

## ✨ Key Features

Klein doesn't compromise on functionality or looks. Here is what you get out of the box:

- 🗂️ **Multi-Tab Editing:** Seamlessly open and manage multiple files using intuitive tab navigation.
- 💻 **Integrated Terminal:** A fully-featured embedded terminal to execute local scripts, manage version control (Git), and run build commands without ever leaving the editor window.
- 🌳 **File Explorer Sidebar:** Effortless project navigation with a collapsible, visual directory tree.
- 📝 **Rich Text Editing:** 
  - **Syntax Highlighting:** Powered by `syntect` for beautiful and accurate code coloring across various languages.
  - **Advanced Selection:** Support for multiline selections and bulk text manipulation.
  - **System Clipboard Integration:** Native copy/paste and selection operations matching standard editor workflows.
- 🎨 **Sleek UI & Theming:** Informative status bars, elegant double borders, line numbers, and easily customizable color schemes.

---

## 🚀 Installation

To get started with Klein, ensure you have [Rust and Cargo](https://rustup.rs/) installed on your system. 

> **Note for Windows Users:** The embedded terminal relies on Git Bash by default (`C:\Program Files\Git\bin\bash.exe`). Ensure it is installed on your machine, or adapt the path in the configuration.

1. **Clone the repository:**
   ```bash
   git clone https://github.com/your-username/klein.git
   cd klein
   ```

2. **Build and Run:**
   Fetch the dependencies and launch the IDE. Compiling in release mode is recommended for optimal UI performance.
   ```bash
   cargo run --release
   ```

---

## ⌨️ Keybindings & Shortcuts

Klein is designed to be fully navigable via keyboard, maximizing your productivity while keeping your hands on the home row.

### Navigation & Focus
| Shortcut | Action |
| :--- | :--- |
| <kbd>Ctrl</kbd> + <kbd>←/→</kbd> | Switch focus cyclically between Sidebar, Editor, and Terminal |
| <kbd>Ctrl</kbd> + <kbd>R</kbd> | Focus Explorer (Sidebar) |
| <kbd>Ctrl</kbd> + <kbd>E</kbd> | Focus Editor |
| <kbd>Ctrl</kbd> + <kbd>T</kbd> | Focus Terminal |
| <kbd>Ctrl</kbd> + <kbd>B</kbd> | Toggle Sidebar Visibility |
| <kbd>Ctrl</kbd> + <kbd>\`</kbd> | Toggle Terminal Visibility |

### Tab Management
| Shortcut | Action |
| :--- | :--- |
| <kbd>Ctrl</kbd> + <kbd>Shift</kbd> + <kbd>Z</kbd> | Navigate to the Next Tab |
| <kbd>Ctrl</kbd> + <kbd>Shift</kbd> + <kbd>X</kbd> | Close the Current Tab |

### Editor Actions
| Shortcut | Action |
| :--- | :--- |
| <kbd>Ctrl</kbd> + <kbd>S</kbd> | Save Current File |
| <kbd>Ctrl</kbd> + <kbd>A</kbd> | Select All text in the active buffer |
| <kbd>Ctrl</kbd> + <kbd>C</kbd> | Copy the current selection |
| <kbd>Ctrl</kbd> + <kbd>V</kbd> | Paste from system clipboard |
| <kbd>Ctrl</kbd> + <kbd>Shift</kbd> + <kbd>↑/↓</kbd>| Expand Multiline Selection |
| <kbd>Enter</kbd> | Expand Folder (Sidebar) / Insert New Line (Editor) |

### System Commands
| Shortcut | Action |
| :--- | :--- |
| <kbd>Ctrl</kbd> + <kbd>H</kbd> | Toggle display of the Help Command Overlay |
| <kbd>Ctrl</kbd> + <kbd>Q</kbd> | Quit the Application (safeguarded against unsaved changes) |

> 💡 **Tip:** Don't remember a shortcut? Just press <kbd>Ctrl</kbd> + <kbd>H</kbd> inside the IDE to summon the handy help menu!

---

## ⚙️ Configuration

Make Klein truly yours. Core configurations, including default shell paths, UI colors, and text banners, can be easily modified.

Navigate to `src/config.rs` to adjust the IDE's environment:
```rust
// Example: Altering default focus colors in src/config.rs
pub mod colors {
    use ratatui::style::Color;
    pub const EXPLORER_FOCUS: Color = Color::Green;
    pub const EDITOR_FOCUS: Color = Color::Yellow;
    pub const TERMINAL_FOCUS: Color = Color::Cyan;
    // ...
}
```
*Note: Any changes made to `src/config.rs` require a recompilation `cargo run` to take effect.*

---

## 🤝 Contributing

We welcome contributions of all shapes and sizes! Whether you are squashing a bug, adding a new feature, or simply fixing a typo, your help is greatly appreciated.

Please read our [Contributing Guidelines](CONTRIBUTING.md) to understand the workflow for submitting pull requests, setting up the development environment, and reporting issues. Let's build a better terminal IDE together!

---

## 📜 License

Klein IDE is open-sourced software licensed under the [MIT license](LICENSE).
