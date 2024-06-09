{
  description = "home-dashboard-rs";

  inputs = {
    nixpkgs.url = "nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
    crane.url = "github:ipetkov/crane";
    crane.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs = { self, nixpkgs, rust-overlay, crane, flake-utils }:
    flake-utils.lib.eachSystem [ "x86_64-linux" "aarch64-linux" ] (system: let
      pkgs = import nixpkgs {
        inherit system;
        overlays = [ (import rust-overlay) ];
      };

      rustToolchain = pkgs.rust-bin.stable.latest.default.override {
        extensions = [ "rust-src" ];
      };

      craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;

      commonArgs = {
        src = pkgs.lib.cleanSourceWith {
          src = craneLib.path ./.;
          filter = path: type:
            (builtins.match ".*\.css$" path != null) || (craneLib.filterCargoSources path type);
        };

        nativeBuildInputs = with pkgs; [
          appstream-glib
          pkg-config
          stdenv.cc
        ];

        buildInputs = with pkgs; [
          clapper
          gst_all_1.gst-plugins-bad
          gst_all_1.gst-plugins-base
          gst_all_1.gst-plugins-good
          gst_all_1.gst-plugins-rs
          gst_all_1.gst-vaapi
          gst_all_1.gstreamer
          gtk4
          openssl
        ];
      };
      
      cargoArtifacts = craneLib.buildDepsOnly commonArgs;
      home-dashboard-rs = craneLib.buildPackage (commonArgs // {
        inherit cargoArtifacts;
      });
    in {
      checks = { inherit home-dashboard-rs; };
      packages = {
        inherit home-dashboard-rs;
        default = home-dashboard-rs;
      };
            
      devShells.default = craneLib.devShell {
        inputsFrom = [ home-dashboard-rs ];

        packages = [ pkgs.rust-analyzer rustToolchain ];

        RUST_SRC_PATH = "${rustToolchain}/lib/rustlib/src/rust/library";

        CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER = "${pkgs.stdenv.cc.targetPrefix}cc";
        CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_RUNNER = "qemu-aarch64";
      };
    });
}
