# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed

These were found by fuzzing, and also exist in ratatui's `Table`:

- Panic from arithmetic overflow (in debug builds) with very tall rows or margins, or cells spanning
  columns with a very large column spacing.
- Cells spanning multiple columns, and highlight symbols wider than the table, were drawn past the
  right edge of the table.
- Cells in rows pushed below the table by their top margin could be drawn outside the table.

### Added

- Fuzz targets comparing the rendering with ratatui's `Table`, see `fuzz/README.md`.

## [0.1.0] - 2026-09-27

### Added

- `Table`, `Row` and `Cell` widgets, ported from the `Table` widget in `ratatui-widgets`, where
  each `Cell` can contain any widget that implements `Widget` for a reference to itself.
- `Cell::from_fn` for rendering a cell with a closure.
- `Table::scroll_padding`, from ratatui's main branch.
- Re-exports of `TableState` and `HighlightSpacing` from `ratatui-widgets`.
