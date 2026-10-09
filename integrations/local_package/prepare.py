#!/usr/bin/env python3
"""Fixed local source distribution. No publication, activation, or tool installation."""
import hashlib
import io
import json
from pathlib import Path, PurePosixPath
import shutil
import subprocess
import sys
import tarfile
import tempfile

SOURCE_PIN = '594644d02b0de1a74f236340f9a0d301a0d7f58e'
ENGINE_PIN = 'b0e4b445e1a20a9354d55ebb41bbedaf16ec8a74'
ENGINE_SHA256 = '6fa49f998a23803c4df4c47e9b7f55de32cb867e923364f711b6694980600d45'
GITGUARD_PIN = '20f3a82e3858c8bdffe752e6314a97bef82f063a'
LIMIT = 256 * 1024 * 1024


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def extract(raw, destination, prefix=None):
    with tarfile.open(fileobj=io.BytesIO(raw), mode='r:*') as archive:
        members = archive.getmembers()
        if len(members) > 15000 or sum(m.size for m in members) > LIMIT:
            raise ValueError('archive extraction budget')
        seen = set()
        for member in members:
            path = PurePosixPath(member.name)
            canonical = str(path)
            if (path.is_absolute() or '..' in path.parts or '\\' in member.name
                    or not path.parts or member.name.rstrip('/') != canonical):
                raise ValueError('unsafe archive path')
            if prefix and path.parts[0] != prefix:
                raise ValueError('unexpected archive prefix')
            if canonical in seen or not (member.isfile() or member.isdir()):
                raise ValueError('duplicate or non-regular archive member')
            seen.add(canonical)
        archive.extractall(destination, members=members, filter='data')


def archive_source(repository, pin):
    raw = subprocess.check_output(['/usr/bin/git', '-C', str(repository), 'archive', '--format=tar', pin])
    if len(raw) > LIMIT:
        raise ValueError('source archive budget')
    return raw


def prepare(engine_path, destination):
    engine_path, destination = Path(engine_path), Path(destination)
    if destination.exists() or destination.is_symlink():
        raise ValueError('destination exists; retained evidence is immutable')
    if engine_path.stat().st_size > LIMIT:
        raise ValueError('engine archive budget')
    engine = engine_path.read_bytes()
    if sha(engine) != ENGINE_SHA256:
        raise ValueError('engine differs from fixed reviewed source bundle; crate substitution forbidden')
    repository = Path(__file__).resolve().parents[2]
    source = archive_source(repository, SOURCE_PIN)
    gitguard = archive_source(repository.parent / 'gitguard', GITGUARD_PIN)
    destination.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='codeguard-package-', dir=destination.parent) as temporary:
        root = Path(temporary)
        (root / 'codeguard').mkdir()
        (root / 'gitguard').mkdir()
        extract(source, root / 'codeguard')
        extract(gitguard, root / 'gitguard')
        extract(engine, root, 'guardengine-0.1.0')
        (root / 'guardengine-0.1.0').rename(root / 'guardengine')
        manifests = [
            'codeguard/Cargo.toml', 'codeguard/crates/codeguard-cli/Cargo.toml',
            'codeguard/integrations/gg-candidate-host/Cargo.toml',
            'guardengine/Cargo.toml', 'gitguard/Cargo.toml',
        ]
        locks = ['codeguard/Cargo.lock', 'codeguard/integrations/gg-candidate-host/Cargo.lock',
                 'guardengine/Cargo.lock', 'gitguard/Cargo.lock']
        result = {
            'profile': 'codeguard.local-source-distribution/v1',
            'source_commit': SOURCE_PIN, 'source_archive_sha256': sha(source),
            'engine_source_commit': ENGINE_PIN, 'engine_archive_sha256': sha(engine),
            'engine_artifact_kind': 'reviewed_source_bundle_not_released_crate',
            'gitguard_source_commit': GITGUARD_PIN, 'gitguard_archive_sha256': sha(gitguard),
            'gitguard_purpose': 'existing optional candidate host path dependency',
            'manifest_transform': 'none; bundled engine directory renamed to original sibling path',
            'file_sha256': {p: sha((root / p).read_bytes()) for p in manifests + locks},
            'published': False, 'compatibility_verified': False,
            'runtime_tools_included': False,
            'native_source_and_licenses': 'complete tracked source archive; no filtering',
        }
        (root / 'provenance.json').write_text(json.dumps(result, indent=2) + '\n')
        # Exclusive copy: prior or concurrently created destinations are never overwritten.
        shutil.copytree(root, destination)
    return result


if __name__ == '__main__':
    try:
        if len(sys.argv) != 3:
            raise ValueError('expected REVIEWED_GE_SOURCE_BUNDLE NEW_OUTPUT_DIRECTORY')
        print(json.dumps(prepare(*sys.argv[1:]), sort_keys=True))
    except (OSError, ValueError, subprocess.SubprocessError, tarfile.TarError) as error:
        print(json.dumps({'error': str(error), 'published': False}), file=sys.stderr)
        sys.exit(4)
