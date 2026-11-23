# Design note. Re execution verdicts as in-toto statements in a DSSE envelope

Status. Design note only, written 2026-11-23 under the applicability audit charge. No build is charged by this note. Main reviews it before any implementation is ordered.

## Why

OKF v0.3 defines Attested Computations and then deliberately defers the receipt and verdict wire format. A bundle can claim an attested result, and perishables can recompute it and say whether the claim holds, but the verdict today exists only as a line in a perishables report. It cannot travel with the bundle, and no other tool can verify it without rerunning the audit. The ecosystem already has a standing shape for a verdict that travels. An in-toto Statement names a subject by digest and attaches a typed predicate about it, and a DSSE envelope wraps that statement with signatures so a consumer can verify who said it without trusting the carrier. Emitting perishables verdicts in that shape, carried inside OKF bundles through the extension rule, lets any future OKF consumer check a verdict the way the rest of the attestation ecosystem already does, without waiting for the spec to finish deferring.

## The mapping

The statement type is the in-toto v1 statement, identified by its type URI, https://in-toto.io/Statement/v1. Every field the statement needs is one the auditor already computes and prints since checkpoint 1, which is what makes this an emission change rather than new audit machinery.

Subject. The computation concept is the subject. Its name is the concept id. Its digest is the SHA-256 of the bound artifact, the declared parameter bindings plus the declared computation, which the report already records before execution. Binding the verdict to that digest is the point of the exercise. A replaced computation file or a changed parameter changes the digest, and the statement then describes a computation that is no longer the one in the bundle, which a consumer can detect without executing anything.

Predicate type. A URI naming the perishables recompute verdict schema, versioned. The exact home of that URI is an open question for Main, recorded below. The version moves only when the predicate shape changes, under the same discipline as the finding codes.

Predicate. The verdict vocabulary maps one to one. The verdict name from the design table, RECOMPUTED_MATCH, MISMATCH, NOT_ATTESTABLE, NOT_EXECUTABLE_IN_PROFILE, EXECUTION_FAILED, or NON_DETERMINISTIC_PROFILE, with its reason where it carries one. The claimed result and the recomputed result where a run produced one. The runtime and the profile name, so a consumer knows the verdict was reached under the deterministic python profile and not some other executor. The auditor name and version. The pinned as-of instant the audit ran under. The wall clock bound the run executed under.

One field is deliberately excluded. Elapsed time is in the human report and stays out of the signed predicate. A receipt that changes bytes on every run cannot be compared across runs, and the ecosystem habit is that statements about the same subject and predicate are comparable. The bound is a property of the profile and stays. The elapsed time is a property of one machine on one day and goes.

## The carrier inside OKF

The v0.3 extension rule is x-<producer>-<name> at any level, with a duty on producers to preserve extensions they do not emit. The natural home is the computation concept itself, one field named x-perishables-attestation holding the envelope for that concept verdict. A consumer that knows the extension reads the verdict where the claim lives. A consumer that does not know it preserves the field untouched, which the spec already requires. The alternative, a bundle level list on the index, separates verdicts from their concepts and buys nothing.

## The envelope and the signing question

A DSSE envelope is three fields. The payload, which is the base64 of the statement JSON. The payload type, which for an in-toto statement is application/vnd.in-toto+json. And the signatures, each a key id and a signature over the pre auth encoding of the payload type and payload.

The envelope shape is settled. The signing story is not, and it is the real decision in this note. Perishables is deterministic and offline by design, and it holds no keys today. Three options present themselves. First, an auditor key configured out of band, named by a key id in the envelope, which keeps the offline property and makes the operator the trust root. Second, keyless signing in the Sigstore pattern, which needs network access at audit time and breaks the offline rule. Third, emit the statement without an envelope until a key story is ruled, which gives consumers the mapping today and the verifiability later. The recommendation is the first option when a deployment has a key to hold, and the third as the default emission until then. An envelope with an empty signature list is not offered, because verifiers reject it and a shape that cannot verify teaches consumers to ignore the field.

## What emitting it takes

The work is bounded and sits in the adapter crate, beside the executor, never in the library, which performs no IO and signs nothing. A statement serializer for the one predicate shape. Base64 for the payload when the envelope is emitted. A command line surface that writes attestations beside the report rather than inside it, so the existing report stays byte identical for consumers that diff it. A corpus fixture whose label carries the expected statement for a known verdict, so the mapping itself is gated the way every other claim in this repo is gated. And a minor version note, because a new output surface is an addition under the publication discipline, not a change to any existing meaning.

Two cautions close the note. The statement asserts that this auditor, at this clock, under this profile, reached this verdict about this artifact digest. It does not assert that the computation is correct in the world, only that the bundle claim and the recomputation agree or do not. Wording inside the predicate must keep that boundary, because a receipt that overclaims is worse than no receipt. And the predicate type URI needs a home Main controls before the first emission, because renaming a published predicate type is a breaking change wearing a small hat.

## Open questions for Main

Where the predicate type URI lives. Whether the first emission is statement only or enveloped with an operator key. Whether verdicts for computations that never executed, the NOT_ATTESTABLE family, are emitted at all or only verdicts backed by a run, noting that an unattestable claim is exactly the case a travelling verdict would serve best.
