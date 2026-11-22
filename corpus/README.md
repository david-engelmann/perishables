# The planted defect corpus

This directory holds the corpus of small OKF bundles with planted trust defects. Each must fail fixture carries exactly one defect and a label that names the rule it trips. Each must pass fixture carries none. The manifest in this directory lists every fixture with its defect code, its expected finding, and the auditor version range the expectation applies to.

The corpus is a published artifact with its own version, recorded in the manifest. Any OKF tool can run these bundles and compare its own findings with the labels. A corpus release is cut alongside a tool release and is never edited silently.

The first fixtures land with the first build round, seeded from the hand built bundles of the design phase.
