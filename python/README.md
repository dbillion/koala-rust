# Python client and adapters

A standard-library-only MCP stdio client and thin memory adapters for koala-rust. No LangChain or
AutoGen package is imported; the adapters expose the common methods that application code can wire
into framework callbacks/tools. This avoids forcing optional framework versions on every user.

```python
from koala_memory import KoalaMCPClient, LangChainMemoryAdapter, AutoGenMemoryAdapter

with KoalaMCPClient("./koala-rust", state_dir="./.koala-demo") as koala:
    koala.remember("project/deploy", "Promote the image after the canary passes.")
    print(koala.recall("How do I deploy?"))

    langchain_memory = LangChainMemoryAdapter(koala, session_id="agent-1")
    autogen_memory = AutoGenMemoryAdapter(koala, default_conversation="review-1")
```

Run the integration tests against a real spawned server:

```sh
PYTHONPATH=python KOALA_BIN=./target/debug/koala-rust \
  KOALA_VEC0_PATH=/path/to/vec0.so python3 -m unittest discover -s python/tests -v
```

The tests create temporary state directories and never use `~/.koala`.
