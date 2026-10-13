set export := true

alias f := fmt
alias l := lint
alias c := clean
alias b := build
alias br := build-release
alias t := test
alias r := run
alias rr := run-release
alias i := install
alias ri := reinstall

export TARGET_OS := os()

[doc('List available recipes')]
default:
    @just --list --unsorted

[doc('Format code with rustfmt')]
[group('lint')]
fmt:
    @cargo fmt --all -- --check

[doc('Run Clippy linter')]
[group('lint')]
lint:
    @cargo clippy --workspace -- -D warnings

[doc('Check that the JSON schemas are up to date')]
[group('lint')]
check-schema:
    @cargo xtask schema --check

[doc('Check that the bundled theme sources are up to date')]
[group('lint')]
check-themes:
    @cargo xtask themes --check

[doc('Generate the JSON schemas for the config and theme files')]
[group('build')]
generate-schema:
    @cargo xtask schema --generate

[arg('dir', help='directory with the theme files to bundle')]
[doc('Add or replace bundled themes with documented copies of the theme files in dir')]
[group('build')]
import-themes dir:
    @cargo xtask themes --import "$dir"

[doc('Regenerate the documented bundled theme sources')]
[group('build')]
generate-themes:
    @cargo xtask themes --generate

[doc('Clean the build directory')]
[group('build')]
clean:
    @cargo xtask clean --profile=both

[doc('Build the pidcatrs binary')]
[group('build')]
build: fmt lint generate-themes generate-schema
    @cargo xtask build --profile=dev

[doc('Build the pidcatrs release binary')]
[group('build')]
build-release: fmt lint generate-themes generate-schema
    @cargo xtask build --profile=release

[doc('Build the installer using Inno Setup Compiler')]
[group('build')]
[windows]
build-installer:
    @cargo xtask build-installer

[arg('profile', help='the pidcatrs binary build profile')]
[doc('Perform a full rebuild and create the installer')]
[group('build')]
[windows]
build-all profile:
    @cargo xtask build-all --profile=$profile

[doc('Run the pidcatrs binary')]
[group('build')]
run args:
    @cargo xtask run -- $args

[doc('Run the pidcatrs release binary')]
[group('build')]
run-release args:
    @cargo xtask run --profile=release -- $args

[doc('Generate and open cargo docs for the project')]
[group('doc')]
doc:
    @cargo doc --document-private-items --open

[doc('Run all tests')]
[group('test')]
test:
    @cargo test --workspace

[doc('Run all tests using cargo nextest')]
[group('test')]
nextest:
    @cargo nextest run --no-fail-fast --no-output-indent --workspace \
                       --final-status-level=all --no-tests=warn \
                       --status-level=all

[doc('Install the application by running the generated installer')]
[group('install')]
[script]
install: build-release
    if [ "$TARGET_OS" != "windows" ]; then
        cargo xtask install
    else
        cargo xtask install --silent
    fi

    just post_install

[doc('Perform a full rebuild, create the installer, and install the application')]
[group('install')]
[script]
reinstall: fmt lint
    if [ "$TARGET_OS" != "windows" ]; then
        cargo xtask reinstall
    else
        cargo xtask reinstall --silent
    fi

    just post_install

[arg('tag', help='git tag to create (vX.Y.Z); must match Cargo.toml version')]
[doc('updates changelog and creats a new git tag')]
[group('release')]
[script]
tag tag:
    set -eu

    tag="{{tag}}"
    case "$tag" in
        v*.*.*) ;;
        *)
            echo "error: tag must be vX.Y.Z (got '$tag')" >&2
            exit 1
            ;;
    esac

    release_version="${tag#v}"
    cargo_version="$(grep -m1 '^version = ' Cargo.toml | sed -E 's/^version = "(.*)"$/\1/')"

    if [ "$release_version" != "$cargo_version" ]; then
        echo "error: tag and package version do not match" >&2
        echo "  tag:        $tag ($release_version)" >&2
        echo "  Cargo.toml: $cargo_version" >&2
        printf 'update Cargo.toml to %s and run just install? [y/N] ' "$release_version" >&2
        read -r reply
        case "$reply" in
            y|Y|yes|Yes|YES)
                sed -i "0,/^version = /s/^version = \"[^\"]*\"/version = \"${release_version}\"/" Cargo.toml
                just nextest
                ;;
            *)
                echo "aborted." >&2
                exit 1
                ;;
        esac
    else
        just nextest
    fi

    new_changelog="$(git-cliff --offline --body="$(cat cliff_body.tera)" "$(git describe --tags --abbrev=0)..HEAD" --tag="$tag")"
    echo "$new_changelog" | cat - CHANGELOG.md > temp.md
    echo "$new_changelog"
    mv temp.md CHANGELOG.md
    git add CHANGELOG.md Cargo.lock Cargo.toml build/setup/setup.iss
    git commit -sm "chore(release): $tag"
    git tag "$tag"

[arg('tag', help='the tag to show changelog for')]
[doc('shows changelog for tag')]
[group('changelog')]
tag_changelog tag:
    @git-cliff --offline --body="$(cat cliff_body.tera)" \
               "$(git describe --tags --abbrev=0 $tag^ 2>/dev/null || git rev-list --max-parents=0 HEAD)..$tag"

[doc('shows changelog for all tagged commits')]
[group('changelog')]
tags_changelog:
    @git-cliff --offline --body="$(cat cliff_body.tera)" --tag "$(git describe --tags --abbrev=0)"

[doc('shows changelog for untagged commits')]
[group('changelog')]
unreleased_changelog:
    @git-cliff --offline --body="$(cat cliff_body.tera)" "$(git describe --tags --abbrev=0)..HEAD"

[doc('shows changelog for all commits')]
[group('changelog')]
all_changelog:
    @git-cliff --offline --body="$(cat cliff_body.tera)"

[doc('updates CHANGELOG.md with changelog from all tagged commits')]
[group('changelog')]
update_changelog:
    @git-cliff --offline --body="$(cat cliff_body.tera)" --tag "$(git describe --tags --abbrev=0)" | tee CHANGELOG.md
    @echo
    @echo "changelog written to '$(realpath CHANGELOG.md)'!"

[private]
[script]
post_install:
    pidcatrs_exe="$(which pidcatrs)"
    pidcatrs_exe_basename="$(basename "$pidcatrs_exe")"

    echo
    if command -v ccze >/dev/null 2>&1; then
        just installed_message "$pidcatrs_exe" | ccze --raw-ansi
    else
        just installed_message "$pidcatrs_exe"
    fi

    if [ "$TARGET_OS" != "windows" ]; then
        strip "$pidcatrs_exe" 2>/dev/null || echo "could not strip $pidcatrs_exe_basename"
    fi

    if command -v ccze >/dev/null 2>&1; then
        just file_info "$pidcatrs_exe" | ccze --raw-ansi
        just ldd_info  "$pidcatrs_exe" | ccze --raw-ansi
        just du_info   "$pidcatrs_exe" | ccze --raw-ansi
    else
        just file_info "$pidcatrs_exe"
        just ldd_info  "$pidcatrs_exe"
        just du_info   "$pidcatrs_exe"
    fi

[private]
installed_message pidcatrs_exe:
    @echo "installed pidcatrs to "$pidcatrs_exe""

[private]
file_info pidcatrs_exe:
    @file "$pidcatrs_exe"

[private]
ldd_info pidcatrs_exe:
    @ldd "$pidcatrs_exe"

[private]
du_info pidcatrs_exe:
    @du -hs --time --time-style=+'%a, %d/%b/%Y - %r' "$pidcatrs_exe"
