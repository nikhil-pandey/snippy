use tracing::info;

pub fn print_diff(file: &str, old: &str, new: &str) {
    let patch = diffy::create_patch(old, new);
    let f = diffy::PatchFormatter::new().with_color();
    info!("Diff for file: {}\n{}", file, f.fmt_patch(&patch));
}
