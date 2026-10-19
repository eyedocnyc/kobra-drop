// Developer validation helper. End users use the Windows desktop application.
fn main() -> anyhow::Result<()> {
    let args = std::env::args_os().skip(1).collect::<Vec<_>>();
    anyhow::ensure!(
        args.len() == 2,
        "Usage: convert <project.3mf> <installed-profiles-directory>"
    );
    let result = kobra_converter::convert(
        std::path::Path::new(&args[0]),
        Some(std::path::Path::new(&args[1])),
        |s| eprintln!("{s}"),
    )?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}
