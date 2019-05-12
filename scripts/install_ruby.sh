#!/usr/bin/env bash

function error() {
    >&2 echo "$@"
    exit 1
}

function check() {
    hash $1 || error "'$1' is not installed"
}

[[ -n "$ALOXIDE_RUBY_VERSION" ]] || error "Specify Ruby version via 'ALOXIDE_RUBY_VERSION'"

if [[ -z "$ALOXIDE_STATIC_RUBY" ]]; then
    CONFIGURE_OPTS="--disable-shared"
else
    CONFIGURE_OPTS="--enable-shared"
fi

if [[ ! -z "$ALOXIDE_USE_RVM" ]]; then
    check rvm
    rvm install "$CONFIGURE_OPTS" "$ALOXIDE_RUBY_VERSION"
elif [[ ! -z "$ALOXIDE_USE_RBENV" ]]; then
    check rbenv
    rbenv install -s "$ALOXIDE_RUBY_VERSION"
else
    error "Neither 'ALOXIDE_USE_RVM' nor 'ALOXIDE_USE_RBENV' set in environment"
fi
