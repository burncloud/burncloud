fn main() -> anyhow::Result<()> {
    // Same commands as `cargo run -- code ...`, without building the application.
    let matches = burncloud_code::command().get_matches();
    burncloud_code::handle(&matches)
}
