use cargo_metadata::Package;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum PkgSource {
    Local,
    CratesIo,
    Foreign,
}
#[derive(Debug, Clone)]
pub struct SourcedPackage {
    pub source: PkgSource,
    pub package: Package,
}
