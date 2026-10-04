//! The grid, as the two files a page needs.
//!
//! Put an element on the page and load the script:
//!
//! ```html
//! <link rel="stylesheet" href="…/grid.css">
//! <div id="fugantt-grid" data-project="abc" data-api="/somewhere/abc"></div>
//! <script src="…/grid.js" defer></script>
//! ```
//!
//! The script draws into the element and talks JSON to the addresses under
//! `data-api`. What those addresses take and give back is in
//! `docs/gantt-api.md`, and as types in `fu-gantt-core`.
//!
//! Nothing here serves a request. How a file reaches a browser is the host's
//! own business, and every framework has its own way; this crate is the bytes
//! and a name for them that changes when they do.
//!
//! The files are built by `npm run build` in `web/` and committed, so that
//! depending on this crate never asks for Node.

use std::sync::LazyLock;

/// The grid island.
pub const GRID_JS: &str = include_str!("../web/dist/grid.js");

/// Its stylesheet. Complete on its own, in a light palette; a host that has
/// colours of its own sets the `--fg-*` properties on `.fg-grid`.
pub const GRID_CSS: &str = include_str!("../web/dist/grid.css");

/// A short digest of the contents, so a changed file gets a changed URL.
///
/// FNV-1a rather than a cryptographic hash: nothing here is a secret, and the
/// only job is to differ when the bytes differ.
pub fn fingerprint(body: &str) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;

    for byte in body.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }

    format!("{hash:016x}")
}

/// [`fingerprint`] of [`GRID_JS`], worked out once.
pub fn grid_js_hash() -> &'static str {
    static HASH: LazyLock<String> = LazyLock::new(|| fingerprint(GRID_JS));
    &HASH
}

/// [`fingerprint`] of [`GRID_CSS`], worked out once.
pub fn grid_css_hash() -> &'static str {
    static HASH: LazyLock<String> = LazyLock::new(|| fingerprint(GRID_CSS));
    &HASH
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_files_made_it_in() {
        assert!(
            GRID_JS.contains("fugantt-grid"),
            "グリッドの JS が入っていない"
        );
        assert!(
            GRID_CSS.contains(".fg-grid"),
            "グリッドの CSS が入っていない"
        );
    }

    /// A changed file must not keep the old URL, or browsers will hold the old
    /// bytes for a year.
    #[test]
    fn the_digest_moves_with_the_contents() {
        assert_ne!(fingerprint("a"), fingerprint("b"));
        assert_eq!(fingerprint("a"), fingerprint("a"));
        assert_eq!(grid_js_hash().len(), 16);
        assert_eq!(grid_js_hash(), fingerprint(GRID_JS));
        assert_ne!(grid_js_hash(), grid_css_hash());
    }

    /// The digest fugantt put in its URLs before this crate existed. If this
    /// moves, every installation's cached files are orphaned for no reason.
    #[test]
    fn the_digest_is_the_one_it_always_was() {
        assert_eq!(fingerprint(""), "cbf29ce484222325");
        assert_eq!(fingerprint("a"), "af63dc4c8601ec8c");
    }
}
