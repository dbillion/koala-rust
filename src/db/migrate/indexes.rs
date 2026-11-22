use anyhow::Result;
use sqlx::SqlitePool;

pub(super) async fn run(pool: &SqlitePool) -> Result<()> {
    sqlx::query(r#"
            CREATE INDEX IF NOT EXISTS idx_facts_name ON facts(name);
            CREATE INDEX IF NOT EXISTS idx_facts_current ON facts(is_current);
            CREATE INDEX IF NOT EXISTS idx_observations_entity ON observations(entity_name);
            CREATE INDEX IF NOT EXISTS idx_relations_from ON relations(from_entity);
            CREATE INDEX IF NOT EXISTS idx_relations_to ON relations(to_entity);
            CREATE INDEX IF NOT EXISTS idx_sync_changes_peer ON sync_changes(peer_id, doc_type);
            CREATE INDEX IF NOT EXISTS idx_embedding_metadata_source ON embedding_metadata(source_type, source_id);
        "#).execute(pool).await?;
    Ok(())
}
