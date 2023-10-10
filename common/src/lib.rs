#[macro_export]
macro_rules! assets_dir {
    () => {
        std::env!("ASSETS_DIR")
    };
}

#[macro_export]
macro_rules! asset {
    ($s:literal) => {
        concat!(std::env!("ASSETS_DIR"), $s)
    };
}
