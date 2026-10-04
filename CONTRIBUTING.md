# Contributing

Run the demo and the checks listed in the README before submitting a pull request.

Keep mouse and keyboard behavior equivalent. Rendering and hit-testing must share the same layout, including horizontal scrolling, column resizing, and terminal resize. Add an interaction regression test for behavior changes.

Keep ordering and record selection in `GridModel`; keep terminal-specific input and rendering in `Grid`. Avoid making the component own terminal setup, application focus, or an event loop.

For a feature request, describe a concrete application and the interaction it needs. For bugs, include terminal, operating system, Rust version, and reproduction steps. Do not include private data.

Contributions are licensed under the project's MIT license.
