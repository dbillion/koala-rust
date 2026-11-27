use anyhow::Result;
use koala_rust::db::Database;
use std::path::PathBuf;
use tempfile::TempDir;
use tokio::time::{Duration, Instant};

#[tokio::main]
async fn main() -> Result<()> {
    println!("Koala Rust Benchmark - Write and Recall Speed\n");
    
    // Create a temporary directory for the database
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("benchmark.db");
    
    // Initialize database
    let db = Database::new(&db_path).await?;
    
    // Test different fact counts
    let fact_counts = vec![1000, 10000, 100000];
    
    for &count in &fact_counts {
        println!("\n=== Benchmarking with {} facts ===", count);
        
        // Generate test facts
        let facts: Vec<String> = (0..count)
            .map(|i| format!("fact_{}", i))
            .collect();
        
        // Measure write speed
        let write_start = Instant::now();
        for fact in &facts {
            let text = format!("This is a test fact with number {}.", fact);
            db.write_fact(fact, &text).await?;
        }
        let write_duration = write_start.elapsed();
        
        // Measure recall speed
        let recall_start = Instant::now();
        for fact in &facts {
            let _ = db.read_fact(fact).await?;
        }
        let recall_duration = recall_start.elapsed();
        
        // Get database size
        let db_size = std::fs::metadata(&db_path)?.len();
        
        // Print results
        println!("Write speed: {:.2} facts/sec", count as f64 / write_duration.as_secs_f64());
        println!("Recall speed: {:.2} facts/sec", count as f64 / recall_duration.as_secs_f64());
        println!("Database size: {} bytes", db_size);
        println!("Write time: {:?}", write_duration);
        println!("Recall time: {:?}", recall_duration);
    }
    
    // Generate report
    generate_report().await?;
    
    Ok(())
}

async fn generate_report() -> Result<()> {
    let report_path = PathBuf::from("docs/PERF.md");
    
    let report_content = r#"# Koala Rust Performance Benchmarks

This document contains performance benchmarks for the Koala Rust knowledge management system.

## Benchmark Methodology

Benchmarks were run using the `benches/benchmark_facts.rs` script, which measures:
- Write speed: Number of facts written per second
- Recall speed: Number of facts read per second
- Database size: Size of the SQLite database file

## Benchmark Results

| Facts | Write Speed (facts/sec) | Recall Speed (facts/sec) | Database Size (bytes) | Write Time | Recall Time |
|-------|-------------------------|--------------------------|----------------------|------------|-------------|
| 1,000 | TBD                     | TBD                      | TBD                  | TBD        | TBD         |
| 10,000| TBD                     | TBD                      | TBD                  | TBD        | TBD         |
| 100,000| TBD                     | TBD                      | TBD                  | TBD        | TBD         |

## Machine Information

- TBD

## Notes

- TBD
"#;
    
    tokio::fs::write(&report_path, report_content).await?;
    println!("\nPerformance report generated at: {}", report_path.display());
    
    Ok(())
}
