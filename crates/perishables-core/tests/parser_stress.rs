//! Deterministic seeded stress tests, symlink loop guards, and edge-case
//! invariants for `perishables-core` parsers and checks.

use perishables_core::{
    AuditClock, Bundle, ChainTerminal, Concept, FindingCode, check_currency, check_drift,
    check_freshness, check_provenance, check_supersession, info_notes, walk_supersession,
};

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        let idx = (self.next_u64() as usize) % items.len();
        &items[idx]
    }
}

#[test]
fn seeded_adversarial_markdown_never_panics_and_stays_deterministic() {
    let tokens: &[&str] = &[
        "---\n",
        "...\n",
        "type: Metric # inline comment\n",
        "status: deprecated\n",
        "superseded_by: /a/../b.md\n",
        "verified: [{by: \"human:İ\", at: \"2026-02-29\"}, {by: 'team:x#1', at: '2024-02-29T23:59:59Z'}]\n",
        "sources:\n  # comment\n  - id: \"a\\\"b\"\n    resource: https://example.com/x#frag\n",
        "parameters: [{name: x, value: \"1\"}, {name: y}]\n",
        "# İstanbul 🦀\n",
        "Claimed result\n",
        "The attested result claimed for this computation is 3.14.\n",
        "```python title=\"demo\"\n[^code-ref]\n```\n",
        "~~~\n[^tilde-ref]\n~~~\n",
        "`[^inline-code]` and [^real-cite] and [^def-only]: text\n",
        "{{{[[[,,,\"\\\"\\\\'",
        "\u{feff}",
        "\r\n",
        "   -   \n",
        ":::\n",
    ];
    let mut rng = Rng::new(0x5EED_CAFE_BAAD_F00D);
    let temp = std::env::temp_dir().join(format!("perishables-core-stress-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp);
    std::fs::create_dir_all(&temp).expect("temp dir");
    let clock = AuditClock::parse("2026-11-20").expect("clock");

    for iter in 0..300 {
        let count = 1 + (rng.next_u64() as usize % 18);
        let mut doc = String::from("---\ntype: Attested Computation\n");
        for _ in 0..count {
            doc.push_str(rng.pick(tokens));
        }
        std::fs::write(temp.join("concept.md"), &doc).expect("write");
        let b1 = Bundle::load(&temp).expect("load 1");
        let b2 = Bundle::load(&temp).expect("load 2");
        assert_eq!(b1, b2, "iteration {iter} must load deterministically");
        let _ = check_currency(&b1, &clock);
        let _ = check_supersession(&b1);
        let _ = check_freshness(&b1, &clock);
        let _ = check_provenance(&b1);
        let _ = check_drift(&b1);
        let _ = info_notes(&b1, &clock);
    }
    let _ = std::fs::remove_dir_all(&temp);
}

#[test]
fn seeded_timestamp_stress_and_rfc3339_roundtrip() {
    let mut rng = Rng::new(0x1234_5678_9ABC_DEF0);
    // Round-trip 500 random epochs between 1970 and 2200.
    for _ in 0..500 {
        let epoch = (rng.next_u64() % 7_258_118_400) as i64;
        let clock = AuditClock {
            epoch_seconds: epoch,
        };
        let formatted = clock.to_rfc3339();
        let reparsed = AuditClock::parse(&formatted).expect("formatted RFC 3339 must parse");
        assert_eq!(reparsed, clock, "round-trip mismatch for {formatted}");
    }

    // Random garbage / boundary strings never panic.
    let alphabet: &[&str] = &[
        "2026", "2024", "1900", "2000", "-", "02", "29", "30", "31", "00", "13", "T", "t", " ",
        "12", "59", "60", "61", ":", ".", "Z", "z", "+02:00", "-05:30", "+25:00", "+02", "İ",
    ];
    for _ in 0..1_000 {
        let parts = 1 + (rng.next_u64() as usize % 12);
        let mut s = String::new();
        for _ in 0..parts {
            s.push_str(rng.pick(alphabet));
        }
        let r1 = AuditClock::parse(&s);
        let r2 = AuditClock::parse(&s);
        assert_eq!(r1, r2);
    }
}

#[test]
fn chain_head_deduplication_and_spec_12_1_fallback_note() {
    let make = |id: &str, superseded_by: Option<&str>| Concept {
        id: id.to_string(),
        concept_type: "Metric".to_string(),
        status: if superseded_by.is_some() {
            "deprecated".to_string()
        } else {
            "stable".to_string()
        },
        superseded_by: superseded_by.map(str::to_string),
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
    };

    // Multi-hop dangling chain a -> b -> missing emits exactly one finding on chain head a.
    let bundle = Bundle::from_concepts(vec![
        make("metrics/a", Some("/metrics/b")),
        make("metrics/b", Some("/metrics/missing")),
    ]);
    let findings = check_supersession(&bundle);
    assert_eq!(
        findings.len(),
        1,
        "expected single chain-head finding, got {findings:?}"
    );
    assert_eq!(findings[0].concept_id, "metrics/a");
    assert_eq!(findings[0].code, FindingCode::DanglingSupersededBy);

    // SPEC 12.1 root-relative fallback records fallback_targets and surfaces an InfoNote.
    let fallback_bundle = Bundle::from_concepts(vec![
        make("legacy/old", Some("current/new")),
        make("current/new", None),
    ]);
    let walk = walk_supersession(&fallback_bundle, "legacy/old");
    assert_eq!(walk.terminal, ChainTerminal::Terminated);
    assert_eq!(walk.fallback_targets, vec!["current/new".to_string()]);
    let clock = AuditClock::parse("2026-11-20").unwrap();
    let notes = info_notes(&fallback_bundle, &clock);
    assert!(
        notes
            .iter()
            .any(|n| n.concept_id == "legacy/old" && n.text.contains("SPEC 12.1")),
        "expected SPEC 12.1 fallback note, got {notes:?}"
    );
}

#[test]
fn escaping_source_paths_report_missing_source_while_external_uris_skip() {
    let dir = std::env::temp_dir().join(format!(
        "perishables-escaping-sources-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("metrics")).expect("mkdir");
    std::fs::write(
        dir.join("index.md"),
        "---\nokf_version: \"0.3\"\ntitle: Escaping source test\n---\n",
    )
    .expect("index");
    std::fs::write(
        dir.join("metrics/rev.md"),
        "---\ntype: Metric\nstatus: stable\nsources:\n  - id: ext-https\n    resource: https://example.com/ok.csv\n  - id: ext-s3\n    resource: s3://bucket/ok.parquet\n  - id: esc-posix\n    resource: ../../etc/passwd\n  - id: esc-rel\n    resource: ../outside.csv\n  - id: esc-win\n    resource: \"..\\\\outside.csv\"\n  - id: esc-root\n    resource: /\n---\nBody.\n",
    )
    .expect("concept");
    let bundle = Bundle::load(&dir).expect("load bundle");
    let _ = std::fs::remove_dir_all(&dir);
    let findings = check_provenance(&bundle);
    assert_eq!(
        findings.len(),
        4,
        "all 4 escaping/empty source paths must emit MISSING_SOURCE while https/s3 skip: {findings:?}"
    );
    assert!(
        findings
            .iter()
            .all(|f| f.code == FindingCode::MissingSource)
    );
}

#[cfg(unix)]
#[test]
fn symlink_cycle_inside_bundle_does_not_stack_overflow() {
    use std::os::unix::fs::symlink;

    let dir = std::env::temp_dir().join(format!("perishables-symlink-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("dir");
    std::fs::write(
        dir.join("index.md"),
        "---\nokf_version: \"0.3\"\ntitle: Symlink test\n---\n",
    )
    .expect("index");
    std::fs::write(
        dir.join("metric.md"),
        "---\ntype: Metric\nstatus: stable\n---\nBody.\n",
    )
    .expect("metric");
    symlink(&dir, dir.join("self_loop")).expect("create directory symlink loop");

    let bundle = Bundle::load(&dir).expect("Bundle::load must survive symlink loops");
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(bundle.concepts.len(), 1);
    assert_eq!(bundle.concepts[0].id, "metric");
}
