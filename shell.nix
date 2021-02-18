let
  moz_overlay = import (builtins.fetchTarball
    "https://github.com/mozilla/nixpkgs-mozilla/archive/master.tar.gz");
  nixpkgs = import <nixpkgs> { overlays = [ moz_overlay ]; };
  stdenv = nixpkgs.stdenv;
  ruststable = (nixpkgs.latest.rustChannels.stable.rust.override {
    extensions = [ "rust-src" "rls-preview" "rust-analysis" "rustfmt-preview" ];
  });
in with nixpkgs; {
  rust_dev = stdenv.mkDerivation {
    name = "rust_dev";
    buildInputs = [ rustup ruststable cargo ];
  };
}
