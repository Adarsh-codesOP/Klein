# Contributing to Klein IDE

First off, thank you for considering contributing to Klein! It's people like you that make learning and open source such a great community.

## Getting Started

1. **Fork the repository** on GitHub.
2. **Clone your fork** locally:
   ```bash
   git clone https://github.com/your-username/klein.git
   cd klein
   ```
3. **Set up upstream** to keep your local repo synchronized with the main project:
   ```bash
   git remote add upstream https://github.com/original-owner/klein.git
   ```

## Development Workflow

1. **Create a branch** for your specific feature or fix:
   ```bash
   git checkout -b feature/your-feature-name
   ```
2. **Make your changes**. Ensure you adhere to standard Rust formatting guidelines (`rustfmt`) and check for code standard consistency with `clippy`:
   ```bash
   cargo fmt
   cargo clippy
   ```
3. **Commit your changes**. Write clear, concise commit messages explaining *why* the change was made, not just *what* changed.
   ```bash
   git commit -m "Add new syntax highlighting feature"
   ```
4. **Push to your fork**:
   ```bash
   git push origin feature/your-feature-name
   ```
5. **Open a Pull Request** via GitHub. Provide a descriptive title, link relevant open issues (e.g., "Fixes #12"), and detail the problem solved and the implementation choices made.

## Architecture Overview

Klein uses `ratatui` as its core library for managing terminal layouts and rendering widgets.
If you're looking to modify the fundamental layout, check `src/ui/layout.rs`.
Event handling logic and keystroke combinations are heavily condensed inside `src/events/mod.rs`.
Application state, tabs tracking, and focus management lie inside `src/app.rs` and `src/editor.rs`. 

## Reporting Bugs

Please ensure the bug was not already reported by searching on GitHub under Issues. If you're unable to find an open issue addressing the problem, open a new one. Be sure to include a **title and clear description**, as much relevant information as possible, and a **code sample** or an **executable test case** demonstrating the expected behavior that is not occurring.

Thank you for contributing!
