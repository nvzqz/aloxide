#!/usr/bin/env bash

function error() {
    >&2 echo "$@"
    exit 1
}

function check() {
    hash $1 || error "'$1' is not installed"
}

[[ -n "$ALOXIDE_RUBY_VERSION" ]] || error "Specify Ruby version via 'ALOXIDE_RUBY_VERSION'"

if [[ -n "$ALOXIDE_STATIC_RUBY" ]]; then
    CONFIGURE_OPTS="--disable-shared"
    echo "Setting up Ruby for static linking"
else
    CONFIGURE_OPTS="--enable-shared"
    echo "Setting up Ruby for shared linking"
fi

if [[ -n "$ALOXIDE_USE_RVM" ]]; then
    check rvm
    echo "Installing Ruby via 'rvm'"
    rvm install "$CONFIGURE_OPTS" "$ALOXIDE_RUBY_VERSION"
elif [[ -n "$ALOXIDE_USE_RBENV" ]]; then
    check rbenv
    echo "Installing Ruby via 'rbenv'"
    rbenv install -s "$ALOXIDE_RUBY_VERSION"
else
    error "Neither 'ALOXIDE_USE_RVM' nor 'ALOXIDE_USE_RBENV' set in environment"
fi
