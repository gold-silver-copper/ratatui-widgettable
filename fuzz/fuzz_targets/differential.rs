//! Renders random text-only tables using ratatui-widgettable and ratatui's `Table`, and checks that
//! the output and resulting state are identical, including with a random scroll padding.
//!
//! The comparison is against the commit on ratatui's main branch that ratatui-widgettable was
//! ported from. The released `ratatui-widgets` 0.3.2 is not compared, as main includes a fix for
//! selected rows that are taller than the table (0.3.2 scrolls past them, so they aren't shown).
//!
//! Inputs where the header and footer don't fit are skipped, see
//! [`TableSpec::header_and_footer_fit`].
#![no_main]

use libfuzzer_sys::fuzz_target;
use ratatui_widgettable_fuzz::{TableSpec, assert_same, catch_panic, render_table};

fuzz_target!(|input: (TableSpec, usize, bool)| {
    let (spec, scroll_padding, small_numbers) = input;
    let mut spec = spec.bounded().stable_layout();
    let mut scroll_padding = scroll_padding;
    if small_numbers {
        spec = spec.small_numbers();
        scroll_padding %= 8;
    }

    if !spec.header_and_footer_fit() {
        return;
    }

    let main = catch_panic(|| render_table!(main, &spec, |t| t.scroll_padding(scroll_padding)));
    let ours = catch_panic(|| render_table!(ours, &spec, |t| t.scroll_padding(scroll_padding)));
    assert_same("ratatui main", &spec, &main, &ours);
});
