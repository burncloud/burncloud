use super::{ArtifactSource, Digest};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct ArtifactRequest {
    source: ArtifactSource,
    expected_digest: Digest,
}

impl ArtifactRequest {
    pub(crate) fn new(source: ArtifactSource, expected_digest: Digest) -> Self {
        Self {
            source,
            expected_digest,
        }
    }

    pub(crate) const fn source(&self) -> &ArtifactSource {
        &self.source
    }

    pub(crate) const fn expected_digest(&self) -> &Digest {
        &self.expected_digest
    }
}

#[cfg(test)]
#[path = "../../tests/support/artifact_request.rs"]
mod tests;
