#!/usr/bin/env python3
"""Tests for ci/check-exceptions.py. Standard library only:

    python3 -I ci/test-check-exceptions.py
"""

import datetime
import importlib.util
import os
import sys
import tempfile
import unittest

sys.dont_write_bytecode = True

HERE = os.path.dirname(os.path.abspath(__file__))
spec = importlib.util.spec_from_file_location("ce", os.path.join(HERE, "check-exceptions.py"))
ce = importlib.util.module_from_spec(spec)
spec.loader.exec_module(ce)

TODAY = datetime.date(2026, 10, 25)


def entry(**over):
    fields = {
        "id": "e1",
        "kind": "rust-attribute",
        "where": "src/*.rs",
        "lints": ["dead_code"],
        "reason": "because",
        "approved": "2026-10-01",
        "approved_in": "#1",
        "expires": "2027-01-01",
    }
    fields.update(over)
    return fields


def register(entries):
    out = []
    for e in entries:
        out.append("[[exception]]")
        for k, v in e.items():
            if isinstance(v, list):
                out.append(f"{k} = [" + ", ".join(f'"{x}"' for x in v) + "]")
            else:
                out.append(f'{k} = "{v}"')
        out.append("")
    return "\n".join(out)


class Repo:
    """A throwaway repository tree."""

    def __init__(self, files, entries=()):
        self.dir = tempfile.TemporaryDirectory()
        files = dict(files)
        files[ce.REGISTER] = register(entries)
        for rel, text in files.items():
            path = os.path.join(self.dir.name, rel)
            os.makedirs(os.path.dirname(path), exist_ok=True)
            with open(path, "w", encoding="utf-8") as f:
                f.write(text)

    def check(self, today=TODAY):
        try:
            return ce.check(self.dir.name, today)
        finally:
            self.dir.cleanup()


class Tests(unittest.TestCase):
    def problems(self, files, entries=(), today=TODAY):
        problems, _notes, _total = Repo(files, entries).check(today)
        return problems

    def test_a_clean_tree_passes(self):
        self.assertEqual(self.problems({"src/a.rs": "fn main() {}\n"}), [])

    def test_a_new_allow_is_flagged(self):
        p = self.problems({"src/a.rs": "#[allow(dead_code)]\nfn f() {}\n"})
        self.assertEqual(len(p), 1)
        self.assertIn("src/a.rs", p[0])
        self.assertIn("dead_code", p[0])

    def test_a_registered_allow_passes(self):
        files = {"src/a.rs": "#[allow(dead_code)]\nfn f() {}\n"}
        self.assertEqual(self.problems(files, [entry()]), [])

    def test_a_multiline_expect_with_parentheses_in_the_reason(self):
        text = '#![expect(\n    clippy::unwrap_used,\n    reason = "fail loudly (a panic) here"\n)]\n'
        e = entry(lints=["clippy::unwrap_used"])
        self.assertEqual(self.problems({"src/t.rs": text}, [e]), [])

    def test_every_name_in_the_attribute_must_be_listed(self):
        text = "#[allow(dead_code, unused_variables)]\nfn f() {}\n"
        p = self.problems({"src/a.rs": text}, [entry()])
        self.assertTrue(any("unused_variables" in x for x in p), p)

    def test_cfg_attr_allow_is_flagged(self):
        p = self.problems({"src/a.rs": '#[cfg_attr(test, allow(dead_code))]\nfn f() {}\n'})
        self.assertEqual(len(p), 1)

    def test_comments_and_other_attributes_are_ignored(self):
        text = "// #[allow(dead_code)]\n//! #![expect(unused)]\n#[derive(Debug)]\nstruct S;\n"
        self.assertEqual(self.problems({"src/a.rs": text}), [])

    def test_an_entry_that_covers_nothing_is_stale(self):
        p = self.problems({"src/a.rs": "fn f() {}\n"}, [entry()])
        self.assertEqual(len(p), 1)
        self.assertIn("covers nothing", p[0])

    def test_an_expired_entry_fails(self):
        files = {"src/a.rs": "#[allow(dead_code)]\nfn f() {}\n"}
        p = self.problems(files, [entry(expires="2026-10-25")])
        self.assertTrue(any("expired" in x for x in p))

    def test_an_expiry_more_than_a_year_away_fails(self):
        files = {"src/a.rs": "#[allow(dead_code)]\nfn f() {}\n"}
        p = self.problems(files, [entry(expires="2028-01-01")])
        self.assertTrue(any("more than a year" in x for x in p))

    def test_missing_fields_and_future_approval_fail(self):
        files = {"src/a.rs": "#[allow(dead_code)]\nfn f() {}\n"}
        self.assertTrue(any("'reason'" in x for x in self.problems(files, [entry(reason="")])))
        self.assertTrue(any("'approved_in'" in x for x in self.problems(files, [entry(approved_in="")])))
        self.assertTrue(any("'approved'" in x for x in self.problems(files, [entry(approved="2027-01-01")])))
        self.assertTrue(any("kind" in x for x in self.problems(files, [entry(kind="nonsense")])))

    def test_duplicate_ids_fail(self):
        files = {"src/a.rs": "#[allow(dead_code)]\nfn f() {}\n"}
        p = self.problems(files, [entry(), entry()])
        self.assertTrue(any("more than once" in x for x in p))

    def test_an_entry_close_to_expiry_is_a_note_not_a_failure(self):
        files = {"src/a.rs": "#[allow(dead_code)]\nfn f() {}\n"}
        problems, notes, _ = Repo(files, [entry(expires="2026-11-10")]).check()
        self.assertEqual(problems, [])
        self.assertEqual(len(notes), 1)

    def test_deny_toml_ignores_and_skips_are_flagged_but_own_crates_and_crates_io_are_not(self):
        deny = (
            '[advisories]\nignore = ["RUSTSEC-2099-0001"]\n'
            '[licenses]\n[[licenses.exceptions]]\ncrate = "mine"\nallow = ["AGPL-3.0-or-later"]\n'
            '[[licenses.exceptions]]\ncrate = "other"\nallow = ["GPL-3.0"]\n'
            '[bans]\nskip = [{ crate = "dup" }]\n'
            '[sources]\nallow-registry = ["https://github.com/rust-lang/crates.io-index"]\n'
        )
        files = {
            "deny.toml": deny,
            "Cargo.toml": '[workspace]\nmembers = ["crates/mine"]\n',
            "crates/mine/Cargo.toml": '[package]\nname = "mine"\n',
        }
        p = self.problems(files)
        self.assertEqual(len(p), 3, p)
        joined = " ".join(p)
        for needle in ("RUSTSEC-2099-0001", "licenses.exceptions:other", "bans.skip:dup"):
            self.assertIn(needle, joined)

    def test_a_cargo_lint_level_of_allow_is_flagged(self):
        files = {"Cargo.toml": '[workspace.lints.clippy]\nall = "warn"\nunwrap_used = "allow"\n'}
        p = self.problems(files)
        self.assertEqual(len(p), 1)
        self.assertIn("clippy::unwrap_used", p[0])

    def test_clippy_toml_relaxations_are_flagged(self):
        p = self.problems({"clippy.toml": "allow-unwrap-in-tests = true\nmsrv = \"1.88\"\n"})
        self.assertEqual(len(p), 1)

    def test_workflow_suppressions_are_flagged(self):
        wf = "jobs:\n  a:\n    continue-on-error: true\n    steps:\n      - run: x # zizmor: ignore[template-injection]\n"
        p = self.problems({".github/workflows/ci.yml": wf})
        self.assertEqual(len(p), 2, p)

    def test_shellcheck_disable_is_flagged(self):
        sh = "#!/bin/sh\n# shellcheck " + "disable=SC2086\necho $x\n"
        p = self.problems({"ci/a.sh": sh})
        self.assertEqual(len(p), 1)
        self.assertIn("SC2086", p[0])

    def test_python_suppression_comments_are_flagged(self):
        p = self.problems({"ci/a.py": "import os  # no" + "qa\n"})
        self.assertEqual(len(p), 1)

    def test_a_gitleaks_allowlist_is_flagged(self):
        p = self.problems({".gitleaks.toml": "[extend]\nuseDefault = true\n[allowlist]\npaths = ['x']\n"})
        self.assertEqual(len(p), 1)

    def test_codeql_filters_and_inline_comments_are_flagged(self):
        yml = "config: |\n  paths-ignore:\n    - tests\n"
        self.assertEqual(len(self.problems({".github/workflows/codeql.yml": yml})), 1)
        self.assertEqual(len(self.problems({"src/a.rs": "// codeql[rust/x]\nfn f() {}\n"})), 1)

    def test_the_repository_itself_is_in_order(self):
        problems, _notes, total = ce.check(ce.ROOT, TODAY)
        self.assertEqual(problems, [])
        self.assertGreater(total, 0)


if __name__ == "__main__":
    unittest.main()
