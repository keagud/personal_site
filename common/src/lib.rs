#[macro_export]
macro_rules! asset {
    ($s:literal) => {
        concat!(std::env!("CARGO_MANIFEST_DIR"), "/..", "/assets/", $s)
    };
}
