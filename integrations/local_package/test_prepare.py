import importlib.util
import io
from pathlib import Path
import tarfile
import tempfile
import unittest

spec = importlib.util.spec_from_file_location('prepare', Path(__file__).with_name('prepare.py'))
prepare = importlib.util.module_from_spec(spec)
spec.loader.exec_module(prepare)

class PrepareTests(unittest.TestCase):
    def test_existing_destination_preserves_original(self):
        with tempfile.TemporaryDirectory() as temp:
            destination = Path(temp) / 'retained'
            destination.mkdir()
            (destination / 'evidence').write_bytes(b'original')
            with self.assertRaises(ValueError):
                prepare.prepare(Path(temp) / 'missing-engine', destination)
            self.assertEqual((destination / 'evidence').read_bytes(), b'original')

    def test_wrong_engine_pin_never_creates_distribution(self):
        with tempfile.TemporaryDirectory() as temp:
            engine = Path(temp) / 'wrong.tar'
            engine.write_bytes(b'old crate or unknown source')
            destination = Path(temp) / 'new'
            with self.assertRaises(ValueError):
                prepare.prepare(engine, destination)
            self.assertFalse(destination.exists())

    def test_unsafe_archive_members_and_duplicate_paths_rejected(self):
        for names, symlink in [(['../escape'], False), (['/absolute'], False), (['x\\y'], False), (['a', 'a'], False), (['link'], True)]:
            raw = io.BytesIO()
            with tarfile.open(fileobj=raw, mode='w') as archive:
                for name in names:
                    member = tarfile.TarInfo(name)
                    if symlink:
                        member.type = tarfile.SYMTYPE
                        member.linkname = '/outside'
                    archive.addfile(member)
            with tempfile.TemporaryDirectory() as temp:
                with self.assertRaises(ValueError):
                    prepare.extract(raw.getvalue(), Path(temp))
                self.assertEqual(list(Path(temp).iterdir()), [])

    def test_regular_file_and_license_bytes_preserved(self):
        raw = io.BytesIO()
        with tarfile.open(fileobj=raw, mode='w') as archive:
            for name, content in [('LICENSE', b'license'), ('src/main.rs', b'fn main() {}')]:
                member = tarfile.TarInfo(name)
                member.size = len(content)
                archive.addfile(member, io.BytesIO(content))
        with tempfile.TemporaryDirectory() as temp:
            prepare.extract(raw.getvalue(), Path(temp))
            self.assertEqual((Path(temp) / 'LICENSE').read_bytes(), b'license')
            self.assertEqual((Path(temp) / 'src/main.rs').read_bytes(), b'fn main() {}')

if __name__ == '__main__':
    unittest.main()
