#!/usr/bin/env bash

set -e

function error() {
    >&2 echo "$@"
    exit 1
}

[[ -n "$ALOXIDE_RUBY_VERSION" ]] || error "Specify Ruby version via 'ALOXIDE_RUBY_VERSION'"

if [[ -n "$ALOXIDE_STATIC_RUBY" ]]; then
    CONFIGURE_OPTS="--disable-shared"
    echo "Setting up Ruby $ALOXIDE_RUBY_VERSION for static linking..."
else
    CONFIGURE_OPTS="--enable-shared"
    echo "Setting up Ruby $ALOXIDE_RUBY_VERSION for shared linking..."
fi

if [[ -n "$ALOXIDE_USE_RVM" ]]; then
    echo "Installing Ruby $ALOXIDE_RUBY_VERSION via 'rvm'..."
    rvm install "$ALOXIDE_RUBY_VERSION" --no-docs "$CONFIGURE_OPTS"
elif [[ -n "$ALOXIDE_USE_RBENV" ]]; then
    echo "Installing Ruby $ALOXIDE_RUBY_VERSION via 'rbenv'..."
    rbenv install -s "$ALOXIDE_RUBY_VERSION"
else
    error "Neither 'ALOXIDE_USE_RVM' nor 'ALOXIDE_USE_RBENV' set in environment"
fi
