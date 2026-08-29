fn main() {
    // Rebuild when embedded frontend assets change.
    println!("cargo:rerun-if-changed=assets");
}
