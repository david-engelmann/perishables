//! The Databricks Genie serialized space reader. A space's
//! configuration travels as a JSON string in serialized_space
//! when the API is asked for it with edit permission, and the
//! same shape arrives as an exported space file. This reader
//! takes that JSON, from either route, into the grading
//! model. Text fields in the serialized form are arrays of
//! strings, and this reader joins them.
//!
//! The mapping. Data source tables are the models, named by
//! their Unity Catalog identifiers, and their column configs
//! are gradeable items in the dimension family. Benchmarks
//! and example question SQLs are stored ground truth pairs,
//! recorded as claims whose shape grades offline. Sample
//! questions carry no stored answer and are coverage
//! observations. Running the ground truth through the
//! Conversation API is the behavioral family and needs a
//! warehouse and the vendor runtime, so it is outside this
//! slice, stated here rather than smoothed over. The learned
//! layer, mined snippets and OntoRank scores, is not in the
//! serialized form at all, and nothing here approximates it.
//! Two further surfaces the design record names are outside
//! this reader, stated as the boundary. Certification is a
//! governed tag on Unity Catalog objects and arrives caller
//! side, like a last change instant, not inside the space.
//! Metric view synonyms live in a separate YAML surface with
//! its own reader when that slice is built.

use super::{Definition, Dimension, GroundTruth, Json, opt_text_of, text_of};

/// Parse serialized space JSON text into the grading model.
pub fn read_databricks_definition(text: &str) -> Result<Definition, String> {
    let root = super::json::parse(text)?;
    let mut def = Definition::default();
    if let Some(tables) = root
        .get("data_sources")
        .and_then(|d| d.get("tables"))
        .and_then(Json::as_list)
    {
        for table in tables {
            let identifier = text_of(table, "identifier");
            def.model_names.push(identifier.clone());
            if let Some(columns) = table.get("column_configs").and_then(Json::as_list) {
                for column in columns {
                    def.dimensions.push(Dimension {
                        name: text_of(column, "name"),
                        label: None,
                        description: opt_text_of(column, "description"),
                        model: identifier.clone(),
                    });
                }
            }
        }
    }
    if let Some(questions) = root
        .get("benchmarks")
        .and_then(|b| b.get("questions"))
        .and_then(Json::as_list)
    {
        for (index, benchmark) in questions.iter().enumerate() {
            let item = opt_text_of(benchmark, "id").unwrap_or_else(|| format!("benchmark-{index}"));
            def.ground_truth.push(GroundTruth {
                item,
                question: joined(benchmark.get("question")),
                expected_sql: sql_answer(benchmark.get("answer")),
            });
        }
    }
    if let Some(examples) = root
        .get("instructions")
        .and_then(|i| i.get("example_question_sqls"))
        .and_then(Json::as_list)
    {
        for (index, example) in examples.iter().enumerate() {
            def.ground_truth.push(GroundTruth {
                item: format!("example-{index}"),
                question: joined(example.get("question")),
                expected_sql: joined(example.get("sql")),
            });
        }
    }
    if let Some(samples) = root
        .get("config")
        .and_then(|c| c.get("sample_questions"))
        .and_then(Json::as_list)
    {
        for sample in samples {
            let question = joined(sample.get("question"));
            if !question.is_empty() {
                def.unverifiable_questions.push(question);
            }
        }
    }
    Ok(def)
}

/// Text fields in the serialized form are arrays of strings.
/// Join one into a single string, tolerating a bare string.
fn joined(value: Option<&Json>) -> String {
    match value {
        Some(Json::List(_)) => value.map(Json::as_str_list).unwrap_or_default().join(" "),
        Some(Json::Str(s)) => s.clone(),
        _ => String::new(),
    }
}

/// A benchmark answer is a list of format and content pairs.
/// The SQL answer is the content of the first SQL entry.
fn sql_answer(value: Option<&Json>) -> String {
    let Some(Json::List(answers)) = value else {
        return String::new();
    };
    for answer in answers {
        if answer.get("format").and_then(Json::as_str) == Some("SQL") {
            return joined(answer.get("content"));
        }
    }
    String::new()
}
