{
  description = "pip-editor-rs";

  inputs = {
    nixpkgs.url = "nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { self, nixpkgs, rust-overlay }: {
    devShell.x86_64-linux = let
      pkgs = import nixpkgs {
        overlays =  [ (import rust-overlay) ];
        system = "x86_64-linux";
      };
      traceIf = pkgs.lib.debug.traceIf;
      version = {
        fixed = "1.78.0";
        latest = pkgs.rust-bin.stable.latest.default.version;
      };
      toolchain = {
        fixed = pkgs.rust-bin.stable.${version.fixed}.default.override { extensions = [ "rust-src" ]; };
        latest = pkgs.rust-bin.stable.${version.latest}.default.override { extensions = [ "rust-src" ]; };
      };
    in with pkgs; mkShell {
      nativeBuildInputs = with pkgs; [
        pkg-config
        rust-analyzer-unwrapped
      ];
      buildInputs = [
        (traceIf (toolchain.latest != toolchain.fixed) "Your Rust version (${version.latest}) is newer than the last version (${version.fixed}) tested" toolchain.latest)
        dbus
        gtk4
        gst_all_1.gstreamer
        gst_all_1.gst-plugins-base
        gst_all_1.gst-plugins-rs
        gst_all_1.gst-plugins-good
        gst_all_1.gst-plugins-bad
        gst_all_1.gst-vaapi
        clapper
        openssl
      ];

      RUST_SRC_PATH = "${toolchain.latest}/lib/rustlib/src/rust/library";
    };
  };
}