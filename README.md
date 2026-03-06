<h1 align="center">
  <br>
  <img src="src/Assets/logo.png" alt="Klein" width="200">
  <br>
  Klein — A TIDE (Terminal Integrated Development Environment)
  <br>
</h1>

<h4 align="center">A professional terminal-based text editor with IDE‑like features, written in Rust.</h4>

<p align="center">
  <a href="https://rustup.rs/">
    <img src="https://img.shields.io/badge/Rust-1.75+-orange.svg" alt="Rust Version">
  </a>
  <a href="https://github.com/Adarsh-CodesOP/klein/blob/main/LICENSE">
  <img src="https://img.shields.io/badge/License-Apache%202.0-blue.svg">
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

**Klein** is a lightweight, terminal-based text editor built in Rust. It provides an IDE-like interface using `ratatui` for the user interface and `portable-pty` for terminal integration, giving developers a keyboard‑centric coding environment directly in the command line.

Whether you're editing multiple files, browsing a project directory, or compiling code from the embedded shell, Klein brings essential workflow tools into one efficient console application.

![Klein TIDE Screenshot](src/Assets/logo.png)


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

## Installation

### Automatic Setup (Recommended for Windows)

A pair of installation scripts will configure your workspace, set a preferred shell, download the latest pre‑built binary, and present a polished, colorful console interface during setup.

**Using PowerShell (Windows):**
```powershell
irm https://raw.githubusercontent.com/Adarsh-CodesOP/Klein/main/install.ps1 | iex
```
*(Note: If you already have the repository cloned, you can just run `.\install.ps1`)*

**Using Bash (Git Bash, WSL or Linux):**
```bash
curl -sSL https://raw.githubusercontent.com/Adarsh-CodesOP/Klein/main/install.sh | bash
```

**Reconfiguring:**
To adjust your workspace or shell after installation, run:
```powershell
.\install.ps1 -Reconfigure
```
*(Or `./install.sh --reconfigure`)*

### Manual Build and Install

If you prefer to build it manually from source:

1. Clone the repository:
   ```bash
   git clone https://github.com/adarsh-codesOP/Klein.git
   cd Klein
   ```
2. Build and install:
   ```bash
   cargo install --path .
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

> 💡 **Tip:** Don't remember a shortcut? Just press <kbd>Ctrl</kbd> + <kbd>H</kbd> inside Klein to summon the handy help menu!

---

## ⚙️ Configuration

Make Klein truly yours. Core configurations, including default shell paths, UI colors, and text banners, can be easily modified.

Navigate to `src/config.rs` to adjust the TIDE's environment:
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

Please read our [Contributing Guidelines](CONTRIBUTING.md) to understand the workflow for submitting pull requests, setting up the development environment, and reporting issues. Let's build a better terminal text editor together!

---

## 📜 License

Klein TIDE is open‑sourced software licensed under the [MIT license](LICENSE).
