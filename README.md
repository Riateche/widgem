# salvation

**salvation is a work in progress. It's not ready for use.**

salvation is a lightweight cross-platform desktop widget toolkit. It aims to provide standard UI elements for desktop applications (buttons, text inputs, menus, etc.) with consistent look and feel across Linux, Windows, and macOS.

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
