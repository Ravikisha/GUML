//! The formatter, which by design runs on input that does not compile — that is its main caller.
//!
//! Two properties, both of which a repair loop depends on:
//!
//! 1. **Idempotence.** Format twice, get the same thing. A formatter that oscillates would make
//!    `fmt --check` fail forever on a file that is already formatted.
//! 2. **No panic**, in either mode.
#![no_main]

use guml_fmt::{Options, format};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(src) = std::str::from_utf8(data) else { return };

    for opts in [Options::default(), Options::canonical()] {
        let once = format(src, opts);
        let twice = format(&once.text, opts);
        assert_eq!(twice.text, once.text, "formatting is not idempotent");
    }
});
