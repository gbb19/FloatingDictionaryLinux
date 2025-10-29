# Floating Dictionary (Linux)

A fast, simple, and modern screen-capture OCR and translation tool for Linux desktops.

This application intelligently adapts to your desktop environment, using KDE Spectacle for screen captures on Plasma and the Freedesktop portal for GNOME and other environments.

*(A screenshot or GIF demonstrating the app in action would go here.)*

## Features

- **Instant OCR Capture**: Select any region on your screen to instantly recognize the text within it.
- **Multi-Language OCR**: Supports a wide range of languages for text recognition, including:
  - English (`eng`)
  - Russian (`rus`)
  - Japanese (`jpn`)
  - Korean (`kor`)
  - Simplified Chinese (`chi_sim`)
  - Thai (`tha`)
- **Auto OCR Mode**: Automatically uses all supported OCR languages *except* for your specified target translation language, maximizing recognition accuracy.
- **Reliable Translation**: Uses Google Translate for fast and accurate translations with automatic source language detection.
- **Detailed English Definitions**: When translating a single English word to Thai, it provides detailed definitions and example sentences from the Longdo Dictionary.
- **Modern UI**: A clean, transparent, and auto-sizing window that gets out of your way. It automatically closes when it loses focus.
- **Portable**: Tesseract's language data files are bundled with the application, so you don't need to install them separately.

## Prerequisites

Before using the application, you must have the following screenshot tools and backend libraries installed on your system.

1.  **Tesseract OCR Engine**: This is the core library used for text recognition.

2.  **Screenshot Utility**: The application automatically detects your desktop environment and uses the appropriate tool:
    *   **For KDE Plasma**: `spectacle` is required. It is usually pre-installed with the desktop environment.
    *   **For GNOME and others**: The `xdg-desktop-portal` infrastructure is used. This is standard on most modern non-KDE Linux desktops.

**Installation Instructions for Core Dependencies:**

-   **Fedora / Red Hat:**
    ```sh
    sudo dnf install tesseract spectacle
    ```
-   **Debian / Ubuntu:**
    ```sh
    sudo apt-get update
    sudo apt-get install tesseract-ocr spectacle
    ```
-   **Arch Linux:**
    ```sh
    sudo pacman -S tesseract spectacle
    ```

## Installation

1.  Go to the [**Releases**](https://github.com/your-username/your-repo/releases) page.
2.  Download the latest binary file for your system.
3.  Make the file executable:
    ```sh
    chmod +x floating-dictionary-linux
    ```
4.  (Optional) Move the binary to a directory in your system's PATH, like `~/.local/bin/` or `/usr/local/bin/`, for easy access from anywhere.

## Usage

Run the application from your terminal. The UI will appear, allowing you to select a screen region. Once the OCR and translation are complete, the results will be displayed.

### Command-Line Arguments

-   `--ocr-lang <LANGUAGE>`
    -   Specifies the language for Tesseract to use for OCR.
    -   **Default**: `auto`
    -   **Available values**: `auto`, `eng`, `rus`, `jpn`, `kor`, `chi_sim`, `thai`.
    -   In `auto` mode, the application uses all available languages for recognition *except* for the one specified as the target language, which prevents redundant processing.

-   `-t, --target <LANGUAGE_CODE>`
    -   The language you want to translate the text into.
    -   **Default**: `th`
    -   Uses standard language codes (e.g., `en` for English, `th` for Thai, `ja` for Japanese).

### Examples

-   **Default behavior (Auto OCR, translate to Thai)**:
    ```sh
    ./floating-dictionary-linux
    ```

-   **Recognize Japanese text and translate it to English**:
    ```sh
    ./floating-dictionary-linux --ocr-lang jpn --target en
    ```

-   **Recognize text of an unknown language and translate to Russian**:
    ```sh
    ./floating-dictionary-linux --target ru
    ```
    *(This will use `auto` OCR mode, which is `eng+jpn+kor+chi_sim+tha`)*

## Building from Source

If you prefer to build the application yourself, follow these steps:

1.  **Clone the repository**:
    ```sh
    git clone https://github.com/your-username/your-repo.git
    cd your-repo
    ```

2.  **Install Rust**:
    If you don't have Rust installed, get it from [rustup.rs](https://rustup.rs/).

3.  **Install System Dependencies**:
    You'll need the development packages for Tesseract and other libraries.
    -   **Fedora / Red Hat:**
        ```sh
        sudo dnf install tesseract-devel clang
        ```
    -   **Debian / Ubuntu:**
        ```sh
        sudo apt-get install tesseract-ocr libtesseract-dev clang
        ```

4.  **Build the application**:
    Run the build command in release mode.
    ```sh
    cargo build --release
    ```

The final binary will be located at `target/release/floating-dictionary-linux`.

## License

This project is licensed under the MIT License. See the `LICENSE` file for details.