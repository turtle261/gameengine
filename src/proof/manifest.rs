//! Parsed proof manifest and claim-status helpers.

use std::sync::OnceLock;

/// Classification for how strongly a component is covered by the proof system.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ProofStatus {
    /// Backed by bounded checks over the Rust implementation.
    Checked,
    /// Backed by an abstract Verus model only.
    Model,
    /// Backed by both Verus model lemmas and Kani implementation/refinement proofs.
    Refined,
    /// Backed by runtime tests and checks, not formal proofs.
    Runtime,
    /// Explicitly outside the formal proof boundary.
    OutOfScope,
}

impl ProofStatus {
    /// Parses a manifest status token.
    fn parse(raw: &str) -> Option<Self> {
        match raw {
            "checked" => Some(Self::Checked),
            "model" => Some(Self::Model),
            "refined" => Some(Self::Refined),
            "runtime" => Some(Self::Runtime),
            "out_of_scope" => Some(Self::OutOfScope),
            _ => None,
        }
    }

    /// Returns the markdown heading used for this status in the claim matrix.
    pub fn heading(self) -> &'static str {
        match self {
            Self::Checked => "Implementation-Checked Claims",
            Self::Model => "Model-Only Claims",
            Self::Refined => "Refined Claims",
            Self::Runtime => "Runtime-Tested Claims",
            Self::OutOfScope => "Out Of Scope",
        }
    }
}

/// Kind of verification harness referenced by the proof manifest.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum HarnessKind {
    /// A Kani harness over compiled Rust code.
    Kani,
    /// A Verus proof file or model-checking target.
    Verus,
}

/// One proof harness entry declared in the manifest.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ManifestHarness {
    /// Verification technology used by the harness.
    pub kind: HarnessKind,
    /// Stable manifest identifier for the harness.
    pub id: &'static str,
    /// Logical scope or component group the harness belongs to.
    pub scope: &'static str,
    /// Concrete target invoked by tooling for this harness.
    pub target: &'static str,
}

/// One claim about a component inside the verified boundary.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ManifestClaim {
    /// Strength of the claim.
    pub status: ProofStatus,
    /// Stable component identifier used in reports.
    pub component: &'static str,
    /// Human-readable statement of what is claimed.
    pub text: &'static str,
    /// Proof harness identifiers that justify the claim.
    pub links: &'static [&'static str],
}

/// One explicit assumption required by a proof claim.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ManifestAssumption {
    /// Component the assumption applies to.
    pub component: &'static str,
    /// Human-readable statement of the assumption.
    pub text: &'static str,
}

/// Parsed proof manifest used by reporting and verification tooling.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerificationManifest {
    boundary: &'static str,
    harnesses: Vec<ManifestHarness>,
    claims: Vec<ManifestClaim>,
    assumptions: Vec<ManifestAssumption>,
}

impl VerificationManifest {
    /// Returns the crate's statically embedded proof manifest.
    pub fn current() -> &'static Self {
        static MANIFEST: OnceLock<VerificationManifest> = OnceLock::new();
        MANIFEST.get_or_init(|| {
            let manifest = Self::parse(include_str!("../../proofs/manifest.txt"));
            manifest.validate().expect("proof manifest is invalid");
            manifest
        })
    }

    /// Parses a manifest file into a structured representation.
    pub fn parse(raw: &'static str) -> Self {
        let mut boundary = "kernel+builtins";
        let mut harnesses = Vec::new();
        let mut claims = Vec::new();
        let mut assumptions = Vec::new();

        for line in raw.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let parts: Vec<&'static str> = line.split('|').collect();
            match parts.as_slice() {
                ["boundary", value] => boundary = value,
                ["kani", id, scope, target] => harnesses.push(ManifestHarness {
                    kind: HarnessKind::Kani,
                    id,
                    scope,
                    target,
                }),
                ["verus", id, target] => harnesses.push(ManifestHarness {
                    kind: HarnessKind::Verus,
                    id,
                    scope: "global",
                    target,
                }),
                ["claim", status, component, text, links] => {
                    let status =
                        ProofStatus::parse(status).expect("proof manifest claim status is invalid");
                    let links = parse_links(links);
                    claims.push(ManifestClaim {
                        status,
                        component,
                        text,
                        links,
                    });
                }
                ["assumption", component, text] => {
                    assumptions.push(ManifestAssumption { component, text })
                }
                _ => panic!("invalid proof manifest line: {line}"),
            }
        }

        Self {
            boundary,
            harnesses,
            claims,
            assumptions,
        }
    }

    /// Returns the declared proof boundary label.
    pub fn boundary(&self) -> &'static str {
        self.boundary
    }

    /// Returns every declared proof harness.
    pub fn harnesses(&self) -> &[ManifestHarness] {
        &self.harnesses
    }

    /// Returns every declared proof claim.
    pub fn claims(&self) -> &[ManifestClaim] {
        &self.claims
    }

    /// Returns every explicit assumption listed in the manifest.
    pub fn assumptions(&self) -> &[ManifestAssumption] {
        &self.assumptions
    }

    /// Returns the Kani harnesses belonging to one manifest scope.
    pub fn kani_harnesses_for_scope(&self, scope: &str) -> impl Iterator<Item = &ManifestHarness> {
        self.harnesses
            .iter()
            .filter(move |harness| harness.kind == HarnessKind::Kani && harness.scope == scope)
    }

    /// Returns all Verus entries in the manifest.
    pub fn verus_models(&self) -> impl Iterator<Item = &ManifestHarness> {
        self.harnesses
            .iter()
            .filter(|harness| harness.kind == HarnessKind::Verus)
    }

    /// Renders the manifest into the public proof-claim markdown summary.
    pub fn render_claim_markdown(&self) -> String {
        let mut output = String::new();
        output.push_str("# Proof Claim Matrix\n\n");
        output.push_str(
            "This document is derived from `proofs/manifest.txt` and states the current proof boundary.\n\n",
        );
        output.push_str("## Verified Boundary\n\n");
        output.push_str("- ");
        output.push_str(self.boundary);
        output.push('\n');

        for status in [
            ProofStatus::Refined,
            ProofStatus::Checked,
            ProofStatus::Model,
            ProofStatus::Runtime,
            ProofStatus::OutOfScope,
        ] {
            let mut first = true;
            for claim in self.claims.iter().filter(|claim| claim.status == status) {
                if first {
                    output.push_str("\n## ");
                    output.push_str(status.heading());
                    output.push_str("\n\n");
                    first = false;
                }
                output.push_str("- `");
                output.push_str(claim.component);
                output.push_str("`: ");
                output.push_str(claim.text);
                if !claim.links.is_empty() {
                    output.push_str(" (proof ids: ");
                    let mut first_link = true;
                    for link in claim.links {
                        if !first_link {
                            output.push_str(", ");
                        }
                        output.push('`');
                        output.push_str(link);
                        output.push('`');
                        first_link = false;
                    }
                    output.push(')');
                }
                output.push('\n');
            }
        }

        if !self.assumptions.is_empty() {
            output.push_str("\n## Assumptions\n\n");
            for assumption in &self.assumptions {
                output.push_str("- `");
                output.push_str(assumption.component);
                output.push_str("`: ");
                output.push_str(assumption.text);
                output.push('\n');
            }
        }

        output
    }

    /// Validates manifest consistency, proof links, and claim/status coherence.
    pub fn validate(&self) -> Result<(), String> {
        let mut harness_ids = Vec::new();
        for harness in &self.harnesses {
            if harness_ids.contains(&harness.id) {
                return Err(format!(
                    "duplicate harness id `{}` in proof manifest",
                    harness.id
                ));
            }
            harness_ids.push(harness.id);
        }

        let mut claim_components = Vec::new();
        for claim in &self.claims {
            if claim_components.contains(&claim.component) {
                return Err(format!(
                    "duplicate claim component `{}` in proof manifest",
                    claim.component
                ));
            }
            claim_components.push(claim.component);

            for link in claim.links {
                if !harness_ids.contains(link) {
                    return Err(format!(
                        "claim `{}` references unknown proof id `{link}`",
                        claim.component
                    ));
                }
            }

            let has_kani = claim.links.iter().any(|link| {
                self.harnesses
                    .iter()
                    .any(|harness| harness.id == *link && harness.kind == HarnessKind::Kani)
            });
            let has_verus = claim.links.iter().any(|link| {
                self.harnesses
                    .iter()
                    .any(|harness| harness.id == *link && harness.kind == HarnessKind::Verus)
            });

            match claim.status {
                ProofStatus::Refined => {
                    if !has_kani || !has_verus {
                        return Err(format!(
                            "refined claim `{}` must link both Kani and Verus proofs",
                            claim.component
                        ));
                    }
                }
                ProofStatus::Checked => {
                    if !has_kani || has_verus {
                        return Err(format!(
                            "checked claim `{}` must link Kani proofs only",
                            claim.component
                        ));
                    }
                }
                ProofStatus::Model => {
                    if !has_verus || has_kani {
                        return Err(format!(
                            "model claim `{}` must link Verus proofs only",
                            claim.component
                        ));
                    }
                }
                ProofStatus::Runtime | ProofStatus::OutOfScope => {
                    if has_kani || has_verus {
                        return Err(format!(
                            "{} claim `{}` must not link formal proof ids",
                            match claim.status {
                                ProofStatus::Runtime => "runtime",
                                ProofStatus::OutOfScope => "out_of_scope",
                                _ => unreachable!(),
                            },
                            claim.component
                        ));
                    }
                }
            }
        }

        Ok(())
    }
}

fn parse_links(raw: &'static str) -> &'static [&'static str] {
    let links: Vec<&'static str> = raw
        .split(',')
        .map(str::trim)
        .filter(|link| !link.is_empty())
        .collect();
    Box::leak(links.into_boxed_slice())
}

#[cfg(test)]
mod tests {
    use super::{ProofStatus, VerificationManifest};

    #[test]
    fn manifest_is_valid() {
        VerificationManifest::current().validate().unwrap();
    }

    #[test]
    fn rendered_claims_include_refined_section() {
        let rendered = VerificationManifest::current().render_claim_markdown();
        assert!(rendered.contains(ProofStatus::Refined.heading()));
    }

    #[test]
    fn checked_claims_require_kani_only_links() {
        let manifest = VerificationManifest::parse(
            "kani|k|default|k\nverus|v|proofs/verus/core_model.rs\nclaim|checked|engine.bad|bad claim|v\n",
        );
        let error = manifest.validate().unwrap_err();
        assert!(error.contains("must link Kani proofs only"));
    }

    #[test]
    fn runtime_claims_reject_formal_links() {
        let manifest =
            VerificationManifest::parse("kani|k|default|k\nclaim|runtime|engine.bad|bad claim|k\n");
        let error = manifest.validate().unwrap_err();
        assert!(error.contains("must not link formal proof ids"));
    }
}
