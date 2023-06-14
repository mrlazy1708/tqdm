use version_check as rustc;

fn main() {
    if rustc::is_min_version("1.70.0").unwrap_or(false) {
        println!("cargo:rustc-cfg=std_once_cell")
    } else {
        println!("cargo:rustc-cfg=extern_once_cell")
    }
}
