fmt:
    cargo fmt --all
    just --fmt --unstable

lint:
    cargo fmt --check --all
    just --fmt --check --unstable
    cargo clippy --all --tests -- -D warnings
    cargo shear

# taiki-e/install-action can't find these
lint-more: lint
    cargo upgrades

install:
    cargo install --features cli --path .
