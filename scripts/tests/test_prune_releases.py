#!/usr/bin/env python3
"""Tests for scripts/ci/prune_releases.py against a fake Gitea API (no network)."""
import http.server
import json
import os
import sys
import threading
import unittest

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "ci"))
import prune_releases  # noqa: E402


class FakeGitea(http.server.BaseHTTPRequestHandler):
    releases = []
    deleted = []

    def log_message(self, *a):
        pass

    def do_GET(self):
        page = int(self.path.split("page=")[1].split("&")[0]) if "page=" in self.path else 1
        body = json.dumps(self.releases if page == 1 else []).encode()
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.end_headers()
        self.wfile.write(body)

    def do_DELETE(self):
        self.deleted.append(self.path)
        self.send_response(204)
        self.end_headers()


def release(rid, tag, assets):
    return {"id": rid, "tag_name": tag, "assets": [{"id": i, "name": f"{tag}-{i}.bin", "size": 1000} for i in range(assets)]}


class PruneTests(unittest.TestCase):
    def setUp(self):
        FakeGitea.releases = [release(3, "v0.1.4", 3), release(2, "v0.1.3", 2), release(1, "v0.1.2", 1)]
        FakeGitea.deleted = []
        self.srv = http.server.HTTPServer(("127.0.0.1", 0), FakeGitea)
        threading.Thread(target=self.srv.serve_forever, daemon=True).start()
        self.url = f"http://127.0.0.1:{self.srv.server_port}"

    def tearDown(self):
        self.srv.shutdown()

    def test_only_the_latest_release_keeps_its_files(self):
        deleted, freed = prune_releases.prune(self.url, "t", "o/r", "v0.1.4", log=lambda *_: None)
        self.assertEqual((deleted, freed), (3, 3000))
        self.assertEqual(len(FakeGitea.deleted), 3)
        self.assertTrue(all("/releases/3/" not in p for p in FakeGitea.deleted), "v0.1.4 untouched")
        self.assertTrue(all(p.startswith("/api/v1/repos/o/r/releases/") for p in FakeGitea.deleted))

    def test_a_dry_run_deletes_nothing(self):
        deleted, _ = prune_releases.prune(self.url, "t", "o/r", "v0.1.4", dry_run=True, log=lambda *_: None)
        self.assertEqual(deleted, 3)
        self.assertEqual(FakeGitea.deleted, [])

    def test_an_unknown_keep_tag_refuses_instead_of_wiping_everything(self):
        with self.assertRaises(SystemExit):
            prune_releases.prune(self.url, "t", "o/r", "v9.9.9", log=lambda *_: None)
        self.assertEqual(FakeGitea.deleted, [])

    def test_a_single_release_has_nothing_to_prune(self):
        FakeGitea.releases = [release(3, "v0.1.4", 3)]
        self.assertEqual(prune_releases.prune(self.url, "t", "o/r", "v0.1.4", log=lambda *_: None), (0, 0))

    def test_releases_without_assets_are_fine(self):
        FakeGitea.releases = [release(2, "v0.1.4", 1), {"id": 1, "tag_name": "v0.1.3", "assets": None}]
        self.assertEqual(prune_releases.prune(self.url, "t", "o/r", "v0.1.4", log=lambda *_: None), (0, 0))


if __name__ == "__main__":
    unittest.main()
