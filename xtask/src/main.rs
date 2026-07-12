fn main() -> nih_plug_xtask::Result<()> {
    let mut args = std::env::args().skip(1);
    let Some(command) = args.next() else {
        anyhow::bail!("Usage: cargo xtask bundle <package> [cargo build args]");
    };

    match command.as_str() {
        "bundle" => {
            let package = args.next().ok_or_else(|| {
                anyhow::anyhow!("Usage: cargo xtask bundle <package> [cargo build args]")
            })?;
            let cargo_args = args.collect::<Vec<_>>();
            let packages = vec![package.clone()];
            let target_dir = std::env::current_dir()?.join("target");

            nih_plug_xtask::build(&packages, &cargo_args)?;
            nih_plug_xtask::bundle(&target_dir, &package, &cargo_args, false)
        }
        _ => anyhow::bail!(
            "Unknown command '{command}'. Usage: cargo xtask bundle <package> [cargo build args]"
        ),
    }
}
