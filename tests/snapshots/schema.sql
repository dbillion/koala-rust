-- index: idx_embedding_metadata_source (embedding_metadata)
CREATE INDEX idx_embedding_metadata_source ON embedding_metadata(source_type, source_id);

-- index: idx_facts_current (facts)
CREATE INDEX idx_facts_current ON facts(is_current);

-- index: idx_facts_expires_at (facts)
CREATE INDEX idx_facts_expires_at ON facts(expires_at);

-- index: idx_facts_name (facts)
CREATE INDEX idx_facts_name ON facts(name);

-- index: idx_observations_entity (observations)
CREATE INDEX idx_observations_entity ON observations(entity_name);

-- index: idx_relations_from (relations)
CREATE INDEX idx_relations_from ON relations(from_entity);

-- index: idx_relations_to (relations)
CREATE INDEX idx_relations_to ON relations(to_entity);

-- index: idx_sync_changes_peer (sync_changes)
CREATE INDEX idx_sync_changes_peer ON sync_changes(peer_id, doc_type);

-- table: brain_meta (brain_meta)
CREATE TABLE brain_meta (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );

-- table: conclusions (conclusions)
CREATE TABLE conclusions (
                rowid INTEGER PRIMARY KEY AUTOINCREMENT,
                summary TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );

-- table: embedding_metadata (embedding_metadata)
CREATE TABLE embedding_metadata (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                source_type TEXT NOT NULL, -- 'fact' | 'entity'
                source_id INTEGER NOT NULL,
                model_name TEXT NOT NULL,
                dimension INTEGER NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );

-- table: entities (entities)
CREATE TABLE entities (
                rowid INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT UNIQUE NOT NULL,
                type TEXT,
                project TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );

-- table: entity_embeddings (entity_embeddings)
CREATE VIRTUAL TABLE entity_embeddings USING vec0(embedding FLOAT[256]);

-- table: entity_embeddings_chunks (entity_embeddings_chunks)
CREATE TABLE "entity_embeddings_chunks"(chunk_id INTEGER PRIMARY KEY AUTOINCREMENT,size INTEGER NOT NULL,validity BLOB NOT NULL,rowids BLOB NOT NULL);

-- table: entity_embeddings_info (entity_embeddings_info)
CREATE TABLE "entity_embeddings_info" (key text primary key, value any);

-- table: entity_embeddings_rowids (entity_embeddings_rowids)
CREATE TABLE "entity_embeddings_rowids"(rowid INTEGER PRIMARY KEY AUTOINCREMENT,id,chunk_id INTEGER,chunk_offset INTEGER);

-- table: entity_embeddings_vector_chunks00 (entity_embeddings_vector_chunks00)
CREATE TABLE "entity_embeddings_vector_chunks00"(rowid PRIMARY KEY,vectors BLOB NOT NULL);

-- table: fact_embeddings (fact_embeddings)
CREATE VIRTUAL TABLE fact_embeddings USING vec0(embedding FLOAT[256]);

-- table: fact_embeddings_chunks (fact_embeddings_chunks)
CREATE TABLE "fact_embeddings_chunks"(chunk_id INTEGER PRIMARY KEY AUTOINCREMENT,size INTEGER NOT NULL,validity BLOB NOT NULL,rowids BLOB NOT NULL);

-- table: fact_embeddings_info (fact_embeddings_info)
CREATE TABLE "fact_embeddings_info" (key text primary key, value any);

-- table: fact_embeddings_rowids (fact_embeddings_rowids)
CREATE TABLE "fact_embeddings_rowids"(rowid INTEGER PRIMARY KEY AUTOINCREMENT,id,chunk_id INTEGER,chunk_offset INTEGER);

-- table: fact_embeddings_vector_chunks00 (fact_embeddings_vector_chunks00)
CREATE TABLE "fact_embeddings_vector_chunks00"(rowid PRIMARY KEY,vectors BLOB NOT NULL);

-- table: facts (facts)
CREATE TABLE facts (
                rowid INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                text TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                is_current BOOLEAN DEFAULT 1
            , expires_at DATETIME);

-- table: facts_tombstone (facts_tombstone)
CREATE TABLE facts_tombstone (
                name TEXT PRIMARY KEY,
                deleted_at DATETIME DEFAULT CURRENT_TIMESTAMP
            , actor TEXT NOT NULL DEFAULT 'local', reason TEXT NOT NULL DEFAULT 'user_requested');

-- table: model_weights (model_weights)
CREATE TABLE model_weights (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                step INTEGER NOT NULL,
                weights_json TEXT NOT NULL,
                biases_json TEXT NOT NULL,
                optimizer_json TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );

-- table: observations (observations)
CREATE TABLE observations (
                rowid INTEGER PRIMARY KEY AUTOINCREMENT,
                entity_name TEXT NOT NULL,
                content TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (entity_name) REFERENCES entities(name) ON DELETE CASCADE
            );

-- table: plugin_registry (plugin_registry)
CREATE TABLE plugin_registry (
                name TEXT PRIMARY KEY,
                version TEXT NOT NULL,
                manifest_json TEXT NOT NULL,
                plugin_type TEXT NOT NULL, -- 'wasm' | 'native'
                path TEXT NOT NULL,
                enabled BOOLEAN DEFAULT 1,
                loaded_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );

-- table: rank_signals (rank_signals)
CREATE TABLE rank_signals (
                name TEXT PRIMARY KEY,
                signal_type TEXT NOT NULL,
                value REAL NOT NULL,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );

-- table: relations (relations)
CREATE TABLE relations (
                rowid INTEGER PRIMARY KEY AUTOINCREMENT,
                from_entity TEXT NOT NULL,
                to_entity TEXT NOT NULL,
                rel_type TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (from_entity) REFERENCES entities(name) ON DELETE CASCADE,
                FOREIGN KEY (to_entity) REFERENCES entities(name) ON DELETE CASCADE
            );

-- table: sqlite_sequence (sqlite_sequence)
CREATE TABLE sqlite_sequence(name,seq);

-- table: sync_changes (sync_changes)
CREATE TABLE sync_changes (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                peer_id TEXT NOT NULL,
                doc_type TEXT NOT NULL,
                change_bytes BLOB NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );

-- table: sync_peers (sync_peers)
CREATE TABLE sync_peers (
                peer_id TEXT PRIMARY KEY,
                last_seen DATETIME DEFAULT CURRENT_TIMESTAMP,
                doc_type TEXT NOT NULL,
                heads_json TEXT NOT NULL
            );

-- table: tasks (tasks)
CREATE TABLE tasks (
                rowid INTEGER PRIMARY KEY AUTOINCREMENT,
                title TEXT NOT NULL,
                body TEXT DEFAULT '',
                status TEXT DEFAULT 'not_started',
                project TEXT DEFAULT '',
                priority TEXT DEFAULT 'medium',
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );

-- table: thoughts (thoughts)
CREATE TABLE thoughts (
                rowid INTEGER PRIMARY KEY AUTOINCREMENT,
                step INTEGER NOT NULL,
                thought TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );
