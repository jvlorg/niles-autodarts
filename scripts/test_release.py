"""Packaging safety regressions; fake compiler/checker are not host evidence."""
import hashlib
import json
import os
import pathlib
import shutil
import subprocess
import tempfile
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[1]


class PackageTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="autodarts-package-")
        self.addCleanup(self.temporary.cleanup)
        self.root = pathlib.Path(self.temporary.name) / "source"
        self.root.mkdir()
        for name in ("scripts/package.sh", "scripts/prepare-host.sh", "Cargo.toml", "Cargo.lock",
                     "manifest.toml", "component/Cargo.toml", "component/niles-integration.toml",
                     "component/wit/niles-integration.wit", "crates/autodarts-core/Cargo.toml"):
            target = self.root / name
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(ROOT / name, target)
        (self.root / ".gitignore").write_text(".cache/\ndist/\nbuild/\n")
        self.host = self.root / ".cache/niles"
        self.host.mkdir(parents=True)
        (self.host / "schemas").mkdir()
        shutil.copyfile(self.root / "component/wit/niles-integration.wit",
                        self.host / "schemas/niles-integration-0.5.wit")
        (self.host / "Cargo.toml").write_text("# synthetic host build input\n")
        for repo in (self.root, self.host):
            self.git(repo, "init", "--quiet")
            self.git(repo, "config", "user.email", "fixture@example.invalid")
            self.git(repo, "config", "user.name", "Fixture")
        self.git(self.host, "remote", "add", "origin", "https://github.com/MarkNygaard/niles.git")
        self.commit(self.host)
        pin = self.git(self.host, "rev-parse", "HEAD").stdout.strip()
        (self.root / "scripts/niles-host-revision").write_text(pin + "\n")
        self.commit(self.root)
        self.bin = pathlib.Path(self.temporary.name) / "bin"
        self.bin.mkdir()
        compiler = self.bin / "cargo"
        compiler.write_text('''#!/usr/bin/env python3
import os, pathlib, sys
root = pathlib.Path(os.environ["FIXTURE_ROOT"])
target = pathlib.Path(os.environ.get("CARGO_TARGET_DIR", root / "target"))
if "--bin" in sys.argv:
    cli = target / "debug/niles"
    cli.parent.mkdir(parents=True, exist_ok=True)
    cli.write_text("#!/usr/bin/env python3\\nimport os,sys\\nsys.exit(int(os.environ.get('CHECK_EXIT', '0')))\\n")
    cli.chmod(0o755)
elif os.environ.get("MISSING_WASM") != "1":
    wasm = target / "wasm32-wasip2/release/niles_autodarts_component.wasm"
    wasm.parent.mkdir(parents=True, exist_ok=True)
    wasm.write_bytes(b"synthetic compiler output, not a component")
''')
        compiler.chmod(0o755)
        rustc = self.bin / "rustc"
        rustc.write_text("#!/bin/sh\nprintf 'synthetic rustc fixture\\n'\n")
        rustc.chmod(0o755)
        self.env = {**os.environ, "PATH": f"{self.bin}:{os.environ['PATH']}",
                    "FIXTURE_ROOT": str(self.root), "CARGO_TARGET_DIR": "build"}
        self.output = self.root / "dist/v0.1.0-alpha.3"

    def git(self, repo, *args):
        return subprocess.run(["git", "-C", str(repo), *args], check=True,
                              capture_output=True, text=True)

    def commit(self, repo):
        self.git(repo, "add", ".")
        self.git(repo, "commit", "--quiet", "-m", "fixture")

    def package(self, tag="v0.1.0-alpha.3", **env):
        return subprocess.run(["bash", "scripts/package.sh", "--tag", tag,
                               "--output", str(self.output)], cwd=self.root,
                              env={**self.env, **env}, capture_output=True, text=True)

    def assert_refused(self, result, reason):
        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn(reason, result.stderr)
        self.assertFalse(self.output.exists(), "failed candidate must not be published locally")

    def test_metadata_mismatch_fails_before_build(self):
        manifest = self.root / "manifest.toml"
        manifest.write_text(manifest.read_text().replace('version = "0.1.0-alpha.3"', 'version = "0.1.0"'))
        self.commit(self.root)
        self.assert_refused(self.package(), "version mismatch")
        self.assertFalse((self.root / "build").exists())

    def test_non_candidate_tags_are_refused_before_build(self):
        for tag in ("v0.1.0-alpha.1", "0.1.0-alpha.3"):
            with self.subTest(tag=tag):
                result = self.package(tag=tag)
                self.assertEqual(result.returncode, 2, result.stderr)
                self.assertFalse(self.output.exists())
                self.assertFalse((self.root / "build").exists())

    def test_wrong_host_revision_is_not_reset(self):
        (self.host / "Cargo.toml").write_text("# another revision\n")
        self.commit(self.host)
        before = self.git(self.host, "rev-parse", "HEAD").stdout
        self.assert_refused(self.package(), "host cache revision mismatch")
        self.assertEqual(self.git(self.host, "rev-parse", "HEAD").stdout, before)

    def test_dirty_host_is_not_reset(self):
        (self.host / "Cargo.toml").write_text("# preserve developer changes\n")
        self.assert_refused(self.package(), "host cache has tracked modifications")
        self.assertEqual((self.host / "Cargo.toml").read_text(), "# preserve developer changes\n")

    def test_missing_wasm_does_not_emit_candidate(self):
        self.assert_refused(self.package(MISSING_WASM="1"), "missing component Wasm")

    def test_failed_checker_does_not_emit_candidate(self):
        result = self.package(CHECK_EXIT="7")
        self.assertEqual(result.returncode, 7, result.stderr)
        self.assertFalse(self.output.exists())
        self.assertEqual(list(self.output.parent.glob("*.tmp.*")), [])

    def test_missing_declared_logo_does_not_emit_candidate(self):
        manifest = self.root / "component/niles-integration.toml"
        manifest.write_text('logo = "missing.svg"\n' + manifest.read_text())
        self.commit(self.root)
        self.assert_refused(self.package(), "missing or external declared logo")

    def test_existing_output_is_preserved(self):
        self.output.mkdir(parents=True)
        sentinel = self.output / "keep"
        sentinel.write_text("existing candidate")
        result = self.package()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("refusing to overwrite", result.stderr)
        self.assertEqual(sentinel.read_text(), "existing candidate")

    def test_relative_shared_target_and_provenance(self):
        result = self.package()
        self.assertEqual(result.returncode, 0, result.stderr)
        provenance = json.loads((self.output / "provenance.json").read_text())
        self.assertEqual(provenance["source_revision"], self.git(self.root, "rev-parse", "HEAD").stdout.strip())
        self.assertTrue(provenance["source_clean"])
        self.assertEqual((self.output / "integration.wasm").read_bytes(),
                         (self.root / "build/wasm32-wasip2/release/niles_autodarts_component.wasm").read_bytes())
        rows = (self.output / "SHA256SUMS").read_text().splitlines()
        self.assertEqual({row.split("  ", 1)[1] for row in rows},
                         {"niles-integration.toml", "integration.wasm", "provenance.json"})
        for row in rows:
            digest, name = row.split("  ", 1)
            self.assertEqual(hashlib.sha256((self.output / name).read_bytes()).hexdigest(), digest)


if __name__ == "__main__":
    unittest.main()
