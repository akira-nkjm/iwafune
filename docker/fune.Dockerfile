# The fune CLI as a small image for other repositories' CI (e.g. `fune check` and `fune fmt --check` in the
# repository of a shared parts catalog). Built by .github/workflows/image.yml, which copies in a release binary built on
# ubuntu-24.04, so the base image matches the binary's glibc.
#
# No ENTRYPOINT: GitHub Actions container jobs start the image with their own command and run steps in a shell.
FROM ubuntu:24.04

LABEL org.opencontainers.image.source="https://github.com/akira-nkjm/iwafune"
LABEL org.opencontainers.image.description="fune, the iwafune CLI (satellite concept design data)"
LABEL org.opencontainers.image.licenses="MIT OR Apache-2.0"

# git: `fune catalog fetch` gets shared catalogs with the git CLI (and actions/checkout uses it in container jobs).
RUN apt-get update \
    && apt-get install -y --no-install-recommends git ca-certificates openssh-client \
    && rm -rf /var/lib/apt/lists/*

COPY fune /usr/local/bin/fune
COPY LICENSE-MIT LICENSE-APACHE THIRD-PARTY-NOTICES.md /usr/share/doc/fune/

CMD ["fune", "--help"]
