fn main() {
    // Scan the Rust sources for Tailwind classes and generate the stylesheet.
    topcoat::tailwind::BuildConfig::new().render().unwrap();
}
