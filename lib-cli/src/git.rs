use git2::{DescribeFormatOptions, DescribeOptions, Repository};
use pyrinas_shared::ota::OTAPackageVersion;
use semver::Version;
// Error handling
use thiserror::Error;

// Std
use std::{convert::TryInto, io, num};

#[derive(Debug, Error)]
pub enum GitError {
    #[error("filesystem error: {source}")]
    FileError {
        #[from]
        source: io::Error,
    },

    #[error("git repo not found!")]
    GitNotFound,

    #[error("git error: {source}")]
    GitError {
        #[from]
        source: git2::Error,
    },

    #[error("parse error: {source}")]
    ParseError {
        #[from]
        source: num::ParseIntError,
    },

    #[error("semver error: {source}")]
    SemVerError {
        #[from]
        source: semver::SemVerError,
    },

    #[error("unable to convert hash")]
    HashError,
}

pub fn get_git_describe() -> Result<String, GitError> {
    let mut path = std::env::current_dir()?;

    let repo: Repository;

    // Recursively go up levels to see if there's a .git folder and then stop
    loop {
        repo = match Repository::open(path.clone()) {
            Ok(repo) => repo,
            Err(_e) => {
                if !path.pop() {
                    return Err(GitError::GitNotFound);
                }

                continue;
            }
        };

        break;
    }

    // Describe options
    let mut opts = DescribeOptions::new();
    let opts = opts.describe_all().describe_tags();

    // Describe format
    let mut desc_format_opts = DescribeFormatOptions::new();
    desc_format_opts
        .always_use_long_format(true)
        .dirty_suffix("-dirty");

    // Describe string!
    let des = repo.describe(&opts)?.format(Some(&desc_format_opts))?;

    Ok(des)
}

pub fn get_ota_package_version(ver: &str) -> Result<(OTAPackageVersion, bool), GitError> {
    // Parse the version
    let version = Version::parse(ver)?;

    log::info!("ver: {:?}", version);

    // Then convert it to an OTAPackageVersion
    let dirty = ver.contains("dirty");
    let pre: Vec<&str> = ver.split('-').collect();
    let commit: u8 = pre[1].parse()?;
    let hash: [u8; 8] = get_hash(pre[2].as_bytes().to_vec())?;

    Ok((
        OTAPackageVersion {
            major: version.major as u8,
            minor: version.minor as u8,
            patch: version.patch as u8,
            commit: commit,
            hash: hash,
        },
        dirty,
    ))
}

fn get_hash(v: Vec<u8>) -> Result<[u8; 8], GitError> {
    match v.try_into() {
        Ok(r) => Ok(r),
        Err(_e) => Err(GitError::HashError),
    }
}
