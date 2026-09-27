# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - Unreleased

### Added

- `Table`, `Row` and `Cell` widgets, ported from the `Table` widget in `ratatui-widgets`, where
  each `Cell` can contain any widget that implements `Widget` for a reference to itself.
- `Cell::from_fn` for rendering a cell with a closure.
- `Table::scroll_padding`, from ratatui's main branch.
- Re-exports of `TableState` and `HighlightSpacing` from `ratatui-widgets`.
