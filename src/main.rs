use std::fs;
use std::path::{Path, PathBuf};

use anyhow::anyhow;
use anyhow::Result;
use clap::Parser;
use rattler_conda_types::{ExplicitEnvironmentEntry, ExplicitEnvironmentSpec, Platform};
use rattler_lock::{CondaPackage, LockFile, Package};
use tracing_log::AsTrace;

fn cwd() -> PathBuf {
    std::env::current_dir().expect("failed to obtain current working directory")
}

#[derive(clap::Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Environment to render
    #[arg(short, long, default_value = "default")]
    environment: String,

    /// Platform to render
    #[arg(short, long, default_value = Platform::current().as_str())]
    platform: Platform,

    /// The path to 'pixi.toml' or 'pyproject.toml'
    #[arg(default_value = cwd().join("pixi.toml").into_os_string())]
    manifest_path: PathBuf,

    /// PyPI dependencies are not supported.
    /// This flag allows packing even if PyPI dependencies are present.
    #[arg(long, default_value = "false")]
    ignore_pypi_errors: bool,

    #[command(flatten)]
    verbose: clap_verbosity_flag::Verbosity,
}

fn build_explicit_spec<'a>(
    platform: Platform,
    conda_packages: impl IntoIterator<Item = &'a CondaPackage>,
) -> Result<ExplicitEnvironmentSpec> {
    let mut packages = Vec::new();

    for cp in conda_packages {
        let prec = cp.package_record();
        let mut url = cp.url().to_owned();
        let hash = prec.md5.ok_or(anyhow!(
            "Package {} does not contain an md5 hash",
            prec.name.as_normalized()
        ))?;

        url.set_fragment(Some(&format!("{:x}", hash)));

        packages.push(ExplicitEnvironmentEntry {
            url: url.to_owned(),
        });
    }

    Ok(ExplicitEnvironmentSpec {
        platform: Some(platform),
        packages,
    })
}

fn write_explicit_spec(
    target: impl AsRef<Path>,
    exp_env_spec: &ExplicitEnvironmentSpec,
) -> Result<()> {
    let platform = exp_env_spec
        .platform
        .ok_or(anyhow!("No platform specified in ExplicitEnvironmentSpec"))?
        .as_str();

    let mut environment = String::new();
    environment.push_str("# Generated by pixi :: pixi2ces\n");
    environment.push_str(&format!("# platform: {}\n", platform));
    environment.push_str("@EXPLICIT\n");

    for entry in exp_env_spec.packages.iter() {
        environment.push_str(&format!("{}\n", entry.url.as_str()));
    }

    fs::write(target, environment)
        .map_err(|e| anyhow!("Could not write environment file: {}", e))?;

    Ok(())
}

fn main() -> Result<()> {
    let options = Cli::parse();

    tracing_subscriber::FmtSubscriber::builder()
        .with_max_level(options.verbose.log_level_filter().as_trace())
        .init();

    tracing::debug!("Starting pixi2ces CLI");

    let lockfile_path = options
        .manifest_path
        .parent()
        .ok_or(anyhow!("could not get parent directory"))?
        .join("pixi.lock");

    let lockfile = LockFile::from_path(&lockfile_path).map_err(|e| {
        anyhow!(
            "could not read lockfile at {}: {}",
            lockfile_path.display(),
            e
        )
    })?;

    let env = lockfile.environment(&options.environment).ok_or(anyhow!(
        "environment not found in lockfile: {}",
        options.environment
    ))?;

    let packages = env.packages(options.platform).ok_or(anyhow!(
        "platform not found in lockfile: {}",
        options.platform.as_str()
    ))?;

    let mut conda_packages_from_lockfile: Vec<CondaPackage> = Vec::new();

    for package in packages {
        match package {
            Package::Conda(p) => conda_packages_from_lockfile.push(p),
            Package::Pypi(_) => {
                if options.ignore_pypi_errors {
                    tracing::warn!("ignoring PyPI package since PyPI packages are not supported");
                } else {
                    anyhow::bail!("PyPI packages are not supported. Specify `--ignore-pypi-errors` to ignore this error");
                }
            }
        }
    }

    let ees = build_explicit_spec(options.platform, &conda_packages_from_lockfile)?;

    tracing::info!("Creating conda lock file");
    let target = cwd()
        .join(format!(
            "conda-{}-{}.lock",
            options.platform, options.environment
        ))
        .into_os_string();

    write_explicit_spec(target, &ees)?;

    Ok(())
}
