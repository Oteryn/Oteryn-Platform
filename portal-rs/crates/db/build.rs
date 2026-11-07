fn main() {
    // sqlx's embedded migration macro must rerun when new migration files appear.
    println!("cargo:rerun-if-changed=../../migrations");
}
