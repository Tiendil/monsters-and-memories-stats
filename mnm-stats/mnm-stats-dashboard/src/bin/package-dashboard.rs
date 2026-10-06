//! Package the selected archive and deployment provenance into Trunk's staging directory.
#[path = "../../build_publication.rs"]
mod publication;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args_os().skip(1).collect::<Vec<_>>();
    if args.len() != 2 {
        return Err("usage: package-dashboard HISTORY STAGING_DIRECTORY".into());
    }
    publication::package(
        std::path::Path::new(&args[0]),
        std::path::Path::new(&args[1]),
        &std::env::var("MNM_STATS_BUILD_REVISION").unwrap_or_default(),
        &std::env::var("MNM_STATS_DATA_REVISION").unwrap_or_default(),
    )
}
