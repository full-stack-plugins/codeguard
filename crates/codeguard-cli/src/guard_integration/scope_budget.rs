//! Admission cost for the domain's worst-case missing-target diagnostic expansion.
use guardengine::integration::MAX_ARTIFACT_BYTES;
use std::{collections::BTreeMap, io};

const ERROR: &str = "scope construction budget exceeded";

// Counts escaped JSON bytes without constructing an encoded string or Value.
fn encoded_size(value: &str) -> Result<usize, &'static str> {
    struct Counter(usize);
    impl io::Write for Counter {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            if bytes.len() > MAX_ARTIFACT_BYTES.saturating_sub(self.0) {
                return Err(io::Error::new(io::ErrorKind::InvalidData, ERROR));
            }
            self.0 += bytes.len();
            Ok(bytes.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    let mut counter = Counter(0);
    serde_json::to_writer(&mut counter, value).map_err(|_| ERROR)?;
    Ok(counter.0)
}

pub(super) fn validate(targets: &BTreeMap<String, Vec<String>>) -> Result<(), &'static str> {
    let mut charged = 1024usize; // Fixed unqualified/aggregate diagnostics and collection headers.
    for (id, items) in targets {
        let id_bytes = encoded_size(id)?;
        // Worst case includes one incomplete-obligation diagnostic as well as every
        // missing target. Reserve four encoded equivalents for formatting capacity,
        // owned gaps/source strings, and collection overhead; this is admission
        // accounting, not a promise of an exact process allocation/RSS ceiling.
        charged = charged.saturating_add(id_bytes.saturating_add(128).saturating_mul(4));
        for target in items {
            let gap_bytes = id_bytes
                .saturating_add(encoded_size(target)?)
                .saturating_add(128);
            charged = charged.saturating_add(gap_bytes.saturating_mul(4));
            if charged > MAX_ARTIFACT_BYTES {
                return Err(ERROR);
            }
        }
        if charged > MAX_ARTIFACT_BYTES {
            return Err(ERROR);
        }
    }
    Ok(())
}

/// Admit all possible diagnostic strings and copied target identities before
/// constructing them. The shared 16MiB budget applies to the combined dimensions.
pub(super) fn validate_requirements(
    requirements: &BTreeMap<String, super::scope::RequiredScope>,
) -> Result<(), &'static str> {
    let mut charged = 1024usize;
    for (id, scope) in requirements {
        let id_bytes = encoded_size(id)?;
        charged = charged.saturating_add(id_bytes.saturating_add(128).saturating_mul(4));
        let mut row = |fields: &[&str]| -> Result<(), &'static str> {
            let mut bytes = id_bytes.saturating_add(128);
            for field in fields {
                bytes = bytes.saturating_add(encoded_size(field)?);
            }
            charged = charged.saturating_add(bytes.saturating_mul(4));
            if charged > MAX_ARTIFACT_BYTES {
                return Err(ERROR);
            }
            Ok(())
        };
        for target in &scope.targets {
            row(&[target])?;
        }
        for rule in &scope.rules {
            row(&[rule])?;
        }
        for tool in &scope.tools {
            row(&[&tool.id, &tool.version, &tool.sha256])?;
            row(&[&tool.id])?; // Matching identity and per-obligation execution are distinct.
        }
        for config in &scope.configurations {
            row(&[&config.reference, &config.sha256])?;
        }
        if charged > MAX_ARTIFACT_BYTES {
            return Err(ERROR);
        }
    }
    Ok(())
}
