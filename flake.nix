{
  description = "home-control-panel";

  nixConfig = {
    extra-trusted-substituters = [
      "https://arm.cachix.org"
      "https://home-control-panel.cachix.org"
    ];
    extra-trusted-public-keys = [
      "arm.cachix.org-1:K3XjAeWPgWkFtSS9ge5LJSLw3xgnNqyOaG7MDecmTQ8="
      "home-control-panel.cachix.org-1:oFAMn0verQX4hIEJYrxpvVd8egU8M0szyC/7wy4eBYE="
    ];
  };

  inputs = {
    nixpkgs.url = "nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
    crane.url = "github:ipetkov/crane";
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
          pkg-config
          wrapGAppsHook
        ];

        buildInputs = with pkgs; [
          clapper
          glib
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
      home-control-panel = craneLib.buildPackage (commonArgs // {
        inherit cargoArtifacts;
      });
    in {
      checks = { inherit home-control-panel; };
      packages = {
        inherit home-control-panel;
        default = home-control-panel;
      };

      devShells.default = craneLib.devShell {
        inputsFrom = [ home-control-panel ];

        packages = [ pkgs.rust-analyzer rustToolchain ];

        RUST_SRC_PATH = "${rustToolchain}/lib/rustlib/src/rust/library";
      };
    });
}
