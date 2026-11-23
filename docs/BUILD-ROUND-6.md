# Build round 6 gate report, the drift promotion

Scope. One change, ruled by David on 2026-11-23. Source drift joins the default audit gate.

The condition the design record set for this promotion was separation evidence from the corpus, and it is in. The drift check produces exactly one finding on its own fixture, T9, and silence on all sixteen other fixtures, asserted both by a dedicated separation test and by the exact code set table the checkpoint added. David was presented with that evidence and ruled the promotion.

What changed. The CLI default findings set now includes check_drift, so the audit command and the corpus reference runner both gate on drift. The runner no longer treats the extension set specially, because there is no longer an extension set. The core corpus gate runs drift inside its standard findings, T9 sits in the exact code set table with SOURCE_DRIFT as its only expected code, and the fixture label and the manifest now record its set as gate. The separation test stands unchanged as the regression guard. The drift subcommand remains for running the check on its own.

What did not change. The check itself is byte for byte the round 5 implementation. No expected result in the corpus changed, so the corpus version stays at 0.1.0 under the manifest versioning rules. Only set membership moved.

Sequencing note. The promotion was seated only after the checkpoint fix pass verdict was read, green on head 5d39aed6, so the two changes never stacked unread.

Landing. The promotion commit is 304a6675. Its first run failed at clippy on a single element loop left in the silence test when T9 left its list, repaired in 96656e4f, and the round is hosted green on the exact head 96656e4f, run 45188103451.
