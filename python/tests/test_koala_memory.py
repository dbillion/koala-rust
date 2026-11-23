import os
import tempfile
import unittest
from pathlib import Path

from koala_memory import AutoGenMemoryAdapter, KoalaMCPClient, LangChainMemoryAdapter


ROOT = Path(__file__).resolve().parents[2]
BINARY = Path(os.environ.get("KOALA_BIN", ROOT / "target" / "debug" / "koala-rust"))
VEC0 = os.environ.get("KOALA_VEC0_PATH", "/usr/local/lib/vec0.so")


@unittest.skipUnless(BINARY.is_file(), f"koala-rust binary not found: {BINARY}")
class KoalaAdapterIntegrationTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="koala-python-")
        self.client = KoalaMCPClient(BINARY, state_dir=self.temp.name, env={"KOALA_VEC0_PATH": VEC0})

    def tearDown(self):
        self.client.close()
        self.temp.cleanup()

    def test_stdio_client_remember_recall_delete(self):
        self.client.remember("python-sdk/refund", "The cobalt fox policy refunds unused items within thirty days.")
        hits = self.client.recall("How long do customers have to return unused goods?", k=3)
        self.assertTrue(any(hit.get("name") == "python-sdk/refund" for hit in hits))
        self.assertTrue(self.client.delete("python-sdk/refund").get("deleted"))

    def test_langchain_memory_method_contract(self):
        memory = LangChainMemoryAdapter(self.client, session_id="lc-test")
        memory.save_context({"input": "Where is the blue deployment runbook?"}, {"output": "The runbook is in the ops handbook."})
        loaded = memory.load_memory_variables({"input": "Find the deployment runbook."})
        self.assertIn("ops handbook", loaded["history"])
        memory.clear()

    def test_autogen_add_query_and_clear(self):
        memory = AutoGenMemoryAdapter(self.client, default_conversation="ag-test")
        memory.add("The staging database is backed up at 02:00 UTC.", sender="assistant")
        hits = memory.query("When is the staging database backup?", conversation_id="ag-test")
        self.assertTrue(any("02:00 UTC" in hit.get("text", "") for hit in hits))
        memory.clear()


if __name__ == "__main__":
    unittest.main()
