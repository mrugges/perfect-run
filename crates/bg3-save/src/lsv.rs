use bg3_lib::package::Package;
use bg3_lib::package_reader::PackageReader;
use std::path::Path;

/// Open an LSV save file and return the PackageReader and parsed Package.
pub fn open_package(path: &Path) -> Result<(PackageReader, Package), String> {
    let mut reader = PackageReader::new(path)?;
    let package = reader.read()?;
    Ok((reader, package))
}

/// List all file names contained in an LSV package.
pub fn list_files(package: &Package) -> Vec<String> {
    package
        .files
        .iter()
        .map(|f| f.name.to_string_lossy().to_string())
        .collect()
}
