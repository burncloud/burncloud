use anyhow::Result;
use burncloud_installer::{BundleCreator, BundleVerifier};
use clap::ArgMatches;
use std::path::PathBuf;

pub async fn handle_bundle_command(matches: &ArgMatches) -> Result<()> {
    match matches.subcommand() {
        Some(("create", sub_m)) => {
            let software = sub_m
                .get_one::<String>("software")
                .ok_or_else(|| anyhow::anyhow!("software argument is required"))?;

            let output_dir = sub_m
                .get_one::<String>("output")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("./bundles"));

            let creator = BundleCreator::new(output_dir);
            let bundle_dir = creator.create(software).await?;
            println!("Bundle created at: {}", bundle_dir.display());
        }
        Some(("verify", sub_m)) => {
            let bundle_path = sub_m
                .get_one::<String>("bundle")
                .ok_or_else(|| anyhow::anyhow!("bundle argument is required"))?;

            let bundle_dir = PathBuf::from(bundle_path);
            let manifest = BundleVerifier::verify(&bundle_dir)?;
            println!("Bundle verified: {} ({})", manifest.software.name, manifest.software.id);
        }
        _ => {
            println!("Unknown bundle subcommand. Use 'create' or 'verify'.");
        }
    }

    Ok(())
}
