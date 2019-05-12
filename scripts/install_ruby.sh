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
    echo "Setting up Ruby $ALOXIDE_RUBY_VERSION for static linking..."
else
    CONFIGURE_OPTS="--enable-shared"
    echo "Setting up Ruby $ALOXIDE_RUBY_VERSION for shared linking..."
fi

if [[ -n "$ALOXIDE_USE_RVM" ]]; then
    check rvm
    echo "Installing Ruby $ALOXIDE_RUBY_VERSION via 'rvm'..."
    rvm use "$ALOXIDE_RUBY_VERSION" --install "$CONFIGURE_OPTS"
elif [[ -n "$ALOXIDE_USE_RBENV" ]]; then
    check rbenv
    echo "Installing Ruby $ALOXIDE_RUBY_VERSION via 'rbenv'..."

    if ! rbenv local "$ALOXIDE_RUBY_VERSION"; then
        rbenv install "$ALOXIDE_RUBY_VERSION"
        rbenv local "$ALOXIDE_RUBY_VERSION"
    fi
else
    error "Neither 'ALOXIDE_USE_RVM' nor 'ALOXIDE_USE_RBENV' set in environment"
fi
