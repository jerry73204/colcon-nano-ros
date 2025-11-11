fn main() {
    // Print the path being embedded for debugging
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let rosidl_runtime_rs = std::path::Path::new(&manifest_dir).join("../rosidl-runtime-rs");

    eprintln!(
        "Embedding rosidl-runtime-rs from: {}",
        rosidl_runtime_rs.display()
    );
    eprintln!("Directory exists: {}", rosidl_runtime_rs.exists());

    if rosidl_runtime_rs.exists() {
        eprintln!("Contents:");
        for entry in std::fs::read_dir(&rosidl_runtime_rs).unwrap() {
            let entry = entry.unwrap();
            eprintln!("  {:?}", entry.path());
        }
    }

    // Tell Cargo to re-run if rosidl-runtime-rs changes
    println!("cargo:rerun-if-changed=../rosidl-runtime-rs");
}
