# Build round 14. Cross ecosystem slice 8, the Ossie YAML form

Date. 2026-11-25. The eighth build from the cross ecosystem design record, stacked on slices 1 to 7.

What landed. The Ossie reader gains its YAML form, closing the boundary its first slice stated. The reader is refactored so the mapping lives in one place over a parsed document, and the JSON and YAML entries differ only in which parser feeds it, the YAML reader written for the Snowflake slice. The command line keeps one grade-ossie command and picks the reader by file extension, so the two forms of a document are one surface, not two.

The gate is equivalence, which is the strongest form this slice can take. The YAML fixtures mirror the JSON fixtures field for field, and the tests assert the YAML parse produces a model equal to the JSON parse of the same document, then grade both clean and collision states in the YAML form with the same exact results. A form that graded differently from its twin would be two dialects, and the gate exists to prove there is one.

Landing route. Stacked on cross-ecosystem-atscale-1 as branch cross-ecosystem-ossie-yaml-1, for the standing route. The Cube YAML form follows in the same pass.
