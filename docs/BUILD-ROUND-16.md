# Build round 16. LookML slice 1, the native reader

Date. 2026-11-25. The tenth cross ecosystem build, stacked on the design branch, implementing slice 1 of the design record at docs/DESIGN-LOOKML.md.

What landed. A scanner and block tree parser for the design record's stated grammar subset, and the mapping onto the grading model. Blocks with names, properties with bareword, quoted, list, and SQL values, the SQL form running to its double semicolon terminator, hash comments outside SQL chunks, and interpolation recorded rather than expanded. A file the subset cannot scan is a reader error, and a test pins that refusal, an unterminated SQL chunk must not grade partially.

The mapping matches the API reader where the two describe the same things. Views are models, dimensions are items, synonyms are alias claims, explore joins are relationships. A LookML measure is an aggregation over fields, so it maps to a metric whose measure references are the field references inside its SQL, the TABLE pseudo reference excluded and dotted cross view references left to slice 2 with project scope, as the design record states. One shared check widened honestly to carry this. A measure reference in the dangling check now resolves against dimensions as well as measures, because a LookML field reference names fields of either kind. No existing fixture exercises the difference, and the widening is recorded here rather than passed over.

The gate. A clean two view project grades clean, its measure's reference resolving against its view's dimension. The defect file is the API slice's defect model re-expressed as files plus the one defect only files can show, and it produces exactly three findings, the dangling field reference, the dangling join, and the synonym collision, the first of those invisible in the API document shape. grade-looker now routes by extension, files to the native reader and JSON to the API reader.

Landing route. Stacked on design-lookml-1 as branch cross-ecosystem-lookml-1, for the standing route. Slice 2 of the design record, model files, includes, and extends resolution, is the named next item in this project.
