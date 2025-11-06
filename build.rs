fn main() {
    if std::env::var_os("CARGO_FEATURE_LEGACY_FFI").is_some() {
        match pkg_config::Config::new()
            .atleast_version("46")
            .probe("gnome-software")
        {
            Ok(library) => {
                for link_path in library.link_paths {
                    println!(
                        "cargo:rustc-link-arg=-Wl,-rpath,{}",
                        link_path.display()
                    );
                }
            }
            Err(err) => {
                panic!(
                    "Failed to locate gnome-software development files via pkg-config: {err}"
                );
            }
        }
    }
}
