# Fact Naming Templates

Copy and adapt for consistent fact organization.

## Project Configuration
```
project:config:{setting_name}
project:config:database_url
project:config:redis_port
project:config:api_timeout
project:config:feature_flags
```

## Team Decisions
```
team:decision:{topic}
team:decision:auth_strategy
team:decision:vector_engine
team:decision:deployment_target
team:decision:code_review_policy
```

## Technical Architecture
```
arch:component:{name}
arch:component:auth_service
arch:component:message_queue
arch:data_flow:{description}
arch:data_flow:user_to_db
arch:api_contract:{endpoint}
```

## Debugging & Incidents
```
debug:issue:{id_or_description}
debug:issue:memory_leak_2024_11
debug:fix:{description}
debug:fix:connection_pool_exhaustion
debug:root_cause:{description}
debug:root_cause:unclosed_cursor
```

## Personal Preferences
```
personal:preference:{category}
personal:preference:editor
personal:preference:terminal_theme
personal:workflow:{description}
personal:workflow:git_commit_style
```

## Learnings & Patterns
```
learning:pattern:{name}
learning:pattern:retry_with_backoff
learning:pattern:batch_embeddings
learning:anti_pattern:{name}
learning:anti_pattern:n_plus_one_queries
```

## Meeting Notes
```
meeting:{date}:{topic}
meeting:2024-11-10:auth_refactor
meeting:2024-11-10:sprint_planning
```

## External References
```
ref:doc:{source}
ref:doc:rfc_7519_jwt
ref:article:{title}
ref:article:hnsw_explained
ref:video:{title}
ref:video:crdt_deep_dive
```

---

## Usage Tips

1. **Always prefix** with category: `project:`, `team:`, `arch:`, `debug:`, `personal:`, `learning:`, `meeting:`, `ref:`
2. **Use lowercase** with underscores for readability
3. **Be specific** — `project:config:db_url` not `project:config`
4. **Version implicitly** — `write_fact` creates new version automatically
5. **Embed for search** — After writing, call `embed_text` + `vector_upsert_fact`