fn main() {
    println!("cargo:rerun-if-changed=app.rc");
    println!("cargo:rerun-if-changed=icon.ico");
    println!("cargo:rerun-if-changed=../ui/index.html");
    embed_resource::compile("app.rc", embed_resource::NONE).manifest_optional().unwrap();
}
