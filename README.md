# widgem

**widgem is a work in progress. It's not ready for use.**

widgem is a lightweight cross-platform desktop widget toolkit. It aims to provide standard UI elements for desktop applications (buttons, text inputs, menus, etc.) with consistent look and feel across Linux, Windows, and macOS.

## Main goals

- Fully featured, predictable behavior: we aim to support all features that are expected from widgets in a native desktop application. This includes:
    - Supporting keyboard-only widget interaction (proper focus handling, navigation, and usage of all widgets).
    - Supporting all standard keyboard shortcuts.
- Accessibility: out-of-the-box integration with screen readers and similar tooling, supporting high-contract themes.
- Theme customization: use a subset of CSS to create a new widget theme for all widgets or customize look of individual widgets.
- Customizable widgets: built-in widgets provide a wide range of properties to augment their behavior.
- Easy extensibility: combine existing widgets into new reusable widgets or create completely new widgets with custom look and behavior.
- Minimal abstraction: we try to keep the codebase as simple as possible, without trying to support all possible use cases.

## Explicit non-goals

- Web applications.
- Animations.
- GPU-accelerated rendering.
- Declarative UI.

## Running snapshot tests

You can run snapshot tests in Docker to ensure a controlled environment:
```
./tests/scripts/run_tests_in_docker.sh
```

It's also possible to run the tests directly on the host system (Linux, Windows, and MacOS are supported):
```
cargo run --locked --bin widgem_tests -- test
```
However, in this case, additional requirements must be met:

* The display's resolution must be large enough to fit the test windows (1600x1200 or 1920x1080 will be enough).
* On MacOS:
    * The display scale must be 1 (i.e. it must not use a HiDPI resolution).
    * The display's color profile must be set to sRGB.
    * The test process must be granted the necessary permissions.
