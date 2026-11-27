#![no_main]

use std::fs;
use std::path::PathBuf;

use libfuzzer_sys::fuzz_target;
use perishables_core::{
    AuditClock, Bundle, Concept, SourceRef, VerifiedEvent, check_currency, check_drift,
    check_freshness, check_provenance, check_supersession, info_notes, recompute_tier, sha256_hex,
    sort_findings, walk_supersession,
};

const MAX_INPUT_BYTES: usize = 32 * 1024;

fn scratch_bundle_dir() -> PathBuf {
    std::env::temp_dir().join(format!("perishables-fuzz-bundle-{}", std::process::id()))
}

fn evaluate_bundle(bundle: &Bundle, clock: &AuditClock) {
    let mut findings = check_currency(bundle, clock);
    findings.extend(check_supersession(bundle));
    findings.extend(check_freshness(bundle, clock));
    findings.extend(check_provenance(bundle));
    findings.extend(check_drift(bundle));
    sort_findings(&mut findings);
    let _ = info_notes(bundle, clock);

    for concept in &bundle.concepts {
        let _ = recompute_tier(concept, clock);
        let _ = walk_supersession(bundle, &concept.id);
    }
}

fn split_at_char_boundary(s: &str, approx: usize) -> (&str, &str) {
    let mut idx = approx.min(s.len());
    while idx > 0 && !s.is_char_boundary(idx) {
        idx -= 1;
    }
    s.split_at(idx)
}

fuzz_target!(|data: &[u8]| {
    if data.len() > MAX_INPUT_BYTES {
        return;
    }

    let digest = sha256_hex(data);
    assert_eq!(digest.len(), 64);

    let text = String::from_utf8_lossy(data);

    // 1. Fuzz AuditClock::parse and verify canonical RFC 3339 round-tripping.
    let fallback_clock = AuditClock {
        epoch_seconds: 1_795_132_800,
    };
    let mut active_clock = fallback_clock;
    if let Some(parsed) = AuditClock::parse(&text) {
        let formatted = parsed.to_rfc3339();
        if let Some(reparsed) = AuditClock::parse(&formatted) {
            assert_eq!(parsed, reparsed);
        }
        active_clock = parsed;
    }
    for line in text.lines().take(16) {
        if let Some(parsed) = AuditClock::parse(line.trim()) {
            let formatted = parsed.to_rfc3339();
            let _ = AuditClock::parse(&formatted);
            active_clock = parsed;
        }
    }

    // 2. Fuzz in-memory Concept and supersession graph checks directly.
    let mut lines = text.lines();
    let l0 = lines.next().unwrap_or("metrics/alpha");
    let l1 = lines.next().unwrap_or("stable");
    let l2 = lines.next().map(str::to_string);
    let l3 = lines.next().unwrap_or("2026-10-01T00:00:00Z");
    let l4 = lines.next().unwrap_or("human:reviewer");

    let mem_bundle = Bundle::from_concepts(vec![
        Concept {
            id: "metrics/a".to_string(),
            concept_type: "Metric".to_string(),
            status: l1.to_string(),
            superseded_by: l2.clone(),
            generated_at: AuditClock::parse(l3).map(|c| c.epoch_seconds),
            generated_text: Some(l3.to_string()),
            stale_after_text: Some(l0.to_string()),
            stale_after: AuditClock::parse(l0).map(|c| c.epoch_seconds),
            verified: vec![VerifiedEvent {
                by: Some(l4.to_string()),
                at_text: Some(l3.to_string()),
                at: AuditClock::parse(l3).map(|c| c.epoch_seconds),
            }],
            sources: vec![SourceRef {
                id: Some("src-1".to_string()),
                resource: Some(l0.to_string()),
                sha256: Some(digest),
            }],
            citations: vec![l0.to_string()],
            runtime: None,
            parameters: Vec::new(),
            code_blocks: Vec::new(),
            file_code: None,
            claimed_result: None,
        },
        Concept {
            id: "metrics/b".to_string(),
            concept_type: "Metric".to_string(),
            status: "deprecated".to_string(),
            superseded_by: Some("metrics/a".to_string()),
            generated_at: None,
            generated_text: None,
            stale_after_text: None,
            stale_after: None,
            verified: Vec::new(),
            sources: Vec::new(),
            citations: Vec::new(),
            runtime: None,
            parameters: Vec::new(),
            code_blocks: Vec::new(),
            file_code: None,
            claimed_result: None,
        },
    ]);
    evaluate_bundle(&mem_bundle, &active_clock);

    // 3. Fuzz full disk-backed frontmatter, Markdown fence, citation, and path parsing.
    let root = scratch_bundle_dir();
    let metrics_dir = root.join("metrics");
    if fs::create_dir_all(&metrics_dir).is_ok() {
        let (head, tail) = split_at_char_boundary(&text, text.len() / 2);
        let _ = fs::write(root.join("index.md"), head.as_bytes());
        let _ = fs::write(metrics_dir.join("raw.md"), text.as_bytes());
        let wrapped = format!("---\ntype: Metric\n{head}\n---\n{tail}\n");
        let _ = fs::write(metrics_dir.join("wrapped.md"), wrapped.as_bytes());

        if let Ok(disk_bundle) = Bundle::load(&root) {
            evaluate_bundle(&disk_bundle, &active_clock);
        }
    }
});
