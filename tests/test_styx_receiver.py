"""Authorization and secret-output contracts for the optional Styx receiver."""
import contextlib
import copy
import importlib.util
import io
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch


ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("styx_receiver", ROOT / "python/styx_receiver.py")
receiver = importlib.util.module_from_spec(spec)
spec.loader.exec_module(receiver)


def specification():
    return {"version": 1, "transport": "https-connect", "listen": "127.0.0.1", "listen_port": 8443,
            "tls": {"certificate_path": "/opt/hton/.secrets/styx/cert.pem",
                    "key_path": "/opt/hton/.secrets/styx/key.pem"},
            "users": [{"username": "d6", "password": "synthetic-test-password-a", "backends": ["node-a"]},
                      {"username": "d6-other", "password": "synthetic-test-password-b", "backends": ["node-b"]}],
            "backends": [{"id": "node-a", "protocol": "vless", "address": "127.0.0.1", "port": 10001},
                         {"id": "node-b", "protocol": "vless", "address": "127.0.0.1", "port": 10002}]}


class ReceiverCompilerTests(unittest.TestCase):
    def test_authorization_binds_identity_selector_port_and_network(self):
        config = receiver.compile_receiver(specification())
        inbound = config["inbounds"][0]
        self.assertEqual(inbound["type"], "http")
        self.assertTrue(inbound["tls"]["enabled"])
        self.assertEqual(len(inbound["users"]), 2)
        rules = config["route"]["rules"]
        self.assertEqual(rules[-1], {"action": "reject"})
        for rule, user, backend, port in zip(rules, ("d6", "d6-other"), ("node-a", "node-b"), (10001, 10002)):
            self.assertEqual(rule["auth_user"], [user])
            self.assertEqual(rule["domain"], ["vless." + backend + ".styx.invalid"])
            self.assertEqual(rule["port"], [443])
            self.assertEqual(rule["network"], "tcp")
            self.assertEqual(rule["inbound"], ["styx-in"])
            self.assertEqual(rule["override_address"], "127.0.0.1")
            self.assertEqual(rule["override_port"], port)
        self.assertNotIn("dns", config)
        self.assertNotIn("final", config["route"])

    def test_pure_compiler_preserves_input(self):
        data = specification()
        original = copy.deepcopy(data)
        with patch("builtins.open", side_effect=AssertionError("compiler filesystem access")):
            receiver.compile_receiver(data)
        self.assertEqual(data, original)

    def test_empty_authentication_or_grants_are_rejected(self):
        for key in ("users", "backends"):
            with self.subTest(key=key):
                data = specification()
                data[key] = []
                with self.assertRaises(ValueError):
                    receiver.compile_receiver(data)
        data = specification()
        data["users"][0]["backends"] = []
        with self.assertRaises(ValueError):
            receiver.compile_receiver(data)

    def test_unknown_and_duplicate_grants_are_rejected(self):
        for grants in (["missing"], ["node-a", "node-a"], ["Node-A"], ["../node-a"]):
            data = specification()
            data["users"][0]["backends"] = grants
            with self.subTest(grants=grants), self.assertRaises(ValueError):
                receiver.compile_receiver(data)

    def test_credentials_are_unambiguous(self):
        for field in ("username", "password"):
            data = specification()
            data["users"][1][field] = data["users"][0][field]
            with self.subTest(field=field), self.assertRaises(ValueError):
                receiver.compile_receiver(data)
        for password in ("", "short", "x" * 1025, "synthetic\npassword", None):
            data = specification()
            data["users"][0]["password"] = password
            with self.subTest(password_type=type(password)), self.assertRaises(ValueError):
                receiver.compile_receiver(data)

    def test_unknown_fields_and_versions_are_rejected(self):
        for record in (lambda d: d, lambda d: d["tls"], lambda d: d["users"][0], lambda d: d["backends"][0]):
            data = specification()
            record(data)["unexpected"] = "synthetic-secret"
            with self.assertRaises(ValueError) as caught:
                receiver.compile_receiver(data)
            self.assertNotIn("synthetic-secret", str(caught.exception))
        for version in (True, 2, "1", None):
            data = specification()
            data["version"] = version
            with self.assertRaises(ValueError):
                receiver.compile_receiver(data)

    def test_backend_destinations_are_explicit_ips_and_valid_ports(self):
        for address in ("example.com", "0.0.0.0", "::", "224.0.0.1", "fe80::1%eth0", 123):
            data = specification()
            data["backends"][0]["address"] = address
            with self.subTest(address=address), self.assertRaises(ValueError):
                receiver.compile_receiver(data)
        for port in (0, 65536, True, "443"):
            data = specification()
            data["backends"][0]["port"] = port
            with self.assertRaises(ValueError):
                receiver.compile_receiver(data)
        data = specification()
        data["backends"][1]["id"] = "node-a"
        with self.assertRaises(ValueError):
            receiver.compile_receiver(data)

    def test_protocol_selector_is_allowlisted(self):
        for protocol in ("awg", "VLESS", "", [], None):
            data = specification()
            data["backends"][0]["protocol"] = protocol
            with self.subTest(protocol=protocol), self.assertRaises(ValueError):
                receiver.compile_receiver(data)

    def test_listener_loops_and_relative_tls_paths_are_rejected(self):
        data = specification()
        data["backends"][0]["port"] = data["listen_port"]
        with self.assertRaisesRegex(ValueError, "receiverLoop"):
            receiver.compile_receiver(data)
        data = specification()
        data["tls"]["key_path"] = "relative.pem"
        with self.assertRaises(ValueError):
            receiver.compile_receiver(data)


class ReceiverCliTests(unittest.TestCase):
    def invoke(self, source, target):
        output, errors = io.StringIO(), io.StringIO()
        with contextlib.redirect_stdout(output), contextlib.redirect_stderr(errors):
            code = receiver.main(["--spec", str(source), "--output", str(target)])
        return code, output.getvalue(), errors.getvalue()

    def test_private_output_and_no_credential_output(self):
        with tempfile.TemporaryDirectory() as folder:
            source, target = Path(folder) / "input.json", Path(folder) / "output.json"
            source.write_text(json.dumps(specification()))
            code, output, errors = self.invoke(source, target)
            self.assertEqual((code, output, errors), (0, "", ""))
            self.assertEqual(json.loads(target.read_text()), receiver.compile_receiver(specification()))
            if os.name == "posix":
                self.assertEqual(target.stat().st_mode & 0o777, 0o600)

    def test_invalid_input_preserves_previous_output_and_redacts(self):
        with tempfile.TemporaryDirectory() as folder:
            source, target = Path(folder) / "input.json", Path(folder) / "output.json"
            target.write_text("retained")
            for content in ('{"password":"synthetic-secret",broken}', '{"version":1,"version":1}', "[" * 2000):
                source.write_text(content)
                code, output, errors = self.invoke(source, target)
                self.assertEqual(code, 2)
                self.assertEqual(output, "")
                self.assertNotIn("synthetic-secret", errors)
                self.assertEqual(target.read_text(), "retained")

    def test_atomic_failure_preserves_previous_output(self):
        with tempfile.TemporaryDirectory() as folder:
            source, target = Path(folder) / "input.json", Path(folder) / "output.json"
            source.write_text(json.dumps(specification()))
            target.write_text("retained")
            with patch.object(receiver.os, "replace", side_effect=OSError("synthetic-secret")):
                code, _, errors = self.invoke(source, target)
            self.assertEqual(code, 2)
            self.assertEqual(errors, "styx.fileOperationFailed\n")
            self.assertEqual(target.read_text(), "retained")
            self.assertEqual(sorted(p.name for p in Path(folder).iterdir()), ["input.json", "output.json"])

    def test_input_output_collision_and_size_limit(self):
        with tempfile.TemporaryDirectory() as folder:
            source, target = Path(folder) / "input.json", Path(folder) / "output.json"
            source.write_text(json.dumps(specification()))
            original = source.read_bytes()
            self.assertEqual(self.invoke(source, source)[0], 2)
            self.assertEqual(source.read_bytes(), original)
            source.write_text(" " * (1024 * 1024 + 1))
            self.assertEqual(self.invoke(source, target)[0], 2)
            self.assertFalse(target.exists())


if __name__ == "__main__":
    unittest.main()
