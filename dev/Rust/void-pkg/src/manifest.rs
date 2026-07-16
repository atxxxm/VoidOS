use serde::{Deserialize, Serialize};

// The package archive is a plain, uncompressed tar with a `void.toml`
// manifest at its root (parsed into this struct) plus payload files under
// a `files/` prefix, which get extracted with that prefix stripped
// (`files/bin/hello` -> `/bin/hello`).
#[derive(Serialize, Deserialize, Clone)]
pub struct Manifest {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub dependencies: Vec<String>,
}
