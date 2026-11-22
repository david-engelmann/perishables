//! The perishables command line auditor.
//!
//! The scaffold prints its name and version. The audit and corpus
//! commands land with the first build round, over the checks in
//! perishables-core.

fn main() {
    println!("perishables {}", env!("CARGO_PKG_VERSION"));
    println!("scaffold build, the audit and corpus commands are not built yet");
}
