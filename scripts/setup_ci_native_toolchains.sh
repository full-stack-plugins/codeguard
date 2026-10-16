#!/usr/bin/env bash
# Supply real native tools required by the default acceptance tests, only in CI.
# Official distribution references checked when pinning these archives:
# https://clojure.org/guides/install_clojure
# https://commandbox.ortusbooks.com/5.4.0/setup/installation
# https://dart.dev/get-dart/archive
# https://github.com/elm/compiler/releases/tag/0.19.1
# https://luau.org/getting-started/
# https://docs.soliditylang.org/en/latest/installing-solidity.html
# https://developer.hashicorp.com/terraform/tutorials/cli/verify-archive
# https://ziglang.org/download/
set -euo pipefail

if [[ ${GITHUB_ACTIONS:-} != true || ${RUNNER_OS:-} != Linux || $(uname -m) != x86_64 ]]; then
  echo 'This installer is restricted to GitHub Actions Linux x86_64 runners.' >&2
  exit 1
fi
. /etc/os-release
if [[ $ID != ubuntu || $VERSION_ID != 24.04 ]]; then
  echo 'Native acceptance package provisioning requires Ubuntu 24.04.' >&2
  exit 1
fi
: "${RUNNER_TEMP:?}" "${GITHUB_PATH:?}" "${GITHUB_ENV:?}"
prefix=$(mktemp -d "$RUNNER_TEMP/codeguard-native.XXXXXXXX")
mkdir -p "$prefix/bin" "$prefix/downloads"
export PATH="$prefix/bin:$PATH"
if [[ -n ${JAVA_HOME_17_X64:-} ]]; then
  export JAVA_HOME=$JAVA_HOME_17_X64
  printf 'JAVA_HOME=%s\n' "$JAVA_HOME" >> "$GITHUB_ENV"
fi

# Ubuntu's signed archive supplies these compilers. Record exact installed package
# versions below instead of pretending the rolling security archive is immutable.
sudo apt-get update
sudo apt-get install --yes --no-install-recommends \
  ca-certificates curl unzip xz-utils python3-venv \
  crystal elixir erlang-base groovy lua5.4 nim ocaml fp-compiler protobuf-compiler r-base-core
dpkg-query -W -f='${Package} ${Version}\n' \
  crystal elixir erlang-base groovy lua5.4 nim ocaml fp-compiler protobuf-compiler r-base-core

fetch() {
  local url=$1 sha=$2 file=$3
  curl --fail --location --retry 4 --retry-all-errors --connect-timeout 30 --max-time 600 \
    "$url" --output "$file"
  printf '%s  %s\n' "$sha" "$file" | sha256sum --check --strict
}

# Official Clojure CLI is required: the distribution's clojure.main executable
# does not support the -M option exercised by the actual acceptance tests.
fetch 'https://download.clojure.org/install/clojure-tools-1.12.0.1530.tar.gz' \
  0ff24b8a8126ba39d32de784a08767c5df259384cb76c6ee3db4d6102705ed49 "$prefix/downloads/clojure.tar.gz"
tar -xzf "$prefix/downloads/clojure.tar.gz" -C "$prefix"
mkdir -p "$prefix/lib/clojure/libexec"
install -m644 "$prefix/clojure-tools/"{deps.edn,example-deps.edn,tools.edn} "$prefix/lib/clojure/"
install -m644 "$prefix/clojure-tools/"{exec.jar,clojure-tools-1.12.0.1530.jar} "$prefix/lib/clojure/libexec/"
sed "s@PREFIX@$prefix/lib/clojure@g" "$prefix/clojure-tools/clojure" > "$prefix/bin/clojure"
chmod +x "$prefix/bin/clojure"

fetch 'https://downloads.ortussolutions.com/ortussolutions/commandbox/6.2.1/commandbox-bin-6.2.1.zip' \
  c8f4c9befda31046fd535542024b21ae121ce12fa517f6c1405f88b432ce1f1a "$prefix/downloads/box.zip"
mkdir "$prefix/commandbox"
unzip -q "$prefix/downloads/box.zip" -d "$prefix/commandbox"
install -m755 "$prefix/commandbox/box" "$prefix/bin/box"

fetch 'https://storage.googleapis.com/dart-archive/channels/stable/release/3.7.2/sdk/dartsdk-linux-x64-release.zip' \
  c216fdd70f656c50c78dc6a0ffae6e5ced7a9a7ccedea3e402fa6b5ebe24788c "$prefix/downloads/dart.zip"
unzip -q "$prefix/downloads/dart.zip" -d "$prefix"
ln -s "$prefix/dart-sdk/bin/dart" "$prefix/bin/dart"

fetch 'https://github.com/elm/compiler/releases/download/0.19.1/binary-for-linux-64-bit.gz' \
  e44af52bb27f725a973478e589d990a6428e115fe1bb14f03833134d6c0f155c "$prefix/downloads/elm.gz"
gzip -dc "$prefix/downloads/elm.gz" > "$prefix/bin/elm"
chmod +x "$prefix/bin/elm"

fetch 'https://github.com/luau-lang/luau/releases/download/0.690/luau-ubuntu.zip' \
  a4504ae9569e5030b387a7146a023b3670d505db9a09ebc645532fe65e8479c8 "$prefix/downloads/luau.zip"
mkdir "$prefix/luau"
unzip -q "$prefix/downloads/luau.zip" -d "$prefix/luau"
install -m755 "$prefix/luau/luau-analyze" "$prefix/bin/luau-analyze"

# Solidity's documented official solc-bin mirror, with its published SHA-256.
fetch 'https://raw.githubusercontent.com/ethereum/solc-bin/gh-pages/linux-amd64/solc-linux-amd64-v0.8.29+commit.ab55807c' \
  18d418a40dc04d17656b1b5c8a7b35cfbab8942b51f38d005d5b59e8aa6637e0 "$prefix/bin/solc"
chmod +x "$prefix/bin/solc"

fetch 'https://releases.hashicorp.com/terraform/1.11.4/terraform_1.11.4_linux_amd64.zip' \
  1ce994251c00281d6845f0f268637ba50c0005657eb3cf096b92f753b42ef4dc "$prefix/downloads/terraform.zip"
unzip -q "$prefix/downloads/terraform.zip" terraform -d "$prefix/bin"

fetch 'https://ziglang.org/download/0.13.0/zig-linux-x86_64-0.13.0.tar.xz' \
  d45312e61ebcc48032b77bc4cf7fd6915c11fa16e4aad116b66c9468211230ea "$prefix/downloads/zig.tar.xz"
tar -xJf "$prefix/downloads/zig.tar.xz" -C "$prefix"
ln -s "$prefix/zig-linux-x86_64-0.13.0/zig" "$prefix/bin/zig"

# The runner's selected Python may not see apt's python3-markdown. Use a genuine
# isolated environment and a hash-pinned wheel; prepend it for subsequent tests.
python3 -m venv "$prefix/python"
printf '%s\n' 'Markdown==3.7 --hash=sha256:7eb6df5690b81a1d7942992c97fad2938e956e79df20cbc6186e9c3a77b1c803' > "$prefix/downloads/python-requirements.txt"
"$prefix/python/bin/python3" -m pip install --disable-pip-version-check --only-binary=:all: \
  --require-hashes -r "$prefix/downloads/python-requirements.txt"
export PATH="$prefix/python/bin:$PATH"
printf '%s\n' "$prefix/bin" "$prefix/python/bin" >> "$GITHUB_PATH"

# Report actual tools; no missing-tool fallback can make acceptance tests green.
crystal --version
elixir --version
erl -noshell -eval 'io:format("OTP ~s~n", [erlang:system_info(otp_release)]), halt().'
groovy --version
luac -v
nim --version
ocamlc -version
fpc -iV
protoc --version
R --version
clojure -Sdescribe
clojure -Sdeps '{:mvn/repos {"central" {:url "https://repo.maven.apache.org/maven2/"}}}' -M -e '(println (clojure-version))'
box version
dart --version
elm --version
# Warm the exact official package closure once before three concurrent tests.
export ELM_HOME="$prefix/elm-home"
printf 'ELM_HOME=%s\n' "$ELM_HOME" >> "$GITHUB_ENV"
mkdir "$prefix/elm-smoke"
cat > "$prefix/elm-smoke/elm.json" <<'JSON'
{"type":"application","source-directories":["."],"elm-version":"0.19.1","dependencies":{"direct":{"elm/core":"1.0.5","elm/html":"1.0.0"},"indirect":{"elm/json":"1.1.3","elm/virtual-dom":"1.0.3"}},"test-dependencies":{"direct":{},"indirect":{}}}
JSON
cat > "$prefix/elm-smoke/Main.elm" <<'ELM'
module Main exposing (main)
import Html exposing (text)
main = text "Native toolchain readiness"
ELM
(cd "$prefix/elm-smoke" && elm make Main.elm --output=/dev/null)
# Luau does not expose a version flag; retain the release pin and binary digest.
printf 'Luau release 0.690 analyzer: '
sha256sum "$prefix/bin/luau-analyze"
solc --version
terraform version
zig version
python3 -c 'import markdown, tomllib; print("Python Markdown", markdown.__version__)'
