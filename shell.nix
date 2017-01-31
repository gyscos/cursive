{ pkgs ? import <nixpkgs> {} }:

pkgs.stdenv.mkDerivation {
  name = "cursive-env";
  buildInputs = with pkgs; [
    ncurses
  ];

  RUST_BACKTRACE = 1;
}
